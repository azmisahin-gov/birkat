//! # BİRKÂT şifreli yedekleme ve geri yükleme
//!
//! 19.4 maddesindeki yedekleme gereksinimlerini uygular. Yedek:
//!
//! * Şifrelidir (AES-256-GCM, parola ile korunur).
//! * Vaka bütünlüğünü korur.
//! * Model sürümünü ve işlem metaverisini korur.
//! * Raporları ve imzalı dosyaları içerir.
//! * Hash manifesti içerir.
//! * Geri yüklenebilir.
//!
//! ## Dürüstlük sınırı
//!
//! Yalnızca yedek oluşturulması yeterli kabul edilmez. Başka bir konuma
//! **gerçek geri yükleme ve bütünlük testi** yapılmalıdır; bu kitaplık
//! [`restore_backup`] ve [`verify_restore`] ile bunu destekler.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use birkat_core::error::{CoreError, Result};
use birkat_core::time_util::Timestamp;
use birkat_crypto::{check_password_policy, decrypt_with_password, encrypt_with_password};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Yedek paket biçim sürümü.
pub const BACKUP_FORMAT_VERSION: u32 = 1;

/// Yedek manifestosundaki tek bir dosya.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Pakete göreli yol (eğik çizgi ile).
    pub path: String,
    /// Boyut (bayt).
    pub size_bytes: u64,
    /// SHA-256.
    pub sha256: String,
}

/// Yedek manifestosu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Biçim sürümü.
    pub format_version: u32,
    /// Uygulama sürümü.
    pub app_version: String,
    /// Veri modeli sürümü.
    pub schema_version: u32,
    /// Yedek oluşturma zamanı.
    pub created_at: Timestamp,
    /// Kaynak kök dizin (bilgi amaçlı).
    pub source_root: String,
    /// Vaka kimlikleri.
    pub case_ids: Vec<String>,
    /// Dosya kayıtları.
    pub entries: Vec<BackupEntry>,
    /// Toplam boyut.
    pub total_bytes: u64,
    /// Model paketi sürümü (varsa).
    pub model_version: Option<String>,
    /// İşlem metaverisi (özet).
    pub audit_summary: Option<String>,
    /// Manifestonun kendi hash'i.
    pub manifest_sha256: Option<String>,
}

impl BackupManifest {
    /// Manifesto hash'ini hesaplar.
    pub fn compute_hash(&self) -> String {
        let mut clone = self.clone();
        clone.manifest_sha256 = None;
        let json = serde_json::to_string(&clone).unwrap_or_default();
        let mut h = Sha256::new();
        h.update(json.as_bytes());
        hex::encode(h.finalize())
    }

    /// Manifestoyu doğrular.
    pub fn verify(&self) -> bool {
        match &self.manifest_sha256 {
            Some(stored) => stored.eq_ignore_ascii_case(&self.compute_hash()),
            None => false,
        }
    }

    /// Yedeklenen dosya sayısı.
    pub fn file_count(&self) -> usize {
        self.entries.len()
    }
}

/// Yedekleme seçenekleri.
#[derive(Debug, Clone)]
pub struct BackupOptions {
    /// Parola (zorunlu).
    pub password: String,
    /// Yedeklenecek dosya desenlerini hariç tut (ör. geçici dosyalar).
    pub exclude_suffixes: Vec<String>,
    /// Azami yedek boyutu (bayt); aşılırsa hata.
    pub max_bytes: Option<u64>,
}

impl Default for BackupOptions {
    fn default() -> Self {
        Self {
            password: String::new(),
            exclude_suffixes: vec![".tmp".to_string(), ".lock".to_string()],
            max_bytes: None,
        }
    }
}

/// Yedek oluşturma sonucu.
#[derive(Debug, Clone)]
pub struct BackupResult {
    /// Yedek dosyası yolu.
    pub path: PathBuf,
    /// Manifesto.
    pub manifest: BackupManifest,
    /// Yedek boyutu (bayt).
    pub size_bytes: u64,
}

/// Bir vaka klasörünü şifreli yedeğe alır.
///
/// Yedek, iç ZIP paketinin tamamının AES-256-GCM ile şifrelenmesiyle oluşur;
/// böylece dosya adları ve içerikler korunur.
pub fn create_backup(
    source_root: impl AsRef<Path>,
    output: impl AsRef<Path>,
    options: &BackupOptions,
    case_ids: Vec<String>,
    model_version: Option<String>,
    audit_summary: Option<String>,
) -> Result<BackupResult> {
    let policy = check_password_policy(&options.password);
    if !policy.ok {
        return Err(CoreError::Validation(format!(
            "Yedek parolası politikayı karşılamıyor: {}",
            policy.issues.join(" ")
        )));
    }

    let source_root = source_root.as_ref();
    if !source_root.exists() {
        return Err(CoreError::NotFound {
            kind: "yedek kaynağı",
            id: source_root.display().to_string(),
        });
    }

    // 1. Dosyaları topla.
    let mut entries: Vec<(PathBuf, String)> = Vec::new();
    for e in walkdir::WalkDir::new(source_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !e.file_type().is_file() {
            continue;
        }
        let rel = e
            .path()
            .strip_prefix(source_root)
            .unwrap_or(e.path())
            .to_string_lossy()
            .replace('\\', "/");
        if options
            .exclude_suffixes
            .iter()
            .any(|s| rel.ends_with(s.as_str()))
        {
            continue;
        }
        entries.push((e.path().to_path_buf(), rel));
    }
    entries.sort_by(|a, b| a.1.cmp(&b.1));

    // 2. İç ZIP paketini oluştur (henüz şifresiz).
    let mut inner = std::io::Cursor::new(Vec::new());
    let mut manifest_entries = Vec::new();
    let mut total = 0u64;
    {
        let mut zip = zip::ZipWriter::new(&mut inner);
        let opts: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // Manifesto dosyasını en son eklemek için önce içerikleri yaz.
        for (abs, rel) in &entries {
            let data = std::fs::read(abs)?;
            let sha = sha256_hex(&data);
            total += data.len() as u64;
            if let Some(max) = options.max_bytes {
                if total > max {
                    return Err(CoreError::Validation(format!(
                        "Yedek boyutu sınırı aşıyor: {total} > {max}"
                    )));
                }
            }
            manifest_entries.push(BackupEntry {
                path: rel.clone(),
                size_bytes: data.len() as u64,
                sha256: sha,
            });
            zip.start_file(format!("data/{rel}"), opts)
                .map_err(|e| CoreError::Data(format!("Yedek yazılamadı: {e}")))?;
            zip.write_all(&data)?;
        }

        let mut manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            app_version: birkat_core::APP_VERSION.to_string(),
            schema_version: birkat_core::MODEL_VERSION,
            created_at: Timestamp::now(),
            source_root: source_root.display().to_string(),
            case_ids,
            entries: manifest_entries,
            total_bytes: total,
            model_version,
            audit_summary,
            manifest_sha256: None,
        };
        manifest.manifest_sha256 = Some(manifest.compute_hash());
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| CoreError::Data(format!("Manifesto serileştirilemedi: {e}")))?;
        zip.start_file("manifest.json", opts)
            .map_err(|e| CoreError::Data(format!("Manifesto yazılamadı: {e}")))?;
        zip.write_all(manifest_json.as_bytes())?;
        zip.finish()
            .map_err(|e| CoreError::Data(format!("Yedek kapatılamadı: {e}")))?;

        // 3. Şifrele.
        let inner_bytes = inner.into_inner();
        let encrypted = encrypt_with_password(&options.password, &inner_bytes)?;
        std::fs::write(output.as_ref(), &encrypted)?;
        let size = encrypted.len() as u64;
        Ok(BackupResult {
            path: output.as_ref().to_path_buf(),
            manifest,
            size_bytes: size,
        })
    }
}

/// Yedeği hedef klasöre geri yükler ve bütünlüğü doğrular.
pub fn restore_backup(
    backup: impl AsRef<Path>,
    target_root: impl AsRef<Path>,
    password: &str,
) -> Result<RestoreResult> {
    let blob = std::fs::read(backup.as_ref())?;
    let inner = decrypt_with_password(password, &blob)
        .map_err(|e| CoreError::Data(format!("Yedek çözülemedi: {e}")))?;

    let target_root = target_root.as_ref();
    std::fs::create_dir_all(target_root)?;

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(inner))
        .map_err(|e| CoreError::Data(format!("Yedek paketi açılamadı: {e}")))?;

    // Manifestoyu oku.
    let manifest: BackupManifest = {
        let mut f = archive
            .by_name("manifest.json")
            .map_err(|_| CoreError::Data("Yedekte manifesto bulunamadı.".to_string()))?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s)?;
        serde_json::from_str(&s)
            .map_err(|e| CoreError::Data(format!("Manifesto çözülemedi: {e}")))?
    };

    if !manifest.verify() {
        return Err(CoreError::Data(
            "Yedek manifesti bütünlük doğrulamasından geçemedi.".to_string(),
        ));
    }

    // Dosyaları çıkar ve hash'leri doğrula.
    let mut restored = 0usize;
    let mut hash_mismatches = Vec::new();
    for entry in &manifest.entries {
        let name = format!("data/{}", entry.path);
        let mut f = archive
            .by_name(&name)
            .map_err(|_| CoreError::Data(format!("Yedekte dosya eksik: {}", entry.path)))?;
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data)?;

        let sha = sha256_hex(&data);
        if !sha.eq_ignore_ascii_case(&entry.sha256) {
            hash_mismatches.push(entry.path.clone());
            continue;
        }

        let out = safe_join(target_root, &entry.path)?;
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out, &data)?;
        restored += 1;
    }

    Ok(RestoreResult {
        manifest,
        restored_files: restored,
        hash_mismatches,
        target_root: target_root.to_path_buf(),
    })
}

/// Geri yükleme sonucu.
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// Yedek manifestosu.
    pub manifest: BackupManifest,
    /// Geri yüklenen dosya sayısı.
    pub restored_files: usize,
    /// Hash uyuşmayan dosyalar.
    pub hash_mismatches: Vec<String>,
    /// Hedef kök dizin.
    pub target_root: PathBuf,
}

impl RestoreResult {
    /// Geri yükleme başarılı mı?
    pub fn ok(&self) -> bool {
        self.hash_mismatches.is_empty() && self.restored_files == self.manifest.entries.len()
    }

    /// Türkçe özet.
    pub fn summary(&self) -> String {
        if self.ok() {
            format!(
                "Geri yükleme başarılı: {} dosya, {} bayt.",
                self.restored_files, self.manifest.total_bytes
            )
        } else {
            format!(
                "Geri yükleme eksik: {} dosya geri yüklendi, {} hash uyuşmazlığı.",
                self.restored_files,
                self.hash_mismatches.len()
            )
        }
    }
}

/// Yedeği başka bir konuma geri yükleyip bütünlüğü test eder.
///
/// 19.4 gereği yalnızca yedek oluşturmak yeterli değildir; gerçek geri
/// yükleme ve bütünlük testi yapılmalıdır. Bu işlev, test amaçlı geçici bir
/// klasöre geri yükleyip doğrular ve klasörü siler.
pub fn verify_restore(
    backup: impl AsRef<Path>,
    password: &str,
    scratch_root: impl AsRef<Path>,
) -> Result<RestoreResult> {
    let scratch = scratch_root.as_ref().join(format!(
        "birkat-restore-test-{}",
        Timestamp::now().inner().unix_timestamp()
    ));
    let result = restore_backup(&backup, &scratch, password)?;
    // Test klasörünü temizle.
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(result)
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn safe_join(base: &Path, name: &str) -> Result<PathBuf> {
    let normalized = name.replace('\\', "/");
    if normalized.contains("..") || normalized.starts_with('/') {
        return Err(CoreError::Validation(format!(
            "Güvensiz yedek girdi adı: {name}"
        )));
    }
    Ok(base.join(normalized))
}

/// Yedek dosyasının şifreli blob olduğunu doğrular (parola gerekmez).
pub fn is_birkat_backup(backup: impl AsRef<Path>) -> bool {
    match std::fs::read(backup.as_ref()) {
        Ok(b) => birkat_crypto::read_header(&b).is_ok(),
        Err(_) => false,
    }
}

/// Saklama politikası (19.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Saklama süresi (gün); 0 = süresiz.
    pub retention_days: u32,
    /// Azami yedek sayısı; 0 = sınırsız.
    pub max_backups: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            retention_days: 0,
            max_backups: 10,
        }
    }
}

impl RetentionPolicy {
    /// Verilen yedek listesinden saklanacakları seçer.
    pub fn select_kept(&self, backups: &[(PathBuf, Timestamp)]) -> Vec<PathBuf> {
        let mut sorted: Vec<&(PathBuf, Timestamp)> = backups.iter().collect();
        sorted.sort_by_key(|(_, at)| std::cmp::Reverse(*at)); // en yeni önce

        let now = Timestamp::now();
        sorted
            .iter()
            .enumerate()
            .filter(|(i, (_, at))| {
                if self.max_backups > 0 && (*i as u32) >= self.max_backups {
                    return false;
                }
                if self.retention_days > 0 {
                    let age = now.inner() - at.inner();
                    return age.whole_days() < self.retention_days as i64;
                }
                true
            })
            .map(|(_, (p, _))| p.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_case_dir(dir: &Path) -> PathBuf {
        let root = dir.join("vaka-2025-123");
        std::fs::create_dir_all(root.join("kasа")).unwrap();
        std::fs::write(root.join("kasа/kayit.wav"), b"RIFF....WAVEfmt ").unwrap();
        std::fs::write(root.join("BILIRKISI_RAPORU.udf"), b"PK\x03\x04rapor").unwrap();
        let mut f = std::fs::File::create(root.join("notlar.txt")).unwrap();
        f.write_all("Türkçe notlar: ğüşiöç".as_bytes()).unwrap();
        root
    }

    #[test]
    fn backup_and_restore_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let backup = dir.path().join("vaka.yedek");

        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            ..Default::default()
        };
        let result = create_backup(
            &src,
            &backup,
            &opts,
            vec!["case-1".to_string()],
            Some("whisper-large-v3-turbo 1.0".to_string()),
            Some("42 olay".to_string()),
        )
        .unwrap();

        assert!(backup.exists());
        assert_eq!(result.manifest.file_count(), 3);
        assert!(result.manifest.verify());
        assert!(is_birkat_backup(&backup));

        let target = dir.path().join("geri-yukleme");
        let restored = restore_backup(&backup, &target, "Guvenli-Yedek-2026").unwrap();
        assert!(restored.ok(), "{}", restored.summary());
        assert_eq!(restored.restored_files, 3);
        assert_eq!(
            std::fs::read(target.join("notlar.txt")).unwrap(),
            "Türkçe notlar: ğüşiöç".as_bytes()
        );
        assert!(target.join("kasа/kayit.wav").exists());
    }

    #[test]
    fn backup_is_encrypted() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let backup = dir.path().join("v.yedek");
        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            ..Default::default()
        };
        create_backup(&src, &backup, &opts, vec![], None, None).unwrap();
        let bytes = std::fs::read(&backup).unwrap();
        // Düz metin aranmamalı.
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("BILIRKISI_RAPORU.udf"));
        assert!(!text.contains("Türkçe notlar"));
        // Sihirli baytlar bulunmalı.
        assert_eq!(&bytes[0..4], birkat_crypto::MAGIC);
    }

    #[test]
    fn wrong_password_fails() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let backup = dir.path().join("v.yedek");
        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            ..Default::default()
        };
        create_backup(&src, &backup, &opts, vec![], None, None).unwrap();
        assert!(restore_backup(&backup, dir.path().join("x"), "yanlis").is_err());
    }

    #[test]
    fn weak_password_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let opts = BackupOptions {
            password: "kısa".to_string(),
            ..Default::default()
        };
        assert!(create_backup(&src, dir.path().join("b"), &opts, vec![], None, None).is_err());
    }

    #[test]
    fn verify_restore_tests_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let backup = dir.path().join("v.yedek");
        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            ..Default::default()
        };
        create_backup(&src, &backup, &opts, vec![], None, None).unwrap();
        let r = verify_restore(&backup, "Guvenli-Yedek-2026", dir.path()).unwrap();
        assert!(r.ok());
        // Test klasörü temizlenmiş olmalı.
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("birkat-restore-test")
            })
            .collect();
        assert!(leftovers.is_empty(), "geçici test klasörü silinmeli");
    }

    #[test]
    fn tampered_backup_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let backup = dir.path().join("v.yedek");
        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            ..Default::default()
        };
        create_backup(&src, &backup, &opts, vec![], None, None).unwrap();
        let mut bytes = std::fs::read(&backup).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(&backup, &bytes).unwrap();
        assert!(restore_backup(&backup, dir.path().join("x"), "Guvenli-Yedek-2026").is_err());
    }

    #[test]
    fn manifest_records_versions_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let backup = dir.path().join("v.yedek");
        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            ..Default::default()
        };
        let r = create_backup(
            &src,
            &backup,
            &opts,
            vec!["case-1".to_string()],
            Some("model-1.0".to_string()),
            Some("olay ozeti".to_string()),
        )
        .unwrap();
        assert_eq!(r.manifest.app_version, birkat_core::APP_VERSION);
        assert_eq!(r.manifest.schema_version, birkat_core::MODEL_VERSION);
        assert_eq!(r.manifest.case_ids, vec!["case-1"]);
        assert_eq!(r.manifest.model_version.as_deref(), Some("model-1.0"));
        assert!(r.manifest.audit_summary.is_some());
    }

    #[test]
    fn excluded_suffixes_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        std::fs::write(src.join("gecici.tmp"), b"x").unwrap();
        let backup = dir.path().join("v.yedek");
        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            ..Default::default()
        };
        let r = create_backup(&src, &backup, &opts, vec![], None, None).unwrap();
        assert!(!r.manifest.entries.iter().any(|e| e.path.ends_with(".tmp")));
    }

    #[test]
    fn max_bytes_is_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let src = make_case_dir(dir.path());
        let opts = BackupOptions {
            password: "Guvenli-Yedek-2026".to_string(),
            max_bytes: Some(5),
            ..Default::default()
        };
        assert!(create_backup(&src, dir.path().join("b"), &opts, vec![], None, None).is_err());
    }

    #[test]
    fn retention_policy_keeps_newest() {
        let p = RetentionPolicy {
            retention_days: 0,
            max_backups: 2,
        };
        let now = Timestamp::now();
        let older = Timestamp::from(now.inner() - time::Duration::days(1));
        let oldest = Timestamp::from(now.inner() - time::Duration::days(2));
        let backups = vec![
            (PathBuf::from("a"), now),
            (PathBuf::from("b"), older),
            (PathBuf::from("c"), oldest),
        ];
        let kept = p.select_kept(&backups);
        assert_eq!(kept.len(), 2);
        assert!(kept.contains(&PathBuf::from("a")));
        assert!(kept.contains(&PathBuf::from("b")));
        assert!(!kept.contains(&PathBuf::from("c")));
    }

    #[test]
    fn safe_join_blocks_traversal() {
        let base = Path::new("/tmp/hedef");
        assert!(safe_join(base, "a/b.txt").is_ok());
        assert!(safe_join(base, "../kacis.txt").is_err());
        assert!(safe_join(base, "/etc/passwd").is_err());
    }
}
