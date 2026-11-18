//! Kanonik rapor modelinden UDF belgesine köprü (14, 15).
//!
//! Bu modül, [`ReportDocument`] verisini [`UdfDocument`] modeline çevirir.
//! UDF katmanı rapor kavramlarını bilmez; rapor katmanı UDF ayrıntılarını
//! bilmez. Böylece UDF, PDF ve DOCX çıktıları aynı veriden üretilir.

use birkat_core::report::{Align, Block, NumberType, ReportDocument, SectionId};
use birkat_udf::model::*;

/// Dönüştürme seçenekleri.
#[derive(Debug, Clone)]
pub struct ReportToUdfOptions {
    /// İçindekiler bölümü eklensin mi?
    pub include_toc: bool,
    /// Sayfa numarası altbilgisi eklensin mi?
    pub footer_page_number: bool,
    /// Üstbilgi metni.
    pub header: Option<String>,
    /// Gizli bölümler atlanmış olsa bile zorunlu bölümler yazılsın mı?
    pub force_mandatory_sections: bool,
}

impl Default for ReportToUdfOptions {
    fn default() -> Self {
        Self {
            include_toc: true,
            footer_page_number: true,
            header: None,
            force_mandatory_sections: true,
        }
    }
}

/// Rapor belgesini UDF belgesine çevirir.
pub fn report_to_udf(report: &ReportDocument, options: &ReportToUdfOptions) -> UdfDocument {
    let mut doc = UdfDocument::new();

    // Sayfa yapısı
    doc.page.media_size_name = report.page_setup.media_size_name.clone();
    doc.page.left_margin = report.page_setup.left_margin;
    doc.page.right_margin = report.page_setup.right_margin;
    doc.page.top_margin = report.page_setup.top_margin;
    doc.page.bottom_margin = report.page_setup.bottom_margin;
    doc.page.paper_orientation = report.page_setup.orientation;

    // Üstbilgi
    if let Some(h) = options
        .header
        .as_ref()
        .or(report.page_setup.header.as_ref())
    {
        doc.header.push(UdfBlock::Paragraph(Paragraph::styled(
            h,
            TextStyle::sized(9.0),
        )));
    }

    // Altbilgi sayfa numarası
    if options.footer_page_number && report.page_setup.footer_page_number {
        doc.footer_page_number = Some(PageNumberStyle::default());
    }

    // İçindekiler
    if options.include_toc {
        push_heading(&mut doc, "İçindekiler", 1);
        for section in &report.sections {
            if section.hidden && !(options.force_mandatory_sections && section.mandatory) {
                continue;
            }
            let mut p = Paragraph::text(section.title.clone());
            p.left_indent = 12.0;
            doc.push(UdfBlock::Paragraph(p));
        }
        doc.push(UdfBlock::PageBreak);
    }

    // Bölümler
    for section in &report.sections {
        if section.hidden && !(options.force_mandatory_sections && section.mandatory) {
            continue;
        }
        push_heading(&mut doc, &section.title, 1);
        for block in &section.blocks {
            push_block(&mut doc, block);
        }
    }

    // Ekler
    if !report.attachments.is_empty() {
        push_heading(&mut doc, "Ekler listesi", 1);
        let mut table = Table::new(vec![40.0, 260.0, 200.0]);
        table.push_row(Row::header(&[
            "No".to_string(),
            "Ek".to_string(),
            "SHA-256".to_string(),
        ]));
        for a in &report.attachments {
            table.push_row(Row::from_texts(&[
                a.ordinal.to_string(),
                a.title.clone(),
                a.sha256.clone().unwrap_or_else(|| "-".to_string()),
            ]));
        }
        doc.push(UdfBlock::Table(table));
    }

    doc
}

fn push_heading(doc: &mut UdfDocument, text: &str, level: u8) {
    let size = match level {
        1 => 14.0,
        2 => 12.5,
        _ => 11.5,
    };
    let mut p = Paragraph::styled(text, TextStyle::bold().with_size(size));
    p.space_above = Some(10.0);
    p.space_below = Some(4.0);
    doc.push(UdfBlock::Paragraph(p));
}

fn push_block(doc: &mut UdfDocument, block: &Block) {
    match block {
        Block::Paragraph { text, align } => {
            let mut p = Paragraph::styled(text.clone(), TextStyle::default());
            p.alignment = align_value(*align);
            doc.push(UdfBlock::Paragraph(p));
        }
        Block::Heading { text, level } => push_heading(doc, text, *level),
        Block::BulletList { items } => {
            for (i, item) in items.iter().enumerate() {
                let mut p = Paragraph::text(item.clone());
                p.bulleted = true;
                p.bullet_type = Some("BULLET_TYPE_ELLIPSE".to_string());
                p.list_id = Some("1".to_string());
                p.list_level = Some(0);
                p.left_indent = 18.0;
                let _ = i;
                doc.push(UdfBlock::Paragraph(p));
            }
        }
        Block::NumberedList { items, number_type } => {
            for item in items {
                let mut p = Paragraph::text(item.clone());
                p.numbered = true;
                p.number_type = Some(number_type_udf(*number_type).to_string());
                p.list_id = Some("2".to_string());
                p.list_level = Some(0);
                p.left_indent = 18.0;
                doc.push(UdfBlock::Paragraph(p));
            }
        }
        Block::Table {
            header,
            rows,
            column_widths,
        } => {
            let widths = if column_widths.is_empty() {
                // Sayfa genişliğini eşit böl.
                let n = header.len().max(1);
                let usable = crate::A4_USABLE_WIDTH_PT;
                vec![usable / n as f64; n]
            } else {
                column_widths.clone()
            };
            let mut table = Table::new(widths);
            if !header.is_empty() {
                table.push_row(Row::header(header));
            }
            for r in rows {
                table.push_row(Row::from_texts(r));
            }
            doc.push(UdfBlock::Table(table));
        }
        Block::Image {
            data_base64,
            width,
            height,
            caption,
        } => {
            let mut p = Paragraph {
                runs: vec![Run::Image {
                    data_base64: data_base64.clone(),
                    width: *width,
                    height: *height,
                }],
                alignment: 1,
                ..Default::default()
            };
            p.left_indent = 0.0;
            doc.push(UdfBlock::Paragraph(p));
            if let Some(c) = caption {
                let mut cp = Paragraph::styled(c.clone(), TextStyle::sized(9.0));
                cp.alignment = 1;
                doc.push(UdfBlock::Paragraph(cp));
            }
        }
        Block::PageBreak => doc.push(UdfBlock::PageBreak),
        Block::TimeReference {
            label,
            start,
            end,
            quote,
        } => {
            let mut p = Paragraph::text(format!("{label} [{:.3} - {:.3}]", start, end));
            p.left_indent = 18.0;
            doc.push(UdfBlock::Paragraph(p));
            if let Some(q) = quote {
                let mut qp =
                    Paragraph::styled(format!("\u{201C}{q}\u{201D}"), TextStyle::default());
                qp.left_indent = 32.0;
                qp.alignment = 3;
                doc.push(UdfBlock::Paragraph(qp));
            }
        }
        Block::TranscriptSegment {
            time_code,
            speaker,
            text,
        } => {
            let prefix = match speaker {
                Some(s) => format!("{time_code}  {s}: "),
                None => format!("{time_code}  "),
            };
            let mut p = Paragraph {
                runs: vec![
                    Run::Text {
                        text: prefix,
                        style: TextStyle::bold().with_size(10.0),
                    },
                    Run::Text {
                        text: text.clone(),
                        style: TextStyle::sized(10.0),
                    },
                ],
                alignment: 3,
                ..Default::default()
            };
            p.space_below = Some(2.0);
            doc.push(UdfBlock::Paragraph(p));
        }
        Block::Notice { title, text } => {
            let mut p = Paragraph::styled(title.clone(), TextStyle::bold());
            p.space_above = Some(6.0);
            doc.push(UdfBlock::Paragraph(p));
            doc.push(UdfBlock::Paragraph(Paragraph::text(text.clone())));
        }
    }
}

/// Hizalama değerini UDF sayısal değerine çevirir.
pub fn align_value(align: Align) -> u8 {
    align.udf_value().parse().unwrap_or(0)
}

/// Numara türünü UDF öznitelik değerine çevirir.
pub fn number_type_udf(t: NumberType) -> &'static str {
    t.udf_value()
}

/// Bölüm kimliğinin rapor içinde yer alıp almadığını denetler.
pub fn section_present(report: &ReportDocument, id: SectionId) -> bool {
    report.sections.iter().any(|s| s.id == id && !s.hidden)
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::ids::CaseId;
    use birkat_core::report::ReportKind;

    fn sample_report() -> ReportDocument {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.push_block(
            SectionId::Subject,
            Block::Paragraph {
                text: "İnceleme konusu: ses kaydının metne dönüştürülmesi.".to_string(),
                align: Align::Justify,
            },
        );
        r.push_block(
            SectionId::Questions,
            Block::NumberedList {
                items: vec![
                    "Kayıttaki konuşmalar çözülsün.".to_string(),
                    "Konuşmacılar ayrılsın.".to_string(),
                ],
                number_type: NumberType::NumberDot,
            },
        );
        r.push_block(
            SectionId::AudioInventory,
            Block::Table {
                header: vec!["Dosya".to_string(), "Codec".to_string()],
                rows: vec![vec!["kayit1.wav".to_string(), "PCM".to_string()]],
                column_widths: vec![250.0, 150.0],
            },
        );
        r.push_block(
            SectionId::Transcripts,
            Block::TranscriptSegment {
                time_code: "00:00.000".to_string(),
                speaker: Some("Konuşmacı 1".to_string()),
                text: "Merhaba, bu bir denemedir.".to_string(),
            },
        );
        r
    }

    #[test]
    fn report_converts_to_udf_with_sections() {
        let udf = report_to_udf(&sample_report(), &ReportToUdfOptions::default());
        let text = udf.plain_text();
        assert!(text.contains("İnceleme konusu"));
        assert!(text.contains("Kayıttaki konuşmalar çözülsün"));
        assert!(text.contains("kayit1.wav"));
        assert!(text.contains("Konuşmacı 1"));
    }

    #[test]
    fn udf_has_toc_when_requested() {
        let udf = report_to_udf(&sample_report(), &ReportToUdfOptions::default());
        assert!(udf.plain_text().contains("İçindekiler"));
    }

    #[test]
    fn udf_has_no_toc_when_disabled() {
        let opts = ReportToUdfOptions {
            include_toc: false,
            ..Default::default()
        };
        let udf = report_to_udf(&sample_report(), &opts);
        assert!(!udf.plain_text().contains("İçindekiler"));
    }

    #[test]
    fn table_becomes_udf_table() {
        let udf = report_to_udf(&sample_report(), &ReportToUdfOptions::default());
        let has_table = udf.body.iter().any(|b| matches!(b, UdfBlock::Table(_)));
        assert!(has_table);
    }

    #[test]
    fn numbered_list_becomes_udf_numbered_paragraphs() {
        let udf = report_to_udf(&sample_report(), &ReportToUdfOptions::default());
        let numbered: Vec<&Paragraph> = udf
            .body
            .iter()
            .filter_map(|b| match b {
                UdfBlock::Paragraph(p) if p.numbered => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(numbered.len(), 2);
        assert_eq!(
            numbered[0].number_type.as_deref(),
            Some("NUMBER_TYPE_NUMBER_DOT")
        );
    }

    #[test]
    fn footer_page_number_can_be_disabled() {
        let opts = ReportToUdfOptions {
            footer_page_number: false,
            ..Default::default()
        };
        let udf = report_to_udf(&sample_report(), &opts);
        assert!(udf.footer_page_number.is_none());
    }

    #[test]
    fn generated_udf_passes_validation() {
        let udf = report_to_udf(&sample_report(), &ReportToUdfOptions::default());
        let bytes = birkat_udf::write_udf_to_bytes(udf).unwrap();
        let v = birkat_udf::validate_udf(&bytes);
        assert!(v.ok, "hatalar: {:?}", v.errors());
        assert!(v.has_turkish_characters);
    }

    #[test]
    fn generated_udf_round_trips_through_reader() {
        let udf = report_to_udf(&sample_report(), &ReportToUdfOptions::default());
        let bytes = birkat_udf::write_udf_to_bytes(udf).unwrap();
        let r = birkat_udf::read_udf_from_bytes(&bytes).unwrap();
        assert!(r.pool.contains("Merhaba, bu bir denemedir."));
        assert!(r.pool.contains("ğüşiöç".chars().next().unwrap()) || r.pool.contains("İnceleme"));
    }

    #[test]
    fn all_standard_sections_are_emitted() {
        let udf = report_to_udf(&sample_report(), &ReportToUdfOptions::default());
        let text = udf.plain_text();
        for id in SectionId::standard_order() {
            assert!(
                text.contains(id.default_title()),
                "bölüm başlığı eksik: {}",
                id.default_title()
            );
        }
    }

    #[test]
    fn align_mapping_is_correct() {
        assert_eq!(align_value(Align::Left), 0);
        assert_eq!(align_value(Align::Center), 1);
        assert_eq!(align_value(Align::Right), 2);
        assert_eq!(align_value(Align::Justify), 3);
    }

    #[test]
    fn image_block_produces_image_run() {
        let mut r = sample_report();
        r.push_block(
            SectionId::MaterialsExamined,
            Block::Image {
                data_base64: "QUJD".to_string(),
                width: 200.0,
                height: 100.0,
                caption: Some("Şekil 1".to_string()),
            },
        );
        let udf = report_to_udf(&r, &ReportToUdfOptions::default());
        let has_image = udf.body.iter().any(|b| match b {
            UdfBlock::Paragraph(p) => p.runs.iter().any(|r| matches!(r, Run::Image { .. })),
            _ => false,
        });
        assert!(has_image);
    }

    #[test]
    fn notice_block_produces_heading_and_text() {
        let mut r = sample_report();
        r.push_block(
            SectionId::Limitations,
            Block::Notice {
                title: "Yöntem sınırlılığı".to_string(),
                text: "Bu yöntem sınırlıdır.".to_string(),
            },
        );
        let udf = report_to_udf(&r, &ReportToUdfOptions::default());
        let text = udf.plain_text();
        assert!(text.contains("Yöntem sınırlılığı"));
        assert!(text.contains("Bu yöntem sınırlıdır."));
    }
}
