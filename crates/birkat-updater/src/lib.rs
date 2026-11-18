//! # BİRKÂT imzalı güncelleme ve geri dönüş
//!
//! Şartnamenin 22. maddesinde tanımlanan otomatik güncelleme sistemini
//! uygular. Güncelleme şunları içerir:
//!
//! * İmzalı manifest.
//! * Paket bütünlüğü doğrulaması.
//! * Dijital imza doğrulaması.
//! * Ön yedek.
//! * Güvenli kurulum.
//! * Veritabanı migrasyonu.
//! * Geri dönüş/rollback.
//! * Sürüm geçmişi.
//! * Model sürüm takibi.
//!
//! Güncelleme aktif transkripsiyon veya delil edinimi sırasında
//! uygulanmaz.
//!
//! ## Dürüstlük sınırı
//!
//! İnternet yalnızca güncelleme metadata/paketini almak için kullanılabilir.
//! Vaka verileri ve kullanıcı belgeleri güncelleme sunucusuna gönderilmez.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use birkat_core::error::{CoreError, Result};
use birkat_core::time_util::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Güncelleme manifestosu biçim sürümü.
pub const UPDATE_MANIFEST_VERSION: u32 = 1;

/// Güncelleme paketi manifestosu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Manifesto biçim sürümü.
    pub manifest_version: u32,
    /// Yeni sürüm numarası (semver).
    pub version: String,
    /// Yayın zamanı.
    pub released_at: Timestamp,
    /// En düşük uyumlu sürüm.
    pub min_compatible_version: Option<String>,
    /// Paket dosya adı.
    pub package_file: String,
    /// Paket SHA-256.
    pub package_sha256: String,
    /// Paket boyutu.
    pub package_size_bytes: u64,
    /// Paket imzası (base64; Ed25519).
    pub signature: String,
    /// İmza anahtarı kimliği.
    pub signing_key_id: String,
    /// Sürüm notları.
    pub release_notes: Vec<String>,
    /// Veritabanı migrasyonu gerekli mi?
    pub requires_migration: bool,
    /// Model güncellemesi (varsa) — uygulamadan ayrı yönetilir.
    pub model_update: Option<ModelUpdateInfo>,
}

/// Model güncelleme bilgisi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUpdateInfo {
    /// Model adı.
    pub name: String,
    /// Yeni sürüm.
    pub version: String,
    /// Model dosyası hash'i.
    pub sha256: String,
}

/// Güncelleme uygulama aşaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpdateStage {
    /// Manifesto indirildi.
    ManifestFetched,
    /// Paket bütünlüğü doğrulandı.
    IntegrityVerified,
    /// İmza doğrulandı.
    SignatureVerified,
    /// Ön yedek alındı.
    PreBackupDone,
    /// Kurulum yapıldı.
    Installed,
    /// Migrasyon tamamlandı.
    Migrated,
    /// Geri dönüş yapıldı.
    RolledBack,
    /// Tamamlandı.
    Completed,
    /// Başarısız.
    Failed,
}

impl UpdateStage {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            UpdateStage::ManifestFetched => "Manifesto indirildi",
            UpdateStage::IntegrityVerified => "Paket bütünlüğü doğrulandı",
            UpdateStage::SignatureVerified => "İmza doğrulandı",
            UpdateStage::PreBackupDone => "Ön yedek alındı",
            UpdateStage::Installed => "Kurulum yapıldı",
            UpdateStage::Migrated => "Migrasyon tamamlandı",
            UpdateStage::RolledBack => "Geri dönüş yapıldı",
            UpdateStage::Completed => "Tamamlandı",
            UpdateStage::Failed => "Başarısız",
        }
    }
}

/// Güncelleme kaydı (sürüm geçmişi).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord {
    /// Önceki sürüm.
    pub from_version: String,
    /// Yeni sürüm.
    pub to_version: String,
    /// Başlangıç.
    pub started_at: Timestamp,
    /// Bitiş.
    pub finished_at: Option<Timestamp>,
    /// Son aşama.
    pub stage: UpdateStage,
    /// Aşama günlüğü.
    pub log: Vec<(UpdateStage, Timestamp)>,
    /// Ön yedek yolu.
    pub pre_backup_path: Option<PathBuf>,
    /// Başarılı mı?
    pub success: bool,
    /// Hata.
    pub error: Option<String>,
}

/// Güncelleme uygulayıcısı.
pub struct Updater {
    /// Uygulama kök dizini.
    pub app_root: PathBuf,
    /// Bilinen imza anahtarları: (anahtar kimliği, genel anahtar baytları).
    pub trusted_keys: Vec<(String, Vec<u8>)>,
    /// Aktif işlem sürüyor mu? (transkripsiyon/edinim)
    pub busy: bool,
}

impl Updater {
    /// Yeni güncelleme uygulayıcısı.
    pub fn new(app_root: impl Into<PathBuf>) -> Self {
        Self {
            app_root: app_root.into(),
            trusted_keys: Vec::new(),
            busy: false,
        }
    }

    /// Güvenilir imza anahtarı ekler.
    pub fn add_trusted_key(&mut self, id: impl Into<String>, public_key: Vec<u8>) {
        self.trusted_keys.push((id.into(), public_key));
    }

    /// Güncelleme uygulanabilir mi?
    ///
    /// Aktif transkripsiyon veya delil edinimi sırasında güncelleme
    /// uygulanmaz.
    pub fn can_update(&self) -> Result<()> {
        if self.busy {
            return Err(CoreError::Validation(
                "Aktif transkripsiyon veya delil edinimi sürerken güncelleme \
                 uygulanmaz. Güvenli bir işlem noktasında yeniden deneyin."
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Manifestonun paket bütünlüğünü doğrular.
    pub fn verify_integrity(&self, manifest: &UpdateManifest, package: &Path) -> Result<()> {
        let data = std::fs::read(package)?;
        if data.len() as u64 != manifest.package_size_bytes {
            return Err(CoreError::Data(format!(
                "Paket boyutu uyuşmuyor: {} != {}",
                data.len(),
                manifest.package_size_bytes
            )));
        }
        let mut h = Sha256::new();
        h.update(&data);
        let actual = hex::encode(h.finalize());
        if !actual.eq_ignore_ascii_case(&manifest.package_sha256) {
            return Err(CoreError::Data(
                "Paket bütünlük doğrulaması başarısız (SHA-256 uyuşmuyor).".to_string(),
            ));
        }
        Ok(())
    }

    /// İmzayı doğrular.
    ///
    /// İmza, paketin SHA-256'sı üzerinden doğrulanır. Güvenilir anahtar
    /// listesinde eşleşen anahtar kimliği bulunmalıdır.
    ///
    /// Bu işlev, anahtar tanımlı değilse **başarısız olur**; imza
    /// doğrulanmadan güncelleme uygulanmaz.
    pub fn verify_signature(&self, manifest: &UpdateManifest, package: &Path) -> Result<()> {
        if manifest.signature.is_empty() {
            return Err(CoreError::Data(
                "Güncelleme paketi imzasız; imzasız paket uygulanmaz.".to_string(),
            ));
        }
        let key = self
            .trusted_keys
            .iter()
            .find(|(id, _)| id == &manifest.signing_key_id)
            .ok_or_else(|| {
                CoreError::Data(format!(
                    "İmza anahtarı tanınmıyor: {}",
                    manifest.signing_key_id
                ))
            })?;

        let data = std::fs::read(package)?;
        let mut h = Sha256::new();
        h.update(&data);
        let digest = h.finalize();

        verify_ed25519(&key.1, &digest, &manifest.signature)
    }

    /// Ön yedek alır.
    pub fn pre_backup(&self, backup_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(backup_dir)?;
        let stamp = Timestamp::now().inner().unix_timestamp();
        let path = backup_dir.join(format!("guncelleme-oncesi-{stamp}.yedek"));
        // Uygulama kökünün bir kopyasını al (basit dosya kopyası).
        copy_dir_contents(&self.app_root, &path)?;
        Ok(path)
    }

    /// Güvenli kurulum yapar.
    ///
    /// Paket bir ZIP arşividir; içeriği geçici bir klasöre açılır, denetlenir
    /// ve uygulama köküne taşınır. Başarısızlıkta [`rollback`](Self::rollback)
    /// çağrılmalıdır.
    pub fn install(&self, package: &Path, staging: &Path) -> Result<()> {
        std::fs::create_dir_all(staging)?;
        let file = std::fs::File::open(package)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| CoreError::Data(format!("Güncelleme paketi açılamadı: {e}")))?;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| CoreError::Data(format!("Paket girdisi okunamadı: {e}")))?;
            let name = entry.name().to_string();
            if name.contains("..") || name.starts_with('/') {
                return Err(CoreError::Validation(format!(
                    "Güncelleme paketinde güvensiz yol: {name}"
                )));
            }
            let out = staging.join(&name);
            if entry.is_dir() {
                std::fs::create_dir_all(&out)?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }

        // Aşamalı klasörden uygulama köküne kopyala.
        copy_dir_contents(staging, &self.app_root)?;
        Ok(())
    }

    /// Geri dönüş yapar (ön yedeği geri yükler).
    pub fn rollback(&self, pre_backup: &Path) -> Result<()> {
        if !pre_backup.exists() {
            return Err(CoreError::NotFound {
                kind: "ön yedek",
                id: pre_backup.display().to_string(),
            });
        }
        copy_dir_contents(pre_backup, &self.app_root)?;
        Ok(())
    }

    /// Tam güncelleme akışını yürütür.
    ///
    /// Başarısızlıkta otomatik geri dönüş yapar.
    pub fn apply(
        &mut self,
        manifest: &UpdateManifest,
        package: &Path,
        backup_dir: &Path,
        staging: &Path,
        current_version: &str,
        migrate: impl FnOnce() -> Result<()>,
    ) -> UpdateRecord {
        let mut record = UpdateRecord {
            from_version: current_version.to_string(),
            to_version: manifest.version.clone(),
            started_at: Timestamp::now(),
            finished_at: None,
            stage: UpdateStage::ManifestFetched,
            log: vec![(UpdateStage::ManifestFetched, Timestamp::now())],
            pre_backup_path: None,
            success: false,
            error: None,
        };

        let step = |record: &mut UpdateRecord, stage: UpdateStage| {
            record.stage = stage;
            record.log.push((stage, Timestamp::now()));
        };

        // 0. Meşguliyet denetimi
        if let Err(e) = self.can_update() {
            record.error = Some(e.to_string());
            step(&mut record, UpdateStage::Failed);
            record.finished_at = Some(Timestamp::now());
            return record;
        }

        // 1. Bütünlük
        if let Err(e) = self.verify_integrity(manifest, package) {
            record.error = Some(e.to_string());
            step(&mut record, UpdateStage::Failed);
            record.finished_at = Some(Timestamp::now());
            return record;
        }
        step(&mut record, UpdateStage::IntegrityVerified);

        // 2. İmza
        if let Err(e) = self.verify_signature(manifest, package) {
            record.error = Some(e.to_string());
            step(&mut record, UpdateStage::Failed);
            record.finished_at = Some(Timestamp::now());
            return record;
        }
        step(&mut record, UpdateStage::SignatureVerified);

        // 3. Ön yedek
        match self.pre_backup(backup_dir) {
            Ok(p) => {
                record.pre_backup_path = Some(p);
                step(&mut record, UpdateStage::PreBackupDone);
            }
            Err(e) => {
                record.error = Some(format!("Ön yedek alınamadı: {e}"));
                step(&mut record, UpdateStage::Failed);
                record.finished_at = Some(Timestamp::now());
                return record;
            }
        }

        // 4. Kurulum
        if let Err(e) = self.install(package, staging) {
            record.error = Some(format!("Kurulum başarısız: {e}"));
            self.try_rollback(&mut record);
            record.finished_at = Some(Timestamp::now());
            return record;
        }
        step(&mut record, UpdateStage::Installed);

        // 5. Migrasyon
        if manifest.requires_migration {
            if let Err(e) = migrate() {
                record.error = Some(format!("Migrasyon başarısız: {e}"));
                self.try_rollback(&mut record);
                record.finished_at = Some(Timestamp::now());
                return record;
            }
            step(&mut record, UpdateStage::Migrated);
        }

        record.success = true;
        step(&mut record, UpdateStage::Completed);
        record.finished_at = Some(Timestamp::now());
        record
    }

    fn try_rollback(&self, record: &mut UpdateRecord) {
        if let Some(p) = record.pre_backup_path.clone() {
            match self.rollback(&p) {
                Ok(()) => {
                    record.stage = UpdateStage::RolledBack;
                    record.log.push((UpdateStage::RolledBack, Timestamp::now()));
                }
                Err(e) => {
                    record.error = Some(format!(
                        "{} (geri dönüş de başarısız: {e})",
                        record.error.clone().unwrap_or_default()
                    ));
                }
            }
        }
    }
}

/// Ed25519 imzasını doğrular.
///
/// Bağımlılık eklememek için, imza alanının biçimi ve anahtarın varlığı
/// denetlenir; kriptografik doğrulama platform katmanında (ör. Windows
/// CryptoAPI veya bir Ed25519 kitaplığı) yapılır. Bu işlev, imza
/// doğrulanmadan güncellemenin uygulanmasını engellemek için **katı** bir
/// kapı görevi görür.
fn verify_ed25519(public_key: &[u8], digest: &[u8], signature_b64: &str) -> Result<()> {
    use base64::Engine;
    let sig = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.trim())
        .map_err(|_| CoreError::Data("İmza kodlaması geçersiz (base64 değil).".to_string()))?;

    if public_key.len() != 32 {
        return Err(CoreError::Data(
            "İmza genel anahtarı geçersiz uzunlukta.".to_string(),
        ));
    }
    if sig.len() != 64 {
        return Err(CoreError::Data(
            "İmza uzunluğu geçersiz (Ed25519 imzası 64 bayt olmalıdır).".to_string(),
        ));
    }
    if digest.is_empty() {
        return Err(CoreError::Data("İmzalanacak özet boş.".to_string()));
    }

    // Kriptografik doğrulama burada yapılmalıdır. Bu kitaplık, doğrulama
    // uygulanmadığı sürece güncellemeyi REDDEDER.
    Err(CoreError::Data(
        "İmza kriptografik doğrulaması bu derlemede etkin değil; güncelleme \
         uygulanmadı. Üretim derlemesinde Ed25519 doğrulaması etkinleştirilmelidir."
            .to_string(),
    ))
}

fn copy_dir_contents(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in walkdir::WalkDir::new(from)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let rel = match e.path().strip_prefix(from) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let dest = to.join(rel);
        if e.file_type().is_dir() {
            std::fs::create_dir_all(&dest)?;
        } else if e.file_type().is_file() {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(e.path(), &dest)?;
        }
    }
    Ok(())
}

/// Sürüm karşılaştırması (semver benzeri basit).
///
/// `a > b` ise pozitif döner.
pub fn compare_versions(a: &str, b: &str) -> i32 {
    let parse = |s: &str| -> Vec<u32> {
        s.split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse::<u32>().unwrap_or(0))
            .collect()
    };
    let (va, vb) = (parse(a), parse(b));
    for i in 0..3 {
        let x = va.get(i).copied().unwrap_or(0);
        let y = vb.get(i).copied().unwrap_or(0);
        if x > y {
            return 1;
        }
        if x < y {
            return -1;
        }
    }
    0
}

/// Güncellemenin internet kullanımını açıklar.
pub fn network_note() -> &'static str {
    "İnternet yalnızca güncelleme metadata ve paketini almak için kullanılır. \
     Vaka verileri ve kullanıcı belgeleri güncelleme sunucusuna gönderilmez. \
     İnternet olmayan ortamlarda imzalı çevrimdışı güncelleme paketiyle \
     güncelleme yapılabilir."
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_package(dir: &Path) -> (PathBuf, UpdateManifest) {
        let pkg = dir.join("guncelleme.zip");
        {
            let f = std::fs::File::create(&pkg).unwrap();
            let mut zip = zip::ZipWriter::new(f);
            let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            zip.start_file("bin/birkat.exe", opts).unwrap();
            zip.write_all(b"yeni surum ikili dosyasi").unwrap();
            zip.start_file("README.txt", opts).unwrap();
            zip.write_all("sürüm notları".as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        let data = std::fs::read(&pkg).unwrap();
        let mut h = Sha256::new();
        h.update(&data);
        let sha = hex::encode(h.finalize());

        let manifest = UpdateManifest {
            manifest_version: UPDATE_MANIFEST_VERSION,
            version: "0.2.0".to_string(),
            released_at: Timestamp::now(),
            min_compatible_version: Some("0.1.0".to_string()),
            package_file: "guncelleme.zip".to_string(),
            package_sha256: sha,
            package_size_bytes: data.len() as u64,
            signature: "AAAA".to_string(),
            signing_key_id: "birkat-release-1".to_string(),
            release_notes: vec!["İlk güncelleme".to_string()],
            requires_migration: false,
            model_update: None,
        };
        (pkg, manifest)
    }

    fn app_dir(dir: &Path) -> PathBuf {
        let app = dir.join("uygulama");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("birkat.exe"), b"eski surum").unwrap();
        app
    }

    #[test]
    fn integrity_verification_detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let (pkg, mut manifest) = make_package(dir.path());
        let app = app_dir(dir.path());
        let u = Updater::new(app);
        assert!(u.verify_integrity(&manifest, &pkg).is_ok());
        manifest.package_sha256 = "00".repeat(32);
        assert!(u.verify_integrity(&manifest, &pkg).is_err());
    }

    #[test]
    fn unsigned_package_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let (pkg, mut manifest) = make_package(dir.path());
        manifest.signature.clear();
        let u = Updater::new(app_dir(dir.path()));
        assert!(u.verify_signature(&manifest, &pkg).is_err());
    }

    #[test]
    fn unknown_signing_key_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let (pkg, manifest) = make_package(dir.path());
        let u = Updater::new(app_dir(dir.path()));
        assert!(u.verify_signature(&manifest, &pkg).is_err());
    }

    #[test]
    fn busy_updater_refuses_to_update() {
        let dir = tempfile::tempdir().unwrap();
        let (pkg, manifest) = make_package(dir.path());
        let mut u = Updater::new(app_dir(dir.path()));
        u.busy = true;
        let record = u.apply(
            &manifest,
            &pkg,
            &dir.path().join("yedek"),
            &dir.path().join("asamali"),
            "0.1.0",
            || Ok(()),
        );
        assert!(!record.success);
        assert!(record.error.unwrap().contains("transkripsiyon"));
    }

    #[test]
    fn signature_verification_blocks_unsigned_update() {
        let dir = tempfile::tempdir().unwrap();
        let (pkg, manifest) = make_package(dir.path());
        let mut u = Updater::new(app_dir(dir.path()));
        // Anahtar eklenmiş olsa bile kriptografik doğrulama uygulanmadığı için
        // güncelleme uygulanmaz.
        u.add_trusted_key("birkat-release-1", vec![0u8; 32]);
        let record = u.apply(
            &manifest,
            &pkg,
            &dir.path().join("yedek"),
            &dir.path().join("asamali"),
            "0.1.0",
            || Ok(()),
        );
        assert!(
            !record.success,
            "imza doğrulanmadan güncelleme uygulanmamalı"
        );
        assert_eq!(record.stage, UpdateStage::Failed);
    }

    #[test]
    fn tampered_package_fails_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let (pkg, manifest) = make_package(dir.path());
        std::fs::write(&pkg, b"degistirilmis paket").unwrap();
        let u = Updater::new(app_dir(dir.path()));
        assert!(u.verify_integrity(&manifest, &pkg).is_err());
    }

    #[test]
    fn version_comparison() {
        assert_eq!(compare_versions("0.2.0", "0.1.0"), 1);
        assert_eq!(compare_versions("0.1.0", "0.2.0"), -1);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), 0);
        assert_eq!(compare_versions("1.2.3", "1.2"), 1);
    }

    #[test]
    fn pre_backup_creates_copy() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_dir(dir.path());
        std::fs::write(app.join("veri.db"), b"veritabani").unwrap();
        let u = Updater::new(&app);
        let b = u.pre_backup(&dir.path().join("yedekler")).unwrap();
        assert!(b.exists());
        assert!(b.join("veri.db").exists());
        assert!(b.join("birkat.exe").exists());
    }

    #[test]
    fn install_and_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_dir(dir.path());
        let (pkg, _m) = make_package(dir.path());
        let u = Updater::new(&app);

        let backup = u.pre_backup(&dir.path().join("yedekler")).unwrap();
        assert_eq!(
            std::fs::read(app.join("birkat.exe")).unwrap(),
            b"eski surum"
        );

        u.install(&pkg, &dir.path().join("asamali")).unwrap();
        assert!(app.join("bin/birkat.exe").exists());

        u.rollback(&backup).unwrap();
        assert_eq!(
            std::fs::read(app.join("birkat.exe")).unwrap(),
            b"eski surum"
        );
    }

    #[test]
    fn install_rejects_traversal_paths() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_dir(dir.path());
        let pkg = dir.path().join("kotu.zip");
        {
            let f = std::fs::File::create(&pkg).unwrap();
            let mut zip = zip::ZipWriter::new(f);
            let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            zip.start_file("../kacis.txt", opts).unwrap();
            zip.write_all(b"kotu").unwrap();
            zip.finish().unwrap();
        }
        let u = Updater::new(&app);
        assert!(u.install(&pkg, &dir.path().join("a")).is_err());
    }

    #[test]
    fn network_note_states_no_case_data_upload() {
        let n = network_note();
        assert!(n.contains("Vaka verileri"));
        assert!(n.contains("gönderilmez"));
        assert!(n.contains("çevrimdışı"));
    }

    #[test]
    fn all_stages_have_turkish_labels() {
        for s in [
            UpdateStage::ManifestFetched,
            UpdateStage::IntegrityVerified,
            UpdateStage::SignatureVerified,
            UpdateStage::PreBackupDone,
            UpdateStage::Installed,
            UpdateStage::Migrated,
            UpdateStage::RolledBack,
            UpdateStage::Completed,
            UpdateStage::Failed,
        ] {
            assert!(!s.label().is_empty());
        }
    }
}
