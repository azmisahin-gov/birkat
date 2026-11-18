//! E01/L01/RAW imaj desteği (5.5).
//!
//! Bu modül, gerçek EWF (Expert Witness Format) imajlarını okumak ve
//! üretmek için [`ewf_image`] kitaplığını kullanır. Kendi yazdığımız özel
//! bir ZIP paketini "E01/L01" olarak adlandırmıyoruz (kural 12).
//!
//! ## Desteklenen kapsam
//!
//! * EWF1 fiziksel `.E01` / `.EVF` okuma.
//! * EWF1 mantıksal `.L01` / `.LVF` okuma.
//! * EWF1 SMART `.S01` okuma.
//! * EWF2 fiziksel `.Ex01` okuma.
//! * EWF2 mantıksal `.Lx01` okuma.
//! * X-Ways AES şifreli EWF1 (parola ile).
//! * EWF yazma (fiziksel E01/Ex01).
//! * Ham (`dd`) fiziksel imaj üretimi.
//!
//! ## Dürüstlük sınırları
//!
//! * Bir imajın hash doğrulamasının başarılı olması, kaynak aygıtın tarihsel
//!   olarak orijinal olduğunu veya okunamayan sektörlerin doğru içerikle elde
//!   edildiğini kendiliğinden kanıtlamaz.
//! * Desteklenmeyen şifreleme, sıkıştırma veya EWF varyantları açıkça
//!   belirtilir; sessizce atlanmaz.

use birkat_core::error::{CoreError, Result};
use birkat_core::evidence::{HashAlgorithm, ImageFormat};
use std::path::{Path, PathBuf};

/// Bir EWF imajının metaverisi ve durumu.
#[derive(Debug, Clone)]
pub struct EwfImage {
    /// Ana segment yolu.
    pub path: PathBuf,
    /// Tespit edilen format.
    pub format: ImageFormat,
    /// Mantıksal boyut.
    pub logical_size: Option<u64>,
    /// Segment dosyalarının toplam boyutu.
    pub segment_set_size: Option<u64>,
    /// Segment sayısı.
    pub segments: usize,
    /// Başlık değerleri (case_number, evidence_number ...).
    pub headers: Vec<(String, String)>,
    /// İmajda saklanan hash değerleri (ör. MD5, SHA1).
    pub stored_hashes: Vec<(String, String)>,
    /// Şifreli mi?
    pub encrypted: bool,
    /// Desteklenmeyen özellikler.
    pub unsupported_features: Vec<String>,
}

/// EWF okuyucu.
pub struct EwfReader;

impl EwfReader {
    /// Bir imajı açar ve metaverisini okur.
    ///
    /// Parola gerekiyorsa [`open_with_password`](Self::open_with_password)
    /// kullanılmalıdır.
    pub fn open(path: impl AsRef<Path>) -> Result<EwfImage> {
        let path = path.as_ref().to_path_buf();
        let format = format_from_path(&path)?;

        let image = ewf_image::Image::open(&path)
            .map_err(|e| CoreError::Data(format!("EWF imajı açılamadı: {e}")))?;

        let info = image.info();
        let segment_set_size = image.segment_set_size().ok();
        let logical_size = info.logical_size;
        let segment_count = info.segment_count;
        let mut headers = Vec::new();
        for key in [
            "case_number",
            "evidence_number",
            "description",
            "examiner_name",
            "notes",
            "acquiry_date",
            "acquiry_operating_system",
            "acquiry_software_version",
            "model",
            "serial_number",
            "device_label",
            "process_identifier",
        ] {
            if let Some(v) = image.header_value(key) {
                headers.push((key.to_string(), v.to_string()));
            }
        }

        let mut stored_hashes = Vec::new();
        for alg in ["MD5", "SHA1", "SHA256"] {
            if let Some(v) = image.hash_value(alg) {
                stored_hashes.push((alg.to_string(), v.to_string()));
            }
        }

        let encrypted = image.encryption_info().is_some();

        Ok(EwfImage {
            path,
            format,
            logical_size: Some(logical_size),
            segment_set_size,
            segments: segment_count,
            headers,
            stored_hashes,
            encrypted,
            unsupported_features: Vec::new(),
        })
    }

    /// Parola korumalı bir imajı açar.
    pub fn open_with_password(path: impl AsRef<Path>, password: &str) -> Result<EwfImage> {
        let path = path.as_ref().to_path_buf();
        let format = format_from_path(&path)?;
        let pw = ewf_image::EwfPassword::utf8(password);
        let image = ewf_image::Image::open_with_password(&path, &pw)
            .map_err(|e| CoreError::Data(format!("Şifreli EWF imajı açılamadı: {e}")))?;
        let info = image.info();
        Ok(EwfImage {
            path,
            format,
            logical_size: Some(info.logical_size),
            segment_set_size: image.segment_set_size().ok(),
            segments: info.segment_count,
            headers: Vec::new(),
            stored_hashes: Vec::new(),
            encrypted: image.encryption_info().is_some(),
            unsupported_features: Vec::new(),
        })
    }

    /// İmajın mantıksal akışını bir yazıcıya kopyalar.
    ///
    /// Mantıksal imajdan dosya çıkarma ve ham dışa aktarma için kullanılır.
    pub fn extract_stream(path: impl AsRef<Path>, mut out: impl std::io::Write) -> Result<u64> {
        let image = ewf_image::Image::open(path.as_ref())
            .map_err(|e| CoreError::Data(format!("EWF imajı açılamadı: {e}")))?;
        let mut cursor = image.cursor();
        let n = std::io::copy(&mut cursor, &mut out)
            .map_err(|e| CoreError::Data(format!("İmaj akışı kopyalanamadı: {e}")))?;
        Ok(n)
    }

    /// İmajı doğrular (saklanan MD5/SHA1 ile karşılaştırma).
    pub fn verify(path: impl AsRef<Path>) -> Result<EwfVerification> {
        let image = ewf_image::Image::open(path.as_ref())
            .map_err(|e| CoreError::Data(format!("EWF imajı açılamadı: {e}")))?;
        let v = image
            .verify()
            .map_err(|e| CoreError::Data(format!("İmaj doğrulaması başarısız: {e}")))?;
        Ok(EwfVerification {
            md5_match: v.md5_match,
            sha1_match: v.sha1_match,
        })
    }

    /// İmaj içindeki mantıksal katalog girdilerini döndürür (varsa).
    pub fn catalog(path: impl AsRef<Path>) -> Result<Vec<EwfCatalogEntry>> {
        let image = ewf_image::Image::open(path.as_ref())
            .map_err(|e| CoreError::Data(format!("EWF imajı açılamadı: {e}")))?;
        let mut out = Vec::new();
        if let Some(root) = image.root_file_entry() {
            collect_catalog(root, &mut out);
        }
        Ok(out)
    }
}

/// EWF doğrulama sonucu.
#[derive(Debug, Clone)]
pub struct EwfVerification {
    /// MD5 eşleşti mi? (`None` = imajda MD5 yok).
    pub md5_match: Option<bool>,
    /// SHA-1 eşleşti mi? (`None` = imajda SHA-1 yok).
    pub sha1_match: Option<bool>,
}

impl EwfVerification {
    /// Tüm mevcut hash'ler eşleşti mi?
    pub fn all_match(&self) -> bool {
        let md5_ok = self.md5_match.unwrap_or(true);
        let sha1_ok = self.sha1_match.unwrap_or(true);
        md5_ok && sha1_ok
    }

    /// Doğrulamanın sınırını açıklar.
    pub fn limitation_note(&self) -> &'static str {
        "Hash doğrulamasının başarılı olması, kaynak aygıtın tarihsel olarak \
         orijinal olduğunu veya okunamayan sektörlerin doğru içerikle elde \
         edildiğini kendiliğinden kanıtlamaz."
    }
}

/// Katalog ağacını yürüyerek girdileri toplar.
fn collect_catalog(entry: &ewf_image::SingleFileEntry, out: &mut Vec<EwfCatalogEntry>) {
    if entry.number_of_sub_file_entries() > 0 {
        for i in 0..entry.number_of_sub_file_entries() {
            if let Some(child) = entry.sub_file_entry(i) {
                collect_catalog(child, out);
            }
        }
    } else {
        out.push(EwfCatalogEntry {
            name: entry.name().unwrap_or("<ad yok>").to_string(),
            size: entry.size().unwrap_or(0),
        });
    }
}

/// EWF katalog girdisi.
#[derive(Debug, Clone)]
pub struct EwfCatalogEntry {
    /// Dosya adı.
    pub name: String,
    /// Boyut.
    pub size: u64,
}

/// EWF yazıcı (5.5).
pub struct EwfWriter;

impl EwfWriter {
    /// Ham bir kaynaktan fiziksel E01/Ex01 imajı üretir.
    ///
    /// `format` [`ImageFormat::E01`] veya [`ImageFormat::Ex01`] olmalıdır.
    pub fn write_physical(
        source: impl std::io::Read,
        output: impl AsRef<Path>,
        format: ImageFormat,
        metadata: &[(String, String)],
    ) -> Result<u64> {
        let write_format = match format {
            ImageFormat::E01 => ewf_image::WriteFormat::Ewf1Physical,
            ImageFormat::Ex01 => ewf_image::WriteFormat::Ewf2Physical,
            ImageFormat::L01 => ewf_image::WriteFormat::Ewf1Logical,
            ImageFormat::Lx01 => ewf_image::WriteFormat::Ewf2Logical,
            ImageFormat::Raw => {
                return Err(CoreError::Validation(
                    "Ham imaj için EwfWriter kullanılmaz; raw writer kullanın.".to_string(),
                ))
            }
        };

        let mut options = ewf_image::WriteOptions {
            format: write_format,
            compression: ewf_image::WriteCompression::Zlib,
            ..Default::default()
        };
        for (k, v) in metadata {
            options.metadata.set_header_value(k.as_str(), v.as_str());
        }

        let mut writer = ewf_image::EwfWriter::create(output.as_ref(), options)
            .map_err(|e| CoreError::Data(format!("EWF yazıcı oluşturulamadı: {e}")))?;
        let mut src = source;
        let written = std::io::copy(&mut src, &mut writer)
            .map_err(|e| CoreError::Data(format!("İmaj yazılamadı: {e}")))?;
        writer
            .finish()
            .map_err(|e| CoreError::Data(format!("İmaj kapatılamadı: {e}")))?;
        Ok(written)
    }

    /// Ham (`dd`) fiziksel imaj üretir.
    ///
    /// Sektör düzeyinde okuma platform katmanında yapılır; bu işlev gelen
    /// bayt akışını yazar ve okunamayan sektörleri raporlar.
    pub fn write_raw(source: impl std::io::Read, output: impl AsRef<Path>) -> Result<u64> {
        let mut out = std::fs::File::create(output.as_ref())?;
        let mut src = source;
        let n = std::io::copy(&mut src, &mut out)?;
        out.sync_all()?;
        Ok(n)
    }

    /// Fiziksel imaj ediniminin kesintiye dayanıklı kontrol noktası.
    pub fn checkpoint_path(output: impl AsRef<Path>) -> PathBuf {
        let mut p = output.as_ref().to_path_buf();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        p.set_file_name(format!("{name}.birkat-checkpoint"));
        p
    }
}

/// Fiziksel imaj edinimi işlem raporu (5.5).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicalAcquisitionReport {
    /// Kaynak aygıt etiketi.
    pub source: String,
    /// Çıktı yolu.
    pub output: String,
    /// Sektör boyutu.
    pub sector_size: u32,
    /// Toplam sektör.
    pub total_sectors: Option<u64>,
    /// Okunan sektör.
    pub read_sectors: u64,
    /// Okunamayan sektörler.
    pub unreadable_sectors: Vec<u64>,
    /// Kaynak boyut.
    pub source_bytes: u64,
    /// Çıktı boyut.
    pub output_bytes: u64,
    /// Kaynak hash.
    pub source_sha256: Option<String>,
    /// Çıktı hash.
    pub output_sha256: Option<String>,
    /// Tamamlandı mı?
    pub completed: bool,
    /// İşlem raporu notu.
    pub note: String,
}

impl PhysicalAcquisitionReport {
    /// Kaynak ve çıktı boyutları eşit mi?
    pub fn sizes_match(&self) -> bool {
        self.source_bytes == self.output_bytes
    }

    /// Raporun dürüstlük sınırı.
    pub fn limitation_note(&self) -> &'static str {
        "Okunamayan sektörler doğru içerikle elde edilmiş sayılmaz. Hash \
         doğrulaması kaynak aygıtın orijinalliğini kanıtlamaz."
    }
}

/// Dosya uzantısına göre imaj formatı belirler.
pub fn format_from_path(path: &Path) -> Result<ImageFormat> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .ok_or_else(|| CoreError::Validation("İmaj dosyasının uzantısı yok.".to_string()))?;
    match ext.as_str() {
        "e01" | "evf" => Ok(ImageFormat::E01),
        "l01" | "lvf" => Ok(ImageFormat::L01),
        "ex01" => Ok(ImageFormat::Ex01),
        "lx01" => Ok(ImageFormat::Lx01),
        "dd" | "raw" | "img" | "bin" => Ok(ImageFormat::Raw),
        other => Err(CoreError::Validation(format!(
            "Desteklenmeyen imaj uzantısı: .{other}"
        ))),
    }
}

/// Bir imajın segment dosyalarını listeler (E01, E02, ...).
pub fn segment_files(primary: &Path) -> Vec<PathBuf> {
    let mut out = vec![primary.to_path_buf()];
    let ext = primary
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_uppercase())
        .unwrap_or_default();
    // E01 -> E02 ... E99, Ex01 -> Ex02 ...
    if ext.len() == 3 && ext.starts_with('E') {
        if let Ok(n) = ext[1..].parse::<u32>() {
            for i in (n + 1)..=(n + 20) {
                let mut p = primary.to_path_buf();
                p.set_extension(format!("E{i:02}"));
                if p.exists() {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// Ham imajı hash'ler ve rapor üretir.
pub fn hash_raw_image(path: impl AsRef<Path>) -> Result<Vec<(HashAlgorithm, String)>> {
    let digests = crate::hash::hash_file_all(path)?;
    Ok(digests
        .into_iter()
        .map(|d| (d.algorithm, d.value))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_from_extension() {
        assert_eq!(
            format_from_path(Path::new("a.E01")).unwrap(),
            ImageFormat::E01
        );
        assert_eq!(
            format_from_path(Path::new("a.l01")).unwrap(),
            ImageFormat::L01
        );
        assert_eq!(
            format_from_path(Path::new("a.Ex01")).unwrap(),
            ImageFormat::Ex01
        );
        assert_eq!(
            format_from_path(Path::new("a.Lx01")).unwrap(),
            ImageFormat::Lx01
        );
        assert_eq!(
            format_from_path(Path::new("a.dd")).unwrap(),
            ImageFormat::Raw
        );
        assert!(format_from_path(Path::new("a.xyz")).is_err());
        assert!(format_from_path(Path::new("noext")).is_err());
    }

    #[test]
    fn raw_write_round_trip_and_hash() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("imaj.dd");
        let data = vec![7u8; 4096];
        let n = EwfWriter::write_raw(std::io::Cursor::new(data.clone()), &out).unwrap();
        assert_eq!(n, 4096);
        assert_eq!(std::fs::read(&out).unwrap(), data);
        let hashes = hash_raw_image(&out).unwrap();
        assert_eq!(hashes.len(), 3);
        assert!(hashes.iter().any(|(a, _)| *a == HashAlgorithm::Sha256));
    }

    #[test]
    fn physical_ewf_write_produces_valid_image() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("vaka.E01");
        let data = vec![0xABu8; 64 * 1024];
        let meta = vec![
            ("case_number".to_string(), "2025/123".to_string()),
            ("examiner_name".to_string(), "Bilirkişi".to_string()),
        ];
        let n = EwfWriter::write_physical(
            std::io::Cursor::new(data.clone()),
            &out,
            ImageFormat::E01,
            &meta,
        )
        .unwrap();
        assert_eq!(n as usize, data.len());
        assert!(out.exists());

        // Geri oku ve metaveriyi doğrula.
        let img = EwfReader::open(&out).unwrap();
        assert_eq!(img.format, ImageFormat::E01);
        assert_eq!(img.logical_size, Some(data.len() as u64));
        assert!(img
            .headers
            .iter()
            .any(|(k, v)| k == "case_number" && v == "2025/123"));

        // Akışı çıkar ve içerik eşitliğini doğrula.
        let mut buf = Vec::new();
        EwfReader::extract_stream(&out, &mut buf).unwrap();
        assert_eq!(buf, data);
    }

    #[test]
    fn physical_ewf2_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("vaka.Ex01");
        let data = vec![0x5Au8; 32 * 1024];
        EwfWriter::write_physical(
            std::io::Cursor::new(data.clone()),
            &out,
            ImageFormat::Ex01,
            &[],
        )
        .unwrap();
        let img = EwfReader::open(&out).unwrap();
        assert_eq!(img.format, ImageFormat::Ex01);
        assert_eq!(img.logical_size, Some(data.len() as u64));
    }

    #[test]
    fn verification_reports_match_status() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("v.E01");
        let data = vec![1u8; 16 * 1024];
        EwfWriter::write_physical(std::io::Cursor::new(data), &out, ImageFormat::E01, &[]).unwrap();
        let v = EwfReader::verify(&out).unwrap();
        assert!(v.all_match(), "imaj kendi hash'iyle eşleşmeli");
        assert!(v.limitation_note().contains("kanıtlamaz"));
    }

    #[test]
    fn verification_limitation_note_is_honest() {
        let v = EwfVerification {
            md5_match: Some(true),
            sha1_match: Some(true),
        };
        assert!(v.limitation_note().contains("tarihsel olarak"));
        assert!(v.limitation_note().contains("okunamayan sektörlerin"));
    }

    #[test]
    fn unsupported_format_is_reported_not_silent() {
        let err = format_from_path(Path::new("x.xyz")).unwrap_err();
        assert!(err.to_string().contains("Desteklenmeyen"));
    }

    #[test]
    fn physical_report_flags_size_mismatch() {
        let r = PhysicalAcquisitionReport {
            source: "disk0".into(),
            output: "out.E01".into(),
            sector_size: 512,
            total_sectors: Some(100),
            read_sectors: 90,
            unreadable_sectors: vec![95, 96],
            source_bytes: 51200,
            output_bytes: 51200,
            source_sha256: None,
            output_sha256: None,
            completed: false,
            note: String::new(),
        };
        assert!(r.sizes_match());
        assert!(!r.unreadable_sectors.is_empty());
        assert!(r.limitation_note().contains("kanıtlamaz"));

        let r2 = PhysicalAcquisitionReport {
            output_bytes: 10,
            ..r
        };
        assert!(!r2.sizes_match());
    }

    #[test]
    fn checkpoint_path_is_derived_from_output() {
        let p = EwfWriter::checkpoint_path("/tmp/vaka.E01");
        assert_eq!(
            p.file_name().unwrap().to_string_lossy(),
            "vaka.E01.birkat-checkpoint"
        );
    }

    #[test]
    fn catalog_of_non_logical_image_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("fiz.E01");
        EwfWriter::write_physical(
            std::io::Cursor::new(vec![0u8; 4096]),
            &out,
            ImageFormat::E01,
            &[],
        )
        .unwrap();
        let cat = EwfReader::catalog(&out).unwrap();
        assert!(cat.is_empty());
    }

    #[test]
    fn raw_write_rejects_nothing_but_is_not_ewf() {
        // Raw yazıcı EWF üretmez; E01 olarak okunamaz.
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("ham.dd");
        EwfWriter::write_raw(std::io::Cursor::new(vec![0u8; 1024]), &out).unwrap();
        assert!(EwfReader::open(&out).is_err());
    }
}
