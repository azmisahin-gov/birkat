//! UDF doğrulayıcı (15.3).
//!
//! Üretilen UDF şu yönlerden kontrol edilir:
//!
//! * Paket bütünlüğü (ZIP açılabiliyor mu, `content.xml` var mı).
//! * XML yapısı (köke, zorunlu bölümlere).
//! * Metin/ofset tutarlılığı (her offset havuz sınırları içinde mi).
//! * Biçim aralıkları (offset + length taşmıyor mu).
//! * Tablo yapısı (sütun sayısı ve hücre sayısı tutarlı mı).
//! * Görsel referansları (base64 geçerli mi, genişlik/yükseklik pozitif mi).
//! * Sayfa yapısı (kenar boşlukları, yön).
//! * Türkçe karakterler (havuz ve metin UTF-8 olarak korunmuş mu).
//! * Belge yeniden açılabilirliği (kendi okuyucumuz çözebiliyor mu).
//!
//! Doğrulama başarısızsa dosya "hazır" olarak gösterilmez.

use crate::model::*;
use crate::{reader, UdfReadResult};
use serde::{Deserialize, Serialize};

/// Doğrulama bulgu düzeyi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckSeverity {
    /// Bilgi.
    Info,
    /// Uyarı; dosya kullanılabilir ama gözden geçirilmeli.
    Warning,
    /// Hata; dosya "hazır" sayılamaz.
    Error,
}

/// Tek bir doğrulama bulgusu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Kontrol adı.
    pub name: String,
    /// Düzey.
    pub severity: CheckSeverity,
    /// Açıklama.
    pub message: String,
}

/// Doğrulama sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfValidation {
    /// Tüm kontroller geçti mi (hata yok)?
    pub ok: bool,
    /// Kontroller.
    pub checks: Vec<CheckResult>,
    /// Hata sayısı.
    pub error_count: usize,
    /// Uyarı sayısı.
    pub warning_count: usize,
    /// Havuzdaki karakter sayısı.
    pub pool_chars: usize,
    /// En büyük referans edilen offset + length.
    pub max_referenced_offset: usize,
    /// Türkçe karakter içeriyor mu?
    pub has_turkish_characters: bool,
}

impl UdfValidation {
    /// Hataları döndürür.
    pub fn errors(&self) -> Vec<&CheckResult> {
        self.checks
            .iter()
            .filter(|c| c.severity == CheckSeverity::Error)
            .collect()
    }

    /// Uyarıları döndürür.
    pub fn warnings(&self) -> Vec<&CheckResult> {
        self.checks
            .iter()
            .filter(|c| c.severity == CheckSeverity::Warning)
            .collect()
    }

    /// İnsan okuyabilir Türkçe özet.
    pub fn summary(&self) -> String {
        if self.ok {
            format!(
                "UDF doğrulaması başarılı. {} kontrol yapıldı, {} uyarı.",
                self.checks.len(),
                self.warning_count
            )
        } else {
            format!(
                "UDF doğrulaması başarısız: {} hata, {} uyarı.",
                self.error_count, self.warning_count
            )
        }
    }
}

/// Türkçe'ye özgü karakterleri sayar.
pub fn count_turkish_characters(text: &str) -> usize {
    const TURKISH: &[char] = &[
        'ç', 'Ç', 'ğ', 'Ğ', 'ı', 'İ', 'ö', 'Ö', 'ş', 'Ş', 'ü', 'Ü', 'â', 'Â', 'î', 'Î', 'û', 'Û',
    ];
    text.chars().filter(|c| TURKISH.contains(c)).count()
}

/// Belgeyi doğrular (model üzerinden).
pub fn validate_document(doc: &UdfDocument) -> UdfValidation {
    let mut checks = Vec::new();

    // 1. Sayfa yapısı
    let p = &doc.page;
    if p.left_margin < 0.0 || p.right_margin < 0.0 || p.top_margin < 0.0 || p.bottom_margin < 0.0 {
        checks.push(err(
            "page_margins",
            "Sayfa kenar boşlukları negatif olamaz.",
        ));
    } else {
        checks.push(ok("page_margins", "Sayfa kenar boşlukları geçerli."));
    }
    let usable = crate::A4_WIDTH_PT - p.left_margin - p.right_margin;
    if usable <= 0.0 {
        checks.push(err(
            "page_usable_width",
            "Kenar boşlukları kullanılabilir genişliği sıfırlıyor.",
        ));
    } else {
        checks.push(ok("page_usable_width", "Kullanılabilir genişlik yeterli."));
    }
    if p.paper_orientation > 1 {
        checks.push(warn(
            "page_orientation",
            "Sayfa yönü değeri beklenen aralıkta değil (0 veya 1).",
        ));
    }

    // 2. Blokları gez: tablolar ve görseller
    let mut block_count = 0;
    for block in doc
        .body
        .iter()
        .chain(doc.header.iter())
        .chain(doc.footer.iter())
    {
        block_count += 1;
        check_block(block, &mut checks);
    }
    if block_count == 0 {
        checks.push(warn("empty_document", "Belgede hiç içerik bloğu yok."));
    }

    // 3. Metin ve Türkçe karakter
    let text = doc.plain_text();
    let tr = count_turkish_characters(&text);
    checks.push(ok(
        "utf8_text",
        format!(
            "Metin UTF-8 olarak okundu ({} karakter).",
            text.chars().count()
        ),
    ));
    if tr > 0 {
        checks.push(ok(
            "turkish_characters",
            format!("{tr} Türkçe'ye özgü karakter korundu."),
        ));
    }

    // 4. Stiller
    if doc.styles.is_empty() {
        checks.push(warn(
            "styles",
            "Belgede stil tanımı yok; UYAP Editör varsayılan stili kullanabilir.",
        ));
    } else {
        checks.push(ok("styles", "Stil tanımları mevcut."));
    }

    finalize(checks, text.chars().count(), 0, tr > 0)
}

fn check_block(block: &UdfBlock, checks: &mut Vec<CheckResult>) {
    match block {
        UdfBlock::Paragraph(p) => {
            for run in &p.runs {
                if let Run::Image {
                    data_base64,
                    width,
                    height,
                } = run
                {
                    check_image(data_base64, *width, *height, checks);
                }
                if let Run::Text { text, style } = run {
                    if style.size <= 0.0 || style.size > 200.0 {
                        checks.push(warn(
                            "font_size",
                            format!("Olağan dışı punto: {}", style.size),
                        ));
                    }
                    if text.contains(crate::IMAGE_PLACEHOLDER) {
                        checks.push(warn(
                            "placeholder_in_text",
                            "Metin içinde resim yer tutucusu bulundu.".to_string(),
                        ));
                    }
                }
            }
        }
        UdfBlock::Table(t) => {
            if t.column_spans.len() != t.column_count {
                checks.push(err(
                    "table_columns",
                    format!(
                        "Tablo sütun sayısı ({}) ile sütun genişliği sayısı ({}) uyuşmuyor.",
                        t.column_count,
                        t.column_spans.len()
                    ),
                ));
            }
            let total: f64 = t.column_spans.iter().sum();
            if total <= 0.0 {
                checks.push(err(
                    "table_width",
                    "Tablo sütun genişlikleri toplamı sıfır.".to_string(),
                ));
            }
            for (ri, row) in t.rows.iter().enumerate() {
                let spanned: usize = row.cells.iter().map(|c| c.colspan as usize).sum();
                if spanned != t.column_count {
                    checks.push(warn(
                        "table_row_cells",
                        format!(
                            "{}. satırdaki hücre kapsamı ({spanned}) sütun sayısıyla ({}) uyuşmuyor.",
                            ri + 1,
                            t.column_count
                        ),
                    ));
                }
                for cell in &row.cells {
                    if cell.blocks.is_empty() {
                        checks.push(warn(
                            "empty_cell",
                            format!("{}. satırda boş hücre var.", ri + 1),
                        ));
                    }
                    for b in &cell.blocks {
                        check_block(b, checks);
                    }
                }
            }
        }
        UdfBlock::PageBreak => {}
    }
}

fn check_image(data: &str, width: f64, height: f64, checks: &mut Vec<CheckResult>) {
    if data.is_empty() {
        checks.push(err("image_data", "Resim verisi boş.".to_string()));
        return;
    }
    use base64::Engine;
    match base64::engine::general_purpose::STANDARD.decode(data) {
        Ok(bytes) => {
            if bytes.is_empty() {
                checks.push(err("image_data", "Resim verisi çözülemedi.".to_string()));
            }
        }
        Err(_) => checks.push(err(
            "image_base64",
            "Resim verisi geçerli base64 değil.".to_string(),
        )),
    }
    if width <= 0.0 || height <= 0.0 {
        checks.push(err(
            "image_size",
            format!("Resim boyutu geçersiz: {width} x {height}."),
        ));
    }
    if width > crate::A4_WIDTH_PT {
        checks.push(warn(
            "image_overflow",
            format!("Resim genişliği sayfa genişliğini aşıyor ({width} pt)."),
        ));
    }
}

/// Paketlenmiş UDF baytlarını doğrular.
///
/// Paket bütünlüğü, `content.xml` varlığı, XML çözümlenebilirliği ve
/// yeniden açılabilirlik kontrol edilir.
pub fn validate_udf(bytes: &[u8]) -> UdfValidation {
    let mut checks = Vec::new();

    // 1. ZIP bütünlüğü
    let mut archive = match zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())) {
        Ok(a) => {
            checks.push(ok("zip_integrity", "Paket bütünlüğü doğrulandı."));
            a
        }
        Err(e) => {
            checks.push(err("zip_integrity", format!("Paket açılamadı: {e}")));
            return finalize(checks, 0, 0, false);
        }
    };

    // 2. content.xml var mı
    if archive.by_name(crate::CONTENT_XML).is_err() {
        checks.push(err(
            "content_xml",
            "Paket içinde content.xml bulunamadı.".to_string(),
        ));
        return finalize(checks, 0, 0, false);
    }
    checks.push(ok("content_xml", "content.xml pakette mevcut."));

    // 3. Çözümleme
    let read: UdfReadResult = match reader::read_udf_from_bytes(bytes) {
        Ok(r) => r,
        Err(e) => {
            checks.push(err("xml_parse", format!("content.xml çözümlenemedi: {e}")));
            return finalize(checks, 0, 0, false);
        }
    };
    checks.push(ok("xml_parse", "content.xml çözümlendi."));

    // 4. format_id
    match read.format_id.as_deref() {
        Some("1.8") => checks.push(ok("format_id", "format_id=\"1.8\" doğrulandı.")),
        Some(other) => checks.push(warn(
            "format_id",
            format!("format_id beklenen değerde değil: {other}"),
        )),
        None => checks.push(warn("format_id", "format_id bulunamadı.".to_string())),
    }

    // 5. Offset tutarlılığı — XML'i yeniden tarayarak en büyük offset'i bul.
    let xml = read_raw_content_xml(bytes).unwrap_or_default();
    let (max_ref, bad_refs) = scan_offsets(&xml, read.pool.chars().count());
    if bad_refs > 0 {
        checks.push(err(
            "offset_bounds",
            format!("{bad_refs} metin referansı havuz sınırlarını aşıyor."),
        ));
    } else {
        checks.push(ok(
            "offset_bounds",
            format!("Tüm metin referansları havuz içinde (en büyük offset: {max_ref})."),
        ));
    }

    // 6. Metin havuzu boş değil
    if read.pool.is_empty() {
        checks.push(warn("pool_empty", "Metin havuzu boş.".to_string()));
    }

    // 7. Türkçe karakter
    let tr = count_turkish_characters(&read.pool);
    if tr > 0 {
        checks.push(ok(
            "turkish_characters",
            format!("{tr} Türkçe'ye özgü karakter korundu."),
        ));
    }

    // 8. Belge modeli doğrulaması
    let doc_validation = validate_document(&read.document);
    checks.extend(doc_validation.checks);

    finalize(checks, read.pool.chars().count(), max_ref, tr > 0)
}

fn read_raw_content_xml(bytes: &[u8]) -> Option<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).ok()?;
    let mut f = archive.by_name(crate::CONTENT_XML).ok()?;
    let mut s = String::new();
    std::io::Read::read_to_string(&mut f, &mut s).ok()?;
    Some(s)
}

/// `content.xml` metnindeki `startOffset`/`length` çiftlerini tarar.
///
/// Döndürür: (en büyük offset+length, sınırı aşan referans sayısı).
fn scan_offsets(xml: &str, pool_len: usize) -> (usize, usize) {
    let mut max_ref = 0usize;
    let mut bad = 0usize;

    for tag in ["<content ", "<image ", "<tab ", "<space "] {
        let mut search_from = 0;
        while let Some(pos) = xml[search_from..].find(tag) {
            let start_idx = search_from + pos;
            let end_idx = match xml[start_idx..].find('>') {
                Some(e) => start_idx + e,
                None => break,
            };
            let elem = &xml[start_idx..end_idx];
            let start = extract_attr(elem, "startOffset");
            let len = extract_attr(elem, "length");
            if let Some(s) = start {
                let l = len.unwrap_or(0);
                let end = s + l;
                if end > max_ref {
                    max_ref = end;
                }
                if end > pool_len {
                    bad += 1;
                }
            }
            search_from = end_idx + 1;
        }
    }
    (max_ref, bad)
}

fn extract_attr(elem: &str, name: &str) -> Option<usize> {
    let key = format!("{name}=\"");
    let start = elem.find(&key)? + key.len();
    let rest = &elem[start..];
    let end = rest.find('"')?;
    rest[..end].parse::<usize>().ok()
}

fn ok(name: &str, msg: impl Into<String>) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        severity: CheckSeverity::Info,
        message: msg.into(),
    }
}

fn warn(name: &str, msg: impl Into<String>) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        severity: CheckSeverity::Warning,
        message: msg.into(),
    }
}

fn err(name: &str, msg: impl Into<String>) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        severity: CheckSeverity::Error,
        message: msg.into(),
    }
}

fn finalize(
    checks: Vec<CheckResult>,
    pool_chars: usize,
    max_ref: usize,
    has_turkish: bool,
) -> UdfValidation {
    let error_count = checks
        .iter()
        .filter(|c| c.severity == CheckSeverity::Error)
        .count();
    let warning_count = checks
        .iter()
        .filter(|c| c.severity == CheckSeverity::Warning)
        .count();
    UdfValidation {
        ok: error_count == 0,
        checks,
        error_count,
        warning_count,
        pool_chars,
        max_referenced_offset: max_ref,
        has_turkish_characters: has_turkish,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc() -> UdfDocument {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::styled(
            "BİLİRKİŞİ RAPORU",
            TextStyle::bold(),
        )));
        doc.push(UdfBlock::Paragraph(Paragraph::text(
            "İnceleme konusu: ses kaydının metne dönüştürülmesi (ğüşiöç).",
        )));
        let mut t = Table::new(vec![150.0, 300.0]);
        t.push_row(Row::header(&["Soru".into(), "Yanıt".into()]));
        t.push_row(Row::from_texts(&["1".into(), "Çözümleme yapıldı.".into()]));
        doc.push(UdfBlock::Table(t));
        doc
    }

    #[test]
    fn valid_document_passes() {
        let v = validate_document(&sample_doc());
        assert!(v.ok, "{}", v.summary());
        assert!(v.has_turkish_characters);
    }

    #[test]
    fn packed_udf_passes_validation() {
        let bytes = crate::write_udf_to_bytes(sample_doc()).unwrap();
        let v = validate_udf(&bytes);
        assert!(v.ok, "hatalar: {:?}", v.errors());
        assert_eq!(v.error_count, 0);
    }

    #[test]
    fn invalid_zip_fails_validation() {
        let v = validate_udf(b"bu bir zip degil");
        assert!(!v.ok);
        assert!(v.error_count > 0);
    }

    #[test]
    fn table_column_mismatch_is_an_error() {
        let mut t = Table::new(vec![100.0, 100.0]);
        t.column_count = 3; // tutarsızlık
        t.push_row(Row::from_texts(&["a".into(), "b".into()]));
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Table(t));
        let v = validate_document(&doc);
        assert!(!v.ok);
        assert!(v.errors().iter().any(|e| e.name == "table_columns"));
    }

    #[test]
    fn invalid_base64_image_is_an_error() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph {
            runs: vec![Run::Image {
                data_base64: "!!!gecersiz!!!".into(),
                width: 100.0,
                height: 100.0,
            }],
            ..Default::default()
        }));
        let v = validate_document(&doc);
        assert!(!v.ok);
        assert!(v.errors().iter().any(|e| e.name == "image_base64"));
    }

    #[test]
    fn negative_margins_are_an_error() {
        let mut doc = UdfDocument::new();
        doc.page.left_margin = -5.0;
        let v = validate_document(&doc);
        assert!(!v.ok);
    }

    #[test]
    fn turkish_character_counting_works() {
        assert_eq!(count_turkish_characters("ğüşiöçĞÜŞİÖÇ"), 11);
        assert_eq!(count_turkish_characters("abc123"), 0);
    }

    #[test]
    fn offset_scanning_finds_max_and_bad() {
        let xml = r#"<content startOffset="0" length="5" /><content startOffset="10" length="3" /><content startOffset="50" length="5" />"#;
        let (max, bad) = scan_offsets(xml, 20);
        assert_eq!(max, 55);
        assert_eq!(bad, 1);
    }

    #[test]
    fn tampered_pool_that_overruns_is_detected() {
        // Geçerli belge üret, sonra havuzu kısalt ve paketi yeniden kur.
        let bytes = crate::write_udf_to_bytes(sample_doc()).unwrap();
        let mut xml = read_raw_content_xml(&bytes).unwrap();
        // CDATA içeriğini kısalt
        if let Some(start) = xml.find("CDATA[") {
            let content_start = start + "CDATA[".len();
            if let Some(end_rel) = xml[content_start..].find("]]>") {
                let end = content_start + end_rel;
                xml.replace_range(content_start..end, "kisa");
            }
        }
        let tampered = crate::writer::pack_udf(&xml).unwrap();
        let v = validate_udf(&tampered);
        assert!(!v.ok, "bozuk offset yakalanmalıydı: {}", v.summary());
    }

    #[test]
    fn empty_document_produces_warning() {
        let doc = UdfDocument::new();
        let v = validate_document(&doc);
        assert!(v.warnings().iter().any(|w| w.name == "empty_document"));
    }
}
