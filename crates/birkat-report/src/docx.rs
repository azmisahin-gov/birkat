//! DOCX yardımcı çıktısı (15.6).
//!
//! DOCX, UDF'nin yerine geçmeyen bir yardımcı çıktıdır. Kanonik rapor
//! modelinden OPC (Office Open XML) paketi üretilir. Türkçe karakterler
//! UTF-8 olarak korunur.

use birkat_core::report::{Block, ReportDocument};
use birkat_udf::writer::xml_attr;
use std::io::Write as _;

/// DOCX üretim hatası.
#[derive(Debug, thiserror::Error)]
pub enum DocxError {
    /// ZIP yazma hatası.
    #[error("DOCX paketleme hatası: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// Girdi/çıktı hatası.
    #[error("girdi/çıktı hatası: {0}")]
    Io(#[from] std::io::Error),
}

/// Raporu DOCX paketi (bayt) olarak üretir.
pub fn report_to_docx(report: &ReportDocument) -> Result<Vec<u8>, DocxError> {
    let document_xml = build_document_xml(report);

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("[Content_Types].xml", opts)?;
        zip.write_all(CONTENT_TYPES.as_bytes())?;

        zip.start_file("_rels/.rels", opts)?;
        zip.write_all(RELS.as_bytes())?;

        zip.start_file("word/_rels/document.xml.rels", opts)?;
        zip.write_all(DOC_RELS.as_bytes())?;

        zip.start_file("word/styles.xml", opts)?;
        zip.write_all(STYLES.as_bytes())?;

        zip.start_file("word/document.xml", opts)?;
        zip.write_all(document_xml.as_bytes())?;

        zip.finish()?;
    }
    Ok(cursor.into_inner())
}

/// Raporu DOCX olarak dosyaya yazar.
pub fn write_docx(report: &ReportDocument, path: impl AsRef<std::path::Path>) -> Result<(), DocxError> {
    let bytes = report_to_docx(report)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn build_document_xml(report: &ReportDocument) -> String {
    let mut body = String::new();

    for section in &report.sections {
        if section.hidden {
            continue;
        }
        body.push_str(&heading(&section.title, 1));
        for block in &section.blocks {
            body.push_str(&block_xml(block));
        }
    }

    if !report.attachments.is_empty() {
        body.push_str(&heading("Ekler listesi", 1));
        let rows: Vec<Vec<String>> = report
            .attachments
            .iter()
            .map(|a| {
                vec![
                    a.ordinal.to_string(),
                    a.title.clone(),
                    a.sha256.clone().unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect();
        body.push_str(&table_xml(
            &["No".to_string(), "Ek".to_string(), "SHA-256".to_string()],
            &rows,
        ));
    }

    // Son bölüm (sectPr) sayfa yapısı.
    let sect = format!(
        "<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\" /><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" /></w:sectPr>",
        (birkat_udf::A4_WIDTH_PT * 20.0) as i64,
        (birkat_udf::A4_HEIGHT_PT * 20.0) as i64,
        (report.page_setup.top_margin * 20.0) as i64,
        (report.page_setup.right_margin * 20.0) as i64,
        (report.page_setup.bottom_margin * 20.0) as i64,
        (report.page_setup.left_margin * 20.0) as i64,
    );

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:body>{body}{sect}</w:body></w:document>"
    )
}

fn block_xml(block: &Block) -> String {
    match block {
        Block::Paragraph { text, align } => paragraph(text, None, Some(align_w(*align))),
        Block::Heading { text, level } => heading(text, *level),
        Block::BulletList { items } => items
            .iter()
            .map(|i| paragraph(&format!("\u{2022}  {i}"), None, None))
            .collect(),
        Block::NumberedList { items, .. } => items
            .iter()
            .enumerate()
            .map(|(n, i)| paragraph(&format!("{}.  {i}", n + 1), None, None))
            .collect(),
        Block::Table {
            header,
            rows,
            column_widths,
        } => {
            let widths = if column_widths.is_empty() {
                vec![0.0; header.len().max(1)]
            } else {
                column_widths.clone()
            };
            let mut all = Vec::new();
            if !header.is_empty() {
                all.push(header.clone());
            }
            all.extend(rows.clone());
            table_xml_widths(header, rows, &widths)
        }
        Block::Image { caption, .. } => {
            // Görseller DOCX yardımcı çıktısında metin olarak bildirilir;
            // birincil çıktı UDF'dir ve görseller orada tam olarak yer alır.
            let mut s = paragraph("[Görsel: birincil UDF raporunda yer alır]", None, None);
            if let Some(c) = caption {
                s.push_str(&paragraph(&format!("Alt yazı: {c}"), None, None));
            }
            s
        }
        Block::PageBreak => "<w:p><w:r><w:br w:type=\"page\" /></w:r></w:p>".to_string(),
        Block::TimeReference {
            label,
            start,
            end,
            quote,
        } => {
            let mut s = paragraph(&format!("{label} [{start:.3} - {end:.3}]"), None, None);
            if let Some(q) = quote {
                s.push_str(&paragraph(&format!("\u{201C}{q}\u{201D}"), None, None));
            }
            s
        }
        Block::TranscriptSegment {
            time_code,
            speaker,
            text,
        } => paragraph(
            &format!(
                "{time_code}  {}: {text}",
                speaker.as_deref().unwrap_or("-")
            ),
            None,
            None,
        ),
        Block::Notice { title, text } => {
            let mut s = paragraph(title, Some(true), None);
            s.push_str(&paragraph(text, None, None));
            s
        }
    }
}

fn align_w(align: birkat_core::report::Align) -> &'static str {
    use birkat_core::report::Align::*;
    match align {
        Left => "left",
        Center => "center",
        Right => "right",
        Justify => "both",
    }
}

fn paragraph(text: &str, bold: Option<bool>, align: Option<&str>) -> String {
    let mut ppr = String::new();
    if let Some(a) = align {
        ppr.push_str(&format!("<w:jc w:val=\"{a}\" />"));
    }
    let mut rpr = String::new();
    if bold == Some(true) {
        rpr.push_str("<w:b />");
    }
    let text = escape(text);
    format!(
        "<w:p>{}{}<w:r>{rpr}<w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p>",
        if ppr.is_empty() {
            String::new()
        } else {
            format!("<w:pPr>{ppr}</w:pPr>")
        },
        ""
    )
}

fn heading(text: &str, level: u8) -> String {
    let size = match level {
        1 => 28,
        2 => 26,
        _ => 24,
    };
    let text = escape(text);
    format!(
        "<w:p><w:pPr><w:spacing w:before=\"200\" w:after=\"100\" /></w:pPr>\
<w:r><w:rPr><w:b /><w:sz w:val=\"{size}\" /></w:rPr>\
<w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p>"
    )
}

fn table_xml(header: &[String], rows: &[Vec<String>]) -> String {
    let widths = vec![0.0; header.len().max(1)];
    table_xml_widths(header, rows, &widths)
}

fn table_xml_widths(header: &[String], rows: &[Vec<String>], widths: &[f64]) -> String {
    let mut s = String::from("<w:tbl><w:tblPr><w:tblBorders>");
    for edge in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        s.push_str(&format!(
            "<w:{edge} w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"000000\" />"
        ));
    }
    s.push_str("</w:tblBorders></w:tblPr>");

    let total: f64 = widths.iter().sum();
    let n = header.len().max(1);
    let usable = birkat_udf::A4_WIDTH_PT - 42.52 - 28.35;

    let mut push_row = |cells: &[String], bold: bool| {
        let mut r = String::from("<w:tr>");
        for (i, c) in cells.iter().enumerate() {
            let width = if total > 0.0 && i < widths.len() {
                (widths[i] / total) * usable
            } else {
                usable / n as f64
            };
            let twips = (width * 20.0) as i64;
            let text = escape(c);
            let b = if bold { "<w:b />" } else { "" };
            r.push_str(&format!(
                "<w:tc><w:tcPr><w:tcW w:w=\"{twips}\" w:type=\"dxa\" /></w:tcPr>\
<w:p><w:r><w:rPr>{b}</w:rPr><w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p></w:tc>"
            ));
        }
        r.push_str("</w:tr>");
        s.push_str(&r);
    };

    if !header.is_empty() {
        push_row(header, true);
    }
    for r in rows {
        push_row(r, false);
    }

    s.push_str("</w:tbl>");
    s
}

fn escape(text: &str) -> String {
    xml_attr(text)
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml" />
<Default Extension="xml" ContentType="application/xml" />
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml" />
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml" />
</Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml" />
</Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml" />
</Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman" /><w:sz w:val="22" /></w:rPr></w:rPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal" /></w:style>
</w:styles>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::ids::CaseId;
    use birkat_core::report::ReportKind;

    fn report() -> ReportDocument {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.push_block(
            birkat_core::report::SectionId::Subject,
            Block::Paragraph {
                text: "İnceleme konusu: ğüşiöç İĞÜŞÖÇ".to_string(),
                align: birkat_core::report::Align::Justify,
            },
        );
        r.push_block(
            birkat_core::report::SectionId::AudioInventory,
            Block::Table {
                header: vec!["Dosya".into(), "Codec".into()],
                rows: vec![vec!["kayit1.wav".into(), "PCM".into()]],
                column_widths: vec![200.0, 100.0],
            },
        );
        r
    }

    #[test]
    fn produces_valid_zip_package() {
        let bytes = report_to_docx(&report()).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&"[Content_Types].xml".to_string()));
        assert!(names.contains(&"_rels/.rels".to_string()));
        assert!(names.contains(&"word/document.xml".to_string()));
        assert!(names.contains(&"word/styles.xml".to_string()));
    }

    #[test]
    fn turkish_characters_are_preserved() {
        let bytes = report_to_docx(&report()).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut doc = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").unwrap(), &mut doc)
            .unwrap();
        assert!(doc.contains("ğüşiöç İĞÜŞÖÇ"));
        assert!(doc.contains("İnceleme konusu"));
    }

    #[test]
    fn tables_are_emitted() {
        let bytes = report_to_docx(&report()).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut doc = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").unwrap(), &mut doc)
            .unwrap();
        assert!(doc.contains("<w:tbl>"));
        assert!(doc.contains("kayit1.wav"));
        assert!(doc.contains("PCM"));
    }

    #[test]
    fn special_characters_are_escaped() {
        let mut r = report();
        r.push_block(
            birkat_core::report::SectionId::Conclusion,
            Block::Paragraph {
                text: "a < b & c > d \"e\"".to_string(),
                align: birkat_core::report::Align::Left,
            },
        );
        let bytes = report_to_docx(&r).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut doc = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").unwrap(), &mut doc)
            .unwrap();
        assert!(doc.contains("a &lt; b &amp; c &gt; d &quot;e&quot;"));
    }

    #[test]
    fn page_setup_is_emitted() {
        let bytes = report_to_docx(&report()).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut doc = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").unwrap(), &mut doc)
            .unwrap();
        assert!(doc.contains("<w:sectPr>"));
        assert!(doc.contains("<w:pgSz"));
    }

    #[test]
    fn writes_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("rapor.docx");
        write_docx(&report(), &p).unwrap();
        assert!(p.exists());
        assert!(std::fs::metadata(&p).unwrap().len() > 1000);
    }

    #[test]
    fn docx_is_zip_detectable() {
        let bytes = report_to_docx(&report()).unwrap();
        assert_eq!(&bytes[..2], b"PK");
    }
}
