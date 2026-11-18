//! Model paketleme ve doğrulama (9.3).
//!
//! Model ilk çalıştırmada internetten indirilmez. Kurulum paketinde gerekli
//! model dosyaları bulunmalıdır. Model dosyaları sürüm sabitli, hash
//! doğrulamalı, lisans bilgili, yerel ve çevrimdışı kullanılabilir olmalıdır.
//!
//! Model güncellemeleri uygulama güncellemelerinden ayrı yönetilebilir.

use birkat_core::audio::ModelVersion;
use birkat_core::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Model paketi tanımı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackage {
    /// Paket adı (ör. "whisper-large-v3-turbo").
    pub name: String,
    /// Sürüm (sabit).
    pub version: String,
    /// Model türü.
    pub kind: ModelKind,
    /// Lisans.
    pub license: String,
    /// Lisans dosyası (paket içinde).
    pub license_file: Option<String>,
    /// Model dosyaları: (paket içi ad, SHA-256, boyut).
    pub files: Vec<ModelFile>,
    /// Paket kök dizini (yerel).
    pub root: PathBuf,
    /// Dil desteği.
    pub languages: Vec<String>,
    /// Toplam boyut (bayt).
    pub total_bytes: u64,
    /// Kaynak (indirildiği yer, belgeleme için).
    pub source: Option<String>,
}

/// Model türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelKind {
    /// Whisper (kodlayıcı-kodçözücü).
    Whisper,
    /// Tokenizer.
    Tokenizer,
    /// Yapılandırma.
    Config,
    /// Diğer.
    Other,
}

/// Model dosyası.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFile {
    /// Paket içi göreli ad.
    pub name: String,
    /// SHA-256.
    pub sha256: String,
    /// Boyut (bayt).
    pub size_bytes: u64,
}

/// Model doğrulama sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVerification {
    /// Tüm dosyalar mevcut ve hash'leri doğru mu?
    pub ok: bool,
    /// Eksik dosyalar.
    pub missing: Vec<String>,
    /// Hash uyuşmayan dosyalar.
    pub hash_mismatches: Vec<String>,
    /// Lisans dosyası var mı?
    pub license_present: bool,
    /// Açıklama.
    pub message: String,
}

impl ModelPackage {
    /// Paketi oluşturur.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            kind: ModelKind::Whisper,
            license: "MIT".to_string(),
            license_file: Some("LICENSE".to_string()),
            files: Vec::new(),
            root: root.into(),
            languages: Vec::new(),
            total_bytes: 0,
            source: None,
        }
    }

    /// Paket kökündeki dosyaları tarayıp hash'leri hesaplar.
    pub fn index_files(&mut self) -> Result<()> {
        self.files.clear();
        self.total_bytes = 0;
        let mut entries: Vec<PathBuf> = Vec::new();
        for e in walkdir::WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            if e.file_type().is_file() {
                entries.push(e.path().to_path_buf());
            }
        }
        entries.sort();
        for path in entries {
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let size = std::fs::metadata(&path)?.len();
            let sha = hash_file_sha256(&path)?;
            self.files.push(ModelFile {
                name: rel,
                sha256: sha,
                size_bytes: size,
            });
            self.total_bytes += size;
        }
        Ok(())
    }

    /// Paketi doğrular.
    pub fn verify(&self) -> ModelVerification {
        let mut missing = Vec::new();
        let mut mismatches = Vec::new();
        for f in &self.files {
            let p = self.root.join(&f.name);
            if !p.exists() {
                missing.push(f.name.clone());
                continue;
            }
            match hash_file_sha256(&p) {
                Ok(h) if h.eq_ignore_ascii_case(&f.sha256) => {}
                Ok(_) => mismatches.push(f.name.clone()),
                Err(_) => missing.push(f.name.clone()),
            }
        }
        let license_present = self
            .license_file
            .as_ref()
            .map(|l| self.root.join(l).exists())
            .unwrap_or(false);

        let ok = missing.is_empty() && mismatches.is_empty();
        let message = if ok {
            format!(
                "Model paketi doğrulandı: {} ({} dosya, {} bayt).",
                self.name,
                self.files.len(),
                self.total_bytes
            )
        } else {
            format!(
                "Model paketi doğrulanamadı: {} eksik, {} hash uyuşmazlığı.",
                missing.len(),
                mismatches.len()
            )
        };
        ModelVerification {
            ok,
            missing,
            hash_mismatches: mismatches,
            license_present,
            message,
        }
    }

    /// Paketi [`ModelVersion`] kaydına dönüştürür.
    ///
    /// `file_hash`, paketteki tüm dosya hash'lerinin birleşik özetidir.
    pub fn to_model_version(&self) -> ModelVersion {
        ModelVersion {
            name: self.name.clone(),
            version: self.version.clone(),
            file_hash: self.combined_hash(),
            license: self.license.clone(),
            path: self.root.to_string_lossy().to_string(),
        }
    }

    /// Paketteki tüm dosya hash'lerinin birleşik özeti.
    pub fn combined_hash(&self) -> String {
        let mut h = Sha256::new();
        let mut files = self.files.clone();
        files.sort_by(|a, b| a.name.cmp(&b.name));
        for f in &files {
            h.update(f.name.as_bytes());
            h.update(b"|");
            h.update(f.sha256.as_bytes());
            h.update(b"|");
        }
        hex::encode(h.finalize())
    }

    /// Model dosyası yolu.
    pub fn file_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

/// Dosyanın SHA-256 değerini hesaplar.
pub fn hash_file_sha256(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = std::io::Read::read(&mut f, &mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// Kurulum paketinin model içerdiğini doğrular.
///
/// Model dosyaları kurulum paketinde bulunmalıdır; ilk çalıştırmada
/// internetten indirilmez.
pub fn assert_bundled(pkg: &ModelPackage) -> Result<()> {
    let v = pkg.verify();
    if v.ok {
        Ok(())
    } else {
        Err(CoreError::Validation(format!(
            "Model paketi kurulum paketinde eksik veya bozuk: {}",
            v.message
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_pkg(dir: &Path) -> ModelPackage {
        std::fs::create_dir_all(dir.join("models")).unwrap();
        let mut f = std::fs::File::create(dir.join("models/model.safetensors")).unwrap();
        f.write_all(b"model agirliklari (ornek)").unwrap();
        let mut t = std::fs::File::create(dir.join("models/tokenizer.json")).unwrap();
        t.write_all(b"{\"tokenizer\":true}").unwrap();
        let mut l = std::fs::File::create(dir.join("LICENSE")).unwrap();
        l.write_all(b"MIT License").unwrap();

        let mut pkg = ModelPackage::new("whisper-large-v3-turbo", "1.0", dir);
        pkg.languages = vec!["tr".into(), "en".into()];
        pkg.index_files().unwrap();
        pkg
    }

    #[test]
    fn index_and_verify_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = make_pkg(dir.path());
        assert_eq!(pkg.files.len(), 3);
        assert!(pkg.total_bytes > 0);
        let v = pkg.verify();
        assert!(v.ok, "{}", v.message);
        assert!(v.license_present);
    }

    #[test]
    fn missing_file_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = make_pkg(dir.path());
        std::fs::remove_file(dir.path().join("models/tokenizer.json")).unwrap();
        let v = pkg.verify();
        assert!(!v.ok);
        assert!(v.missing.iter().any(|m| m.contains("tokenizer")));
    }

    #[test]
    fn tampered_file_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = make_pkg(dir.path());
        std::fs::write(dir.path().join("models/model.safetensors"), b"degistirilmis").unwrap();
        let v = pkg.verify();
        assert!(!v.ok);
        assert!(v.hash_mismatches.iter().any(|m| m.contains("model")));
    }

    #[test]
    fn combined_hash_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = make_pkg(dir.path());
        let h1 = pkg.combined_hash();
        let h2 = pkg.combined_hash();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn model_version_carries_hash_and_license() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = make_pkg(dir.path());
        let mv = pkg.to_model_version();
        assert_eq!(mv.name, "whisper-large-v3-turbo");
        assert_eq!(mv.version, "1.0");
        assert_eq!(mv.license, "MIT");
        assert_eq!(mv.file_hash.len(), 64);
    }

    #[test]
    fn assert_bundled_fails_when_incomplete() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = make_pkg(dir.path());
        std::fs::remove_file(dir.path().join("models/model.safetensors")).unwrap();
        assert!(assert_bundled(&pkg).is_err());
    }

    #[test]
    fn version_is_pinned_in_package() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = make_pkg(dir.path());
        assert_eq!(pkg.version, "1.0");
    }
}
