//! Vaka kasası (6. bölüm).
//!
//! Her dosyanın güvenli ve bağımsız bir çalışma alanı olur. Kasa:
//!
//! * İlk teslim alınan belgeleri.
//! * Orijinal sesleri ve kapsayıcı dosyaları.
//! * İmaj dosyalarını ve edinim manifestolarını.
//! * Hash değerlerini ve doğrulama raporlarını.
//! * Teknik envanteri, transkriptleri, sürümleri.
//! * Bilirkişi notlarını, soruları, yanıtları.
//! * Raporları, ekleri, UDF dosyalarını.
//! * İşlem geçmişini.
//!
//! saklar.
//!
//! ## Şifreleme
//!
//! Kasadaki büyük dosyalar AES-256-GCM ile şifrelenerek saklanır. Anahtar
//! parola ile türetilir ve yalnızca oturum süresince bellekte tutulur.

use birkat_core::error::{CoreError, Result};
use birkat_core::evidence::{HashAlgorithm, HashRecord, HashSide, HashStage};
use birkat_core::ids::{CaseId, FileId};
use birkat_core::time_util::Timestamp;
use birkat_crypto::{decrypt_with_password, encrypt_with_password, SecretKey};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Kasa hatası.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// Çekirdek hata.
    #[error("{0}")]
    Core(#[from] CoreError),
    /// Şifreleme hatası.
    #[error("kasa şifreleme hatası: {0}")]
    Crypto(String),
}

/// Kasadaki bir nesnenin kaydı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultObject {
    /// Dosya kimliği.
    pub file_id: FileId,
    /// Kasa içi göreli yol.
    pub relative_path: String,
    /// Özgün ad.
    pub original_name: String,
    /// Boyut.
    pub size_bytes: u64,
    /// Şifreli mi?
    pub encrypted: bool,
    /// Hash kayıtları.
    pub hashes: Vec<HashRecord>,
    /// Oluşturma zamanı.
    pub stored_at: Timestamp,
    /// Kaynak (aygıt/dosya yolu).
    pub source: String,
}

/// Kasa manifestosu.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VaultManifest {
    /// Vaka.
    pub case_id: Option<CaseId>,
    /// Nesneler.
    pub objects: Vec<VaultObject>,
    /// Son güncelleme.
    pub updated_at: Option<Timestamp>,
}

impl VaultManifest {
    /// Nesneyi yolla bulur.
    pub fn find(&self, relative_path: &str) -> Option<&VaultObject> {
        self.objects
            .iter()
            .find(|o| o.relative_path == relative_path)
    }

    /// Toplam boyut.
    pub fn total_bytes(&self) -> u64 {
        self.objects.iter().map(|o| o.size_bytes).sum()
    }

    /// SHA-256 ile nesne arar (yinelenen tespiti).
    pub fn find_by_sha256(&self, sha: &str) -> Option<&VaultObject> {
        self.objects.iter().find(|o| {
            o.hashes
                .iter()
                .any(|h| h.algorithm == HashAlgorithm::Sha256 && h.value.eq_ignore_ascii_case(sha))
        })
    }
}

/// Vaka kasası.
pub struct CaseVault {
    root: PathBuf,
    manifest: VaultManifest,
    /// Oturum anahtarı (parola ile açıldıysa).
    session_key: Option<SecretKey>,
}

impl CaseVault {
    /// Kasa kökünü oluşturur/açar (şifresiz metaveri).
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("data"))?;
        std::fs::create_dir_all(root.join("derived"))?;
        std::fs::create_dir_all(root.join("reports"))?;

        let manifest_path = root.join("manifest.json");
        let manifest = if manifest_path.exists() {
            let s = std::fs::read_to_string(&manifest_path)?;
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            VaultManifest::default()
        };

        Ok(Self {
            root,
            manifest,
            session_key: None,
        })
    }

    /// Kasayı parola ile açar (büyük dosya şifrelemesi etkin).
    ///
    /// Bu işlev yalnızca oturum anahtarını bellekte tutar; parola hiçbir
    /// yere yazılmaz.
    pub fn unlock(&mut self, password: &str) -> Result<()> {
        let policy = birkat_crypto::check_password_policy(password);
        if !policy.ok {
            return Err(CoreError::Validation(format!(
                "Kasa parolası politikayı karşılamıyor: {}",
                policy.issues.join(" ")
            )));
        }
        let key_path = self.root.join("key.enc");
        if key_path.exists() {
            let wrapped = std::fs::read_to_string(&key_path)?;
            let key = birkat_crypto::unwrap_vault_key(&wrapped, password)?;
            self.session_key = Some(key);
        } else {
            // Yeni kasa: rastgele anahtar üret ve parolayla sar.
            let key = SecretKey::generate();
            let wrapped = birkat_crypto::wrap_vault_key(&key, password)?;
            std::fs::write(&key_path, wrapped)?;
            self.session_key = Some(key);
        }
        Ok(())
    }

    /// Kasa kilitli mi?
    pub fn is_locked(&self) -> bool {
        self.session_key.is_none()
    }

    /// Kasayı kilitler (oturum anahtarını bellekten düşürür).
    pub fn lock(&mut self) {
        self.session_key = None;
    }

    /// Kasa kökü.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Manifesto.
    pub fn manifest(&self) -> &VaultManifest {
        &self.manifest
    }

    /// Dosyayı kasaya alır (kopyalar + SHA-256).
    ///
    /// Şifreleme etkinse dosya içeriği AES-256-GCM ile şifrelenerek saklanır.
    pub fn store_file(
        &mut self,
        source: impl AsRef<Path>,
        relative_path: &str,
        source_label: &str,
    ) -> Result<VaultObject> {
        let source = source.as_ref();
        let data = std::fs::read(source)?;
        let sha = sha256_hex(&data);
        let file_id = FileId::new();

        let dest_rel = if relative_path.is_empty() {
            source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file_id.to_string())
        } else {
            relative_path.to_string()
        };

        let out = self.root.join("data").join(&dest_rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let (stored, encrypted) = match &self.session_key {
            Some(key) => (encrypt_with_password_blob(key, &data)?, true),
            None => (data.clone(), false),
        };
        std::fs::write(&out, &stored)?;

        let mut hashes = vec![HashRecord::new(
            file_id.clone(),
            HashAlgorithm::Sha256,
            sha.clone(),
            HashStage::SourceRead,
            HashSide::Source,
        )];
        hashes.push(HashRecord::new(
            file_id.clone(),
            HashAlgorithm::Sha256,
            sha256_hex(&stored),
            HashStage::TargetRead,
            HashSide::Target,
        ));

        let obj = VaultObject {
            file_id,
            relative_path: dest_rel,
            original_name: source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            size_bytes: data.len() as u64,
            encrypted,
            hashes,
            stored_at: Timestamp::now(),
            source: source_label.to_string(),
        };
        self.manifest.objects.push(obj.clone());
        self.save_manifest()?;
        Ok(obj)
    }

    /// Kasaya metin içeriği yazar (rapor, transkript, UDF).
    pub fn store_bytes(
        &mut self,
        relative_path: &str,
        data: &[u8],
        source_label: &str,
    ) -> Result<VaultObject> {
        let file_id = FileId::new();
        let sha = sha256_hex(data);
        let out = self.root.join("data").join(relative_path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let (stored, encrypted) = match &self.session_key {
            Some(key) => (encrypt_with_password_blob(key, data)?, true),
            None => (data.to_vec(), false),
        };
        std::fs::write(&out, &stored)?;

        let obj = VaultObject {
            file_id: file_id.clone(),
            relative_path: relative_path.to_string(),
            original_name: Path::new(relative_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            size_bytes: data.len() as u64,
            encrypted,
            hashes: vec![HashRecord::new(
                file_id,
                HashAlgorithm::Sha256,
                sha,
                HashStage::SourceRead,
                HashSide::Source,
            )],
            stored_at: Timestamp::now(),
            source: source_label.to_string(),
        };
        self.manifest.objects.push(obj.clone());
        self.save_manifest()?;
        Ok(obj)
    }

    /// Kasadan dosyayı okur ve şifresini çözer.
    pub fn read_object(&self, relative_path: &str) -> Result<Vec<u8>> {
        let obj = self
            .manifest
            .find(relative_path)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kasa nesnesi",
                id: relative_path.to_string(),
            })?;
        let raw = std::fs::read(self.root.join("data").join(relative_path))?;
        if obj.encrypted {
            let key = self.session_key.as_ref().ok_or_else(|| {
                CoreError::Validation(
                    "Kasa kilitli; şifreli nesne okunamaz. Önce kasayı açın.".to_string(),
                )
            })?;
            decrypt_with_password_blob(key, &raw)
        } else {
            Ok(raw)
        }
    }

    /// Kasadaki dosyanın hash'ini doğrular.
    pub fn verify_object(&self, relative_path: &str) -> Result<bool> {
        let obj = self
            .manifest
            .find(relative_path)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kasa nesnesi",
                id: relative_path.to_string(),
            })?;
        let data = self.read_object(relative_path)?;
        let sha = sha256_hex(&data);
        let expected = obj
            .hashes
            .iter()
            .find(|h| h.algorithm == HashAlgorithm::Sha256 && h.side == HashSide::Source)
            .map(|h| h.value.as_str())
            .unwrap_or("");
        Ok(sha.eq_ignore_ascii_case(expected))
    }

    /// Kasayı ve tüm nesneleri doğrular.
    pub fn verify_all(&self) -> Result<Vec<(String, bool)>> {
        let mut out = Vec::new();
        for o in &self.manifest.objects {
            let ok = self.verify_object(&o.relative_path).unwrap_or(false);
            out.push((o.relative_path.clone(), ok));
        }
        Ok(out)
    }

    /// Manifestoyu diske yazar.
    pub fn save_manifest(&mut self) -> Result<()> {
        self.manifest.updated_at = Some(Timestamp::now());
        let json = serde_json::to_string_pretty(&self.manifest)
            .map_err(|e| CoreError::Data(format!("Manifesto serileştirilemedi: {e}")))?;
        std::fs::write(self.root.join("manifest.json"), json)?;
        Ok(())
    }

    /// Vaka kimliğini ayarlar.
    pub fn set_case(&mut self, case: CaseId) -> Result<()> {
        self.manifest.case_id = Some(case);
        self.save_manifest()
    }

    /// Nesne sayısı.
    pub fn object_count(&self) -> usize {
        self.manifest.objects.len()
    }
}

/// Oturum anahtarı ile veriyi şifreler (kasa blob biçimi).
fn encrypt_with_password_blob(key: &SecretKey, data: &[u8]) -> Result<Vec<u8>> {
    birkat_crypto::encrypt(key, data)
}

fn decrypt_with_password_blob(key: &SecretKey, blob: &[u8]) -> Result<Vec<u8>> {
    birkat_crypto::decrypt(key, blob)
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// Kasa açılmadan parola denemesi yapılabilmesi için yardımcı.
pub fn try_unlock_key_file(key_file: impl AsRef<Path>, password: &str) -> Result<()> {
    let wrapped = std::fs::read_to_string(key_file.as_ref())?;
    birkat_crypto::unwrap_vault_key(&wrapped, password)?;
    Ok(())
}

/// Kasadaki dosyaları doğrudan (şifresiz) çözer; yalnızca iç kullanım.
pub fn decrypt_blob(password: &str, blob: &[u8]) -> Result<Vec<u8>> {
    decrypt_with_password(password, blob)
}

/// Veriyi parola ile şifreler; yalnızca taşınabilir dışa aktarma için.
pub fn encrypt_blob(password: &str, data: &[u8]) -> Result<Vec<u8>> {
    encrypt_with_password(password, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_read_unencrypted() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("kayit.wav");
        std::fs::write(&src, b"RIFF....WAVEfmt ").unwrap();

        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        assert!(v.is_locked());
        let obj = v.store_file(&src, "ses/kayit.wav", "USB bellek").unwrap();
        assert_eq!(obj.size_bytes, 16);
        assert!(!obj.encrypted);

        let data = v.read_object("ses/kayit.wav").unwrap();
        assert_eq!(data, b"RIFF....WAVEfmt ");
        assert!(v.verify_object("ses/kayit.wav").unwrap());
    }

    #[test]
    fn encrypted_vault_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("gizli.wav");
        std::fs::write(&src, b"gizli ses icerigi").unwrap();

        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        v.unlock("Guvenli-Kasa-2026").unwrap();
        assert!(!v.is_locked());
        let obj = v.store_file(&src, "ses/gizli.wav", "USB").unwrap();
        assert!(obj.encrypted);

        // Diskte düz metin olmamalı.
        let raw = std::fs::read(v.root().join("data/ses/gizli.wav")).unwrap();
        let text = String::from_utf8_lossy(&raw);
        assert!(!text.contains("gizli ses icerigi"));

        let data = v.read_object("ses/gizli.wav").unwrap();
        assert_eq!(data, b"gizli ses icerigi");
    }

    #[test]
    fn locked_vault_cannot_read_encrypted_object() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("g.wav");
        std::fs::write(&src, b"veri").unwrap();
        let kasa = dir.path().join("kasa");

        {
            let mut v = CaseVault::open(&kasa).unwrap();
            v.unlock("Guvenli-Kasa-2026").unwrap();
            v.store_file(&src, "g.wav", "x").unwrap();
        }

        // Yeniden aç ama açma (kilitli).
        let v2 = CaseVault::open(&kasa).unwrap();
        assert!(v2.is_locked());
        assert!(v2.read_object("g.wav").is_err());
    }

    #[test]
    fn wrong_vault_password_fails() {
        let dir = tempfile::tempdir().unwrap();
        let kasa = dir.path().join("kasa");
        {
            let mut v = CaseVault::open(&kasa).unwrap();
            v.unlock("Guvenli-Kasa-2026").unwrap();
        }
        let mut v2 = CaseVault::open(&kasa).unwrap();
        assert!(v2.unlock("yanlis-parola").is_err());
    }

    #[test]
    fn store_bytes_for_reports() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        v.store_bytes(
            "reports/BILIRKISI_RAPORU.udf",
            b"PK\x03\x04udf icerik",
            "rapor motoru",
        )
        .unwrap();
        assert_eq!(v.object_count(), 1);
        let data = v.read_object("reports/BILIRKISI_RAPORU.udf").unwrap();
        assert!(data.starts_with(b"PK"));
    }

    #[test]
    fn duplicate_detection_by_hash() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        std::fs::write(&a, b"ayni icerik").unwrap();
        std::fs::write(&b, b"ayni icerik").unwrap();
        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        let oa = v.store_file(&a, "a.bin", "x").unwrap();
        let _ob = v.store_file(&b, "b.bin", "y").unwrap();
        let sha = oa.hashes[0].value.clone();
        assert!(v.manifest().find_by_sha256(&sha).is_some());
    }

    #[test]
    fn verify_all_reports_status() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("f.bin");
        std::fs::write(&src, b"veri").unwrap();
        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        v.store_file(&src, "f.bin", "x").unwrap();
        let results = v.verify_all().unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1);
    }

    #[test]
    fn tampering_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("f.bin");
        std::fs::write(&src, b"orijinal").unwrap();
        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        v.store_file(&src, "f.bin", "x").unwrap();
        // Kasayı boz.
        std::fs::write(v.root().join("data/f.bin"), b"bozulmus").unwrap();
        assert!(!v.verify_object("f.bin").unwrap());
    }

    #[test]
    fn manifest_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("f.bin");
        std::fs::write(&src, b"veri").unwrap();
        let kasa = dir.path().join("kasa");
        {
            let mut v = CaseVault::open(&kasa).unwrap();
            v.store_file(&src, "f.bin", "x").unwrap();
        }
        let v2 = CaseVault::open(&kasa).unwrap();
        assert_eq!(v2.object_count(), 1);
        assert!(v2.manifest().find("f.bin").is_some());
    }

    #[test]
    fn weak_password_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        assert!(v.unlock("kısa").is_err());
    }

    #[test]
    fn lock_clears_session_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = CaseVault::open(dir.path().join("kasa")).unwrap();
        v.unlock("Guvenli-Kasa-2026").unwrap();
        assert!(!v.is_locked());
        v.lock();
        assert!(v.is_locked());
    }
}
