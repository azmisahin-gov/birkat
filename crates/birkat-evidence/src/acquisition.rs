//! Dosya bazlı edinim (5.4).
//!
//! Dosya ve klasörler güvenli şekilde edinilir:
//!
//! * Kaynak dizin yapısı korunur.
//! * Dosya adları korunur.
//! * Dosya boyutları kaydedilir.
//! * Zaman bilgileri kaydedilir.
//! * SHA-256 özeti alınır.
//! * Hedef kopya tekrar okunur.
//! * Kaynak ve hedef hash değerleri karşılaştırılır.
//! * Eksik dosya ve okuma hatası raporlanır.
//! * Dosya sistemi metadata'sının korunma sınırları açıklanır.
//!
//! ## Dürüstlük sınırı
//!
//! Dosya bazlı kopya, fiziksel imaj veya L01 imajı **diye adlandırılmaz**
//! (bkz. [`AcquisitionResult::label`]).

use crate::hash::{hash_file, verify_identical};
use crate::manifest::{AcquisitionManifest, ManifestEntry};
use birkat_core::error::{CoreError, Result};
use birkat_core::evidence::{AcquisitionMethod, HashAlgorithm};
use birkat_core::time_util::Timestamp;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Edinim seçenekleri.
#[derive(Debug, Clone)]
pub struct AcquisitionOptions {
    /// Kaynak dizin yapısını koru.
    pub preserve_structure: bool,
    /// Hedefte var olan dosyanın üzerine yazma (varsayılan: yazma).
    pub no_overwrite: bool,
    /// Edinim sonrası hedef kopyayı okuyup hash doğrula.
    pub verify_after_copy: bool,
    /// Zaman bilgilerini kopyala (mtime + atime).
    pub preserve_timestamps: bool,
    /// Azami dosya boyutu (bayt); aşılırsa atlanır ve raporlanır.
    pub max_file_bytes: Option<u64>,
}

impl Default for AcquisitionOptions {
    fn default() -> Self {
        Self {
            preserve_structure: true,
            no_overwrite: true,
            verify_after_copy: true,
            preserve_timestamps: true,
            max_file_bytes: None,
        }
    }
}

/// Tek bir dosyanın edinim sonucu.
#[derive(Debug, Clone)]
pub struct AcquiredFile {
    /// Kaynak yol.
    pub source: PathBuf,
    /// Hedef yol.
    pub target: PathBuf,
    /// Boyut.
    pub size_bytes: u64,
    /// Kaynak SHA-256.
    pub source_sha256: String,
    /// Hedef SHA-256 (doğrulama yapıldıysa).
    pub target_sha256: Option<String>,
    /// Doğrulandı mı?
    pub verified: bool,
    /// Hata (varsa).
    pub error: Option<String>,
}

/// Edinim sonucu.
#[derive(Debug, Clone)]
pub struct AcquisitionResult {
    /// Yöntem.
    pub method: AcquisitionMethod,
    /// Edinilen dosyalar.
    pub files: Vec<AcquiredFile>,
    /// Manifesto.
    pub manifest: AcquisitionManifest,
    /// Başlangıç.
    pub started_at: Timestamp,
    /// Bitiş.
    pub finished_at: Option<Timestamp>,
    /// Toplam okunan bayt.
    pub bytes_read: u64,
}

impl AcquisitionResult {
    /// Sonucun dürüst etiketi.
    ///
    /// Dosya bazlı edinim, fiziksel imaj veya L01 imajı olarak
    /// adlandırılmaz (kural 12).
    pub fn label(&self) -> &'static str {
        match self.method {
            AcquisitionMethod::FileBased => "Dosya bazlı edinim (imaj değildir)",
            AcquisitionMethod::ArchiveExtraction => "Arşivden çıkarma (imaj değildir)",
            AcquisitionMethod::LogicalImage => "Mantıksal imaj (L01)",
            AcquisitionMethod::PhysicalImage => "Fiziksel imaj (E01)",
            AcquisitionMethod::RawImage => "Ham fiziksel imaj",
        }
    }

    /// Tüm doğrulamalar başarılı mı?
    pub fn all_verified(&self) -> bool {
        !self.files.is_empty() && self.files.iter().all(|f| f.verified && f.error.is_none())
    }

    /// Hatalı dosyalar.
    pub fn failed(&self) -> Vec<&AcquiredFile> {
        self.files
            .iter()
            .filter(|f| f.error.is_some() || !f.verified)
            .collect()
    }
}

/// Bir yol (dosya veya klasör) için dosya bazlı edinim yürütür.
pub fn acquire_path(
    source: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
    options: AcquisitionOptions,
) -> Result<AcquisitionResult> {
    let source = source.as_ref();
    let target_dir = target_dir.as_ref();
    std::fs::create_dir_all(target_dir)?;

    let started_at = Timestamp::now();
    let mut files = Vec::new();
    let mut bytes_read = 0u64;

    let root_base = if source.is_dir() {
        source.to_path_buf()
    } else {
        source
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let walker = if source.is_dir() {
        WalkDir::new(source).follow_links(false)
    } else {
        WalkDir::new(source)
    };

    for entry in walker.into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                files.push(AcquiredFile {
                    source: e.path().map(|p| p.to_path_buf()).unwrap_or_default(),
                    target: PathBuf::new(),
                    size_bytes: 0,
                    source_sha256: String::new(),
                    target_sha256: None,
                    verified: false,
                    error: Some(format!("Dizin okunamadı: {e}")),
                });
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let src = entry.path();
        let relative = if options.preserve_structure {
            src.strip_prefix(&root_base).unwrap_or(src).to_path_buf()
        } else {
            PathBuf::from(src.file_name().unwrap_or_default())
        };
        let dst = target_dir.join(&relative);

        let result = acquire_one_file(src, &dst, &options);
        match result {
            Ok(mut af) => {
                bytes_read += af.size_bytes;
                af.target = dst;
                files.push(af);
            }
            Err(e) => files.push(AcquiredFile {
                source: src.to_path_buf(),
                target: dst,
                size_bytes: 0,
                source_sha256: String::new(),
                target_sha256: None,
                verified: false,
                error: Some(e.to_string()),
            }),
        }
    }

    let finished_at = Timestamp::now();
    let method = AcquisitionMethod::FileBased;

    let mut manifest = AcquisitionManifest::new(
        source.to_string_lossy().to_string(),
        target_dir.to_string_lossy().to_string(),
        method,
    );
    for f in &files {
        manifest.entries.push(ManifestEntry {
            relative_path: f
                .source
                .strip_prefix(&root_base)
                .unwrap_or(&f.source)
                .to_string_lossy()
                .replace('\\', "/"),
            size_bytes: f.size_bytes,
            sha256: f.source_sha256.clone(),
            verified: f.verified,
            error: f.error.clone(),
        });
    }
    manifest.finalize();

    Ok(AcquisitionResult {
        method,
        files,
        manifest,
        started_at,
        finished_at: Some(finished_at),
        bytes_read,
    })
}

/// Tek dosyayı kopyalar ve doğrular.
fn acquire_one_file(src: &Path, dst: &Path, options: &AcquisitionOptions) -> Result<AcquiredFile> {
    let meta = std::fs::metadata(src)?;
    let size = meta.len();

    if let Some(max) = options.max_file_bytes {
        if size > max {
            return Err(CoreError::Validation(format!(
                "Dosya boyutu sınırı aşıyor: {size} > {max}"
            )));
        }
    }

    if dst.exists() && options.no_overwrite {
        return Err(CoreError::Validation(format!(
            "Hedef dosya zaten var, üzerine yazılmadı: {}",
            dst.display()
        )));
    }

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Kaynak hash'i kopyalamadan önce al.
    let source_hash = hash_file(src, HashAlgorithm::Sha256)?;

    std::fs::copy(src, dst)?;

    // Zaman bilgilerini koru.
    if options.preserve_timestamps {
        // Dosya sistemi zamanlarını koru (mtime + atime).
        let mut times = std::fs::FileTimes::new();
        if let Ok(m) = meta.modified() {
            times = times.set_modified(m);
        }
        if let Ok(a) = meta.accessed() {
            times = times.set_accessed(a);
        }
        if let Ok(f) = std::fs::OpenOptions::new().write(true).open(dst) {
            let _ = f.set_times(times);
        }
    }

    // Hedef kopyayı tekrar oku ve doğrula.
    let (target_hash, verified) = if options.verify_after_copy {
        let th = hash_file(dst, HashAlgorithm::Sha256)?;
        let ok = th.value.eq_ignore_ascii_case(&source_hash.value);
        (Some(th.value), ok)
    } else {
        (None, false)
    };

    if options.verify_after_copy && !verified {
        return Err(CoreError::Data(format!(
            "Hedef kopya doğrulanamadı: {}",
            dst.display()
        )));
    }

    Ok(AcquiredFile {
        source: src.to_path_buf(),
        target: dst.to_path_buf(),
        size_bytes: size,
        source_sha256: source_hash.value,
        target_sha256: target_hash,
        verified,
        error: None,
    })
}

/// Dosya bazlı kopyanın koruma sınırlarını açıklar (5.4).
pub fn metadata_limits_note() -> &'static str {
    "Dosya bazlı kopya; dosya adı, boyut ve zaman bilgilerini korur. \
     Dosya sistemi metadata'sının (izinler, sahiplik, alternatif veri akışları, \
     silinmiş kayıtlar) tamamı korunmaz. Bu işlem fiziksel veya mantıksal imaj \
     yerine geçmez."
}

/// İki dosyanın aynı olduğunu doğrular (kaynak-hedef karşılaştırması).
pub fn verify_copy(source: &Path, target: &Path) -> Result<bool> {
    verify_identical(source, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn acquires_single_file_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("kayit.wav");
        std::fs::write(&src, b"RIFF....WAVEfmt ").unwrap();
        let out = dir.path().join("kasa");
        let r = acquire_path(&src, &out, AcquisitionOptions::default()).unwrap();
        assert_eq!(r.files.len(), 1);
        assert!(r.files[0].verified);
        assert!(r.all_verified());
        assert!(out.join("kayit.wav").exists());
    }

    #[test]
    fn acquires_directory_preserving_structure() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("materyal");
        std::fs::create_dir_all(src.join("alt")).unwrap();
        std::fs::write(src.join("a.txt"), b"a").unwrap();
        std::fs::write(src.join("alt/b.txt"), b"b").unwrap();
        let out = dir.path().join("kasa");
        let r = acquire_path(&src, &out, AcquisitionOptions::default()).unwrap();
        assert_eq!(r.files.len(), 2);
        assert!(out.join("a.txt").exists());
        assert!(out.join("alt/b.txt").exists());
    }

    #[test]
    fn does_not_overwrite_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("x.bin");
        std::fs::write(&src, b"yeni").unwrap();
        let out = dir.path().join("kasa");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("x.bin"), b"eski").unwrap();
        let r = acquire_path(&src, &out, AcquisitionOptions::default()).unwrap();
        assert_eq!(r.files.len(), 1);
        assert!(r.files[0].error.is_some());
        assert_eq!(std::fs::read(out.join("x.bin")).unwrap(), b"eski");
    }

    #[test]
    fn detects_corrupted_copy() {
        // Hedefi kopyalamadan sonra bozarak doğrulama başarısızlığını test
        // edemeyiz; bunun yerine hash karşılaştırmasını doğrudan sınıyoruz.
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        std::fs::write(&a, b"veri").unwrap();
        std::fs::write(&b, b"VERI").unwrap();
        assert!(!verify_copy(&a, &b).unwrap());
    }

    #[test]
    fn label_does_not_claim_image() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("f.bin");
        std::fs::write(&src, b"x").unwrap();
        let r = acquire_path(&src, dir.path().join("k"), AcquisitionOptions::default()).unwrap();
        assert!(r.label().contains("imaj değildir"));
        assert!(!r.label().contains("E01"));
        assert!(!r.label().contains("L01"));
    }

    #[test]
    fn manifest_records_hashes_and_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("m");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.bin"), b"abc").unwrap();
        let r = acquire_path(&src, dir.path().join("k"), AcquisitionOptions::default()).unwrap();
        assert_eq!(r.manifest.entries.len(), 1);
        let e = &r.manifest.entries[0];
        assert_eq!(e.size_bytes, 3);
        assert_eq!(e.sha256.len(), 64);
        assert!(e.verified);
        assert!(r.manifest.manifest_sha256.is_some());
    }

    #[test]
    fn max_file_size_is_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("buyuk.bin");
        let mut f = std::fs::File::create(&src).unwrap();
        f.write_all(&vec![0u8; 4096]).unwrap();
        let opts = AcquisitionOptions {
            max_file_bytes: Some(1024),
            ..Default::default()
        };
        let r = acquire_path(&src, dir.path().join("k"), opts).unwrap();
        assert!(r.files[0].error.is_some());
        assert!(!r.all_verified());
    }

    #[test]
    fn metadata_limits_note_mentions_image_boundary() {
        let n = metadata_limits_note();
        assert!(n.contains("imaj"));
    }
}
