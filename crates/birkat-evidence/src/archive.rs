//! Güvenli arşiv açma (4.1).
//!
//! Dosya açılırken bozuk arşiv, arşiv bombası, aşırı sıkıştırılmış dosya,
//! zararlı yol adı ve benzeri güvenlik riskleri denetlenir.
//!
//! ## Denetimler
//!
//! * **Yol kaçışı (zip slip):** girdi adları `..`, mutlak yol veya Windows
//!   sürücü harfi içeremez.
//! * **Arşiv bombası:** sıkıştırma oranı ve toplam açılmış boyut sınırlıdır.
//! * **Girdi sayısı:** aşırı girdi sayısı reddedilir.
//! * **Simge bağlantı/nokta:** sembolik bağlantı ve aygıt girdileri atlanır.

use birkat_core::error::{CoreError, Result};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// Güvenlik sınırları.
#[derive(Debug, Clone, Copy)]
pub struct ArchiveLimits {
    /// Azami girdi sayısı.
    pub max_entries: usize,
    /// Azami toplam açılmış boyut (bayt).
    pub max_total_bytes: u64,
    /// Tek girdi için azami sıkıştırma oranı (açılmış/sıkıştırılmış).
    pub max_compression_ratio: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_total_bytes: 64 * 1024 * 1024 * 1024, // 64 GiB
            max_compression_ratio: 1_000,
        }
    }
}

/// Arşiv girdisi özeti.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Arşiv içindeki ad.
    pub name: String,
    /// Sıkıştırılmış boyut.
    pub compressed_size: u64,
    /// Açılmış boyut.
    pub uncompressed_size: u64,
}

/// Denetim sonucu.
#[derive(Debug, Clone)]
pub struct ArchiveInspection {
    /// Girdiler.
    pub entries: Vec<ArchiveEntry>,
    /// Toplam açılmış boyut.
    pub total_uncompressed: u64,
    /// Uyarılar (güvenli sayılan ama bildirilen durumlar).
    pub warnings: Vec<String>,
}

/// Girdi adının güvenli olup olmadığını denetler.
///
/// Zararlı yol adı: `..`, mutlak yol, sürücü harfi, NUL baytı, ters bölü ile
/// gizlenmiş kaçış.
pub fn is_safe_entry_name(name: &str) -> bool {
    if name.is_empty() || name.contains('\0') {
        return false;
    }
    // Windows sürücü harfi (C:\...) veya UNC (\\) yasak.
    let bytes = name.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return false;
    }
    if name.starts_with("\\\\") || name.starts_with("//") {
        return false;
    }
    let normalized = name.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute() {
        return false;
    }
    for comp in path.components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
            Component::CurDir => {}
            Component::Normal(_) => {}
        }
    }
    true
}

/// ZIP arşivini açar ve güvenlik denetimlerini uygular.
///
/// `dest` klasörüne dosyaları çıkarır. Denetim başarısız olursa hiçbir dosya
/// yazılmaz.
pub fn extract_zip_safely(
    zip_path: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    limits: ArchiveLimits,
) -> Result<ArchiveInspection> {
    let file = std::fs::File::open(zip_path.as_ref())?;
    extract_zip_reader_safely(file, dest, limits)
}

/// Bir `Read` kaynağından ZIP arşivini güvenli biçimde çıkarır.
pub fn extract_zip_reader_safely<R: Read + std::io::Seek>(
    reader: R,
    dest: impl AsRef<Path>,
    limits: ArchiveLimits,
) -> Result<ArchiveInspection> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| CoreError::Data(format!("Arşiv açılamadı: {e}")))?;

    if archive.len() > limits.max_entries {
        return Err(CoreError::Validation(format!(
            "Arşiv girdi sayısı sınırı aşıyor: {} > {}",
            archive.len(),
            limits.max_entries
        )));
    }

    let mut entries = Vec::new();
    let mut total = 0u64;
    let mut warnings = Vec::new();

    // 1. Aşama: denetim (hiçbir şey yazmadan).
    for i in 0..archive.len() {
        let f = archive
            .by_index(i)
            .map_err(|e| CoreError::Data(format!("Arşiv girdisi okunamadı: {e}")))?;
        let name = f
            .name()
            .map_err(|e| CoreError::Data(format!("Arşiv girdi adı okunamadı: {e}")))?
            .to_string();

        if !is_safe_entry_name(&name) {
            return Err(CoreError::Validation(format!(
                "Arşivde zararlı yol adı: {name}"
            )));
        }
        if f.is_dir() {
            continue;
        }
        // Sembolik bağlantı / aygıt girdileri atlanır.
        if let Some(mode) = f.unix_mode() {
            let file_type = mode & 0o170000;
            if file_type == 0o120000 || file_type == 0o020000 || file_type == 0o060000 {
                warnings.push(format!("Sembolik bağlantı/aygıt girdisi atlandı: {name}"));
                continue;
            }
        }

        let uncompressed = f.size();
        let compressed = f.compressed_size().max(1);
        let ratio = uncompressed / compressed;
        if ratio > limits.max_compression_ratio {
            return Err(CoreError::Validation(format!(
                "Olası arşiv bombası: {name} sıkıştırma oranı {ratio} > {}",
                limits.max_compression_ratio
            )));
        }
        total = total.saturating_add(uncompressed);
        if total > limits.max_total_bytes {
            return Err(CoreError::Validation(
                "Arşiv toplam açılmış boyutu sınırı aşıyor (olası arşiv bombası).".to_string(),
            ));
        }
        entries.push(ArchiveEntry {
            name,
            compressed_size: compressed,
            uncompressed_size: uncompressed,
        });
    }

    // 2. Aşama: çıkarma.
    let dest = dest.as_ref();
    std::fs::create_dir_all(dest)?;
    for entry in &entries {
        let out_path = safe_join(dest, &entry.name)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = archive
            .by_name(&entry.name)
            .map_err(|e| CoreError::Data(format!("Girdi açılamadı: {e}")))?;
        let mut out = std::fs::File::create(&out_path)?;
        std::io::copy(&mut f, &mut out)?;
    }

    Ok(ArchiveInspection {
        entries,
        total_uncompressed: total,
        warnings,
    })
}

/// Arşiv girdi adını hedef klasör altına güvenli biçimde birleştirir.
///
/// Sonuç yolun hedefin altında kaldığını doğrular.
pub fn safe_join(base: &Path, name: &str) -> Result<PathBuf> {
    if !is_safe_entry_name(name) {
        return Err(CoreError::Validation(format!(
            "Güvensiz arşiv girdi adı: {name}"
        )));
    }
    let normalized = name.replace('\\', "/");
    let joined = base.join(normalized);
    // Sembolik çözümleme olmadan önek denetimi.
    if !joined.starts_with(base) {
        return Err(CoreError::Validation(format!(
            "Arşiv girdisi hedef klasörün dışına çıkıyor: {name}"
        )));
    }
    Ok(joined)
}

/// ZIP, 7z ve RAR arşivlerini tespit edip güvenli çıkarım yapar.
///
/// 7z ve RAR için bağımlılık eklenmediğinden, bu biçimler açık bir hata ile
/// bildirilir; sessizce atlanmaz.
pub fn extract_archive_safely(
    path: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    limits: ArchiveLimits,
) -> Result<ArchiveInspection> {
    let kind = crate::detect::detect_kind(path.as_ref())?;
    match kind {
        birkat_core::evidence::DetectedKind::Zip
        | birkat_core::evidence::DetectedKind::Docx
        | birkat_core::evidence::DetectedKind::Odt
        | birkat_core::evidence::DetectedKind::Udf => extract_zip_safely(path, dest, limits),
        birkat_core::evidence::DetectedKind::SevenZip => Err(CoreError::Validation(
            "7z arşivi bu sürümde açılamıyor: desteklenen arşiv biçimi değil.".to_string(),
        )),
        birkat_core::evidence::DetectedKind::Rar => Err(CoreError::Validation(
            "RAR arşivi bu sürümde açılamıyor: desteklenen arşiv biçimi değil.".to_string(),
        )),
        other => Err(CoreError::Validation(format!(
            "Desteklenmeyen arşiv biçimi: {}",
            other.label()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let p = dir.join("arsiv.zip");
        let f = std::fs::File::create(&p).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let opts: zip::write::SimpleFileOptions =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            z.start_file(*name, opts).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
        p
    }

    #[test]
    fn safe_names_are_accepted() {
        assert!(is_safe_entry_name("belge.pdf"));
        assert!(is_safe_entry_name("klasor/alt/belge.pdf"));
        assert!(is_safe_entry_name("klasor\\alt\\belge.pdf"));
    }

    #[test]
    fn traversal_names_are_rejected() {
        assert!(!is_safe_entry_name("../gizli.txt"));
        assert!(!is_safe_entry_name("a/../../b.txt"));
        assert!(!is_safe_entry_name("/etc/passwd"));
        assert!(!is_safe_entry_name("C:\\Windows\\system32"));
        assert!(!is_safe_entry_name("\\\\sunucu\\paylasim"));
        assert!(!is_safe_entry_name("a\0b"));
        assert!(!is_safe_entry_name(""));
    }

    #[test]
    fn safe_join_blocks_escape() {
        let base = Path::new("/tmp/hedef");
        assert!(safe_join(base, "a/b.txt").is_ok());
        assert!(safe_join(base, "../kacis.txt").is_err());
    }

    #[test]
    fn extracts_valid_zip() {
        let dir = tempfile::tempdir().unwrap();
        let zip = make_zip(dir.path(), &[("a.txt", b"bir"), ("alt/b.txt", b"iki")]);
        let out = dir.path().join("cikti");
        let insp = extract_zip_safely(&zip, &out, ArchiveLimits::default()).unwrap();
        assert_eq!(insp.entries.len(), 2);
        assert_eq!(std::fs::read(out.join("a.txt")).unwrap(), b"bir");
        assert_eq!(std::fs::read(out.join("alt/b.txt")).unwrap(), b"iki");
    }

    #[test]
    fn rejects_zip_with_traversal_entry() {
        let dir = tempfile::tempdir().unwrap();
        // zip crate yazarken ".." kabul etmez; elle bozuk ZIP üretmek yerine
        // ad denetimini doğrudan test ediyoruz ve çıkarma yolunu da deniyoruz.
        let zip = make_zip(dir.path(), &[("iyi.txt", b"x")]);
        let out = dir.path().join("cikti");
        assert!(extract_zip_safely(&zip, &out, ArchiveLimits::default()).is_ok());
        assert!(!is_safe_entry_name("../kacis.txt"));
    }

    #[test]
    fn rejects_too_many_entries() {
        let dir = tempfile::tempdir().unwrap();
        let entries: Vec<(String, &[u8])> = (0..20)
            .map(|i| (format!("f{i}.txt"), b"x" as &[u8]))
            .collect();
        let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, d)| (n.as_str(), *d)).collect();
        let zip = make_zip(dir.path(), &refs);
        let limits = ArchiveLimits {
            max_entries: 5,
            ..Default::default()
        };
        let err = extract_zip_safely(&zip, dir.path().join("c"), limits).unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }

    #[test]
    fn rejects_compression_bomb_ratio() {
        let dir = tempfile::tempdir().unwrap();
        // Yüksek oranlı sıkışan veri: 1 MiB sıfır.
        let big = vec![0u8; 1024 * 1024];
        let zip = make_zip(dir.path(), &[("bomba.bin", &big)]);
        let limits = ArchiveLimits {
            max_compression_ratio: 10,
            ..Default::default()
        };
        let err = extract_zip_safely(&zip, dir.path().join("c"), limits).unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }

    #[test]
    fn unsupported_archive_formats_report_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.7z");
        std::fs::write(&p, b"7z\xBC\xAF\x27\x1C...").unwrap();
        let err =
            extract_archive_safely(&p, dir.path().join("c"), ArchiveLimits::default()).unwrap_err();
        assert!(err.to_string().contains("7z"));
    }
}
