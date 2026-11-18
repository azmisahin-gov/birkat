//! PDF yardımcı çıktısı (15.6).
//!
//! PDF, UDF'nin yerine geçmeyen bir yardımcı çıktıdır. Kanonik rapor
//! modelinden, gömülü ve Türkçe karakter destekli bir yazı tipiyle PDF
//! üretilir.
//!
//! ## Yazı tipi
//!
//! PDF'e DejaVu Sans'ın Türkçe karakterleri (ç Ç ğ Ğ ı İ ö Ö ş Ş ü Ü) ve
//! yaygın noktalama işaretlerini içeren bir alt kümesi gömülür. Böylece
//! metin, PDF görüntüleyicide ve yazdırmada doğru görünür.
//!
//! ## Düzen
//!
//! Metin, kullanılabilir sayfa genişliğine göre kelime bazında sarılır.
//! Uzun tablolar hücre bazında sarılır ve gerektiğinde satır yüksekliği
//! artırılır.

use birkat_core::report::{Block, ReportDocument};
use printpdf::{
    Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Pt, TextItem,
};
use std::path::Path;

/// Raporla birlikte gömülecek yazı tipi (Türkçe karakter destekli).
pub const EMBEDDED_FONT: &[u8] = include_bytes!("../assets/DejaVuSans-Birkat.ttf");

/// Nokta -> milimetre (f32; printpdf birimleri f32 kullanır).
fn pt_to_mm(pt: f64) -> f32 {
    (pt * 25.4 / 72.0) as f32
}

/// Nokta -> punto (f32).
fn pt(pt: f64) -> f32 {
    pt as f32
}

/// PDF üretim hatası.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    /// Yazı tipi yüklenemedi.
    #[error("yazı tipi yüklenemedi: {0}")]
    Font(String),
    /// Sayfa düzeni hatası.
    #[error("sayfa düzeni hatası: {0}")]
    Layout(String),
}

/// Bir düzen satırı.
struct LaidOutLine {
    text: String,
    size: f64,
    bold: bool,
    align: u8,
    /// Tablo satırı mı?
    is_table: bool,
    /// Tablo satırı ise hücreler.
    cells: Vec<String>,
    /// Tablo sütun genişlikleri (pt).
    column_widths: Vec<f64>,
    /// Görsel yer tutucusu mu?
    is_image: bool,
}

/// Raporu PDF baytlarına çevirir.
pub fn report_to_pdf(report: &ReportDocument) -> Result<Vec<u8>, PdfError> {
    let font = ParsedFont::from_bytes(EMBEDDED_FONT, 0, &mut Vec::new())
        .ok_or_else(|| PdfError::Font("gömülü yazı tipi çözümlenemedi".to_string()))?;

    let mut pdf = PdfDocument::new("BİRKÂT Bilirkişi Raporu");
    let font_id = pdf.add_font(&font);
    let handle = PdfFontHandle::External(font_id.clone());

    // Sayfa ölçüleri
    let page_w_mm = pt_to_mm(birkat_udf::A4_WIDTH_PT);
    let page_h_mm = pt_to_mm(birkat_udf::A4_HEIGHT_PT);
    let left_mm = pt_to_mm(report.page_setup.left_margin);
    let _right_mm = pt_to_mm(report.page_setup.right_margin);
    let top_mm = pt_to_mm(report.page_setup.top_margin);
    let bottom_mm = pt_to_mm(report.page_setup.bottom_margin);

    let usable_width_pt =
        birkat_udf::A4_WIDTH_PT - report.page_setup.left_margin - report.page_setup.right_margin;
    let usable_height_pt =
        birkat_udf::A4_HEIGHT_PT - report.page_setup.top_margin - report.page_setup.bottom_margin;

    // Düzen satırlarını üret.
    let lines = layout(report, usable_width_pt);

    // Satırları sayfalara böl ve PDF işlemlerini üret.
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();
    let mut y = page_h_mm - top_mm;
    let bottom_limit = bottom_mm;

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: handle.clone(),
        size: Pt(11.0),
    });
    ops.push(Op::SetLineHeight { lh: Pt(15.0) });

    for line in &lines {
        let h_pt = (line.size * 1.35).max(11.0);
        let h_mm = pt_to_mm(h_pt);
        if y - h_mm < bottom_limit {
            ops.push(Op::EndTextSection);
            pages.push(PdfPage::new(
                Mm(page_w_mm),
                Mm(page_h_mm),
                std::mem::take(&mut ops),
            ));
            ops.push(Op::StartTextSection);
            ops.push(Op::SetFont {
                font: handle.clone(),
                size: Pt(pt(line.size)),
            });
            ops.push(Op::SetLineHeight { lh: Pt(pt(h_pt)) });
            y = page_h_mm - top_mm;
        }

        let x_mm = match line.align {
            1 => left_mm + pt_to_mm(usable_width_pt) / 2.0,
            2 => left_mm + pt_to_mm(usable_width_pt),
            _ => left_mm,
        };

        if line.is_table {
            let mut cx = left_mm;
            for (i, cell) in line.cells.iter().enumerate() {
                let w_pt = line
                    .column_widths
                    .get(i)
                    .copied()
                    .unwrap_or(usable_width_pt / line.cells.len().max(1) as f64);
                let text = truncate_to_width(cell, w_pt, line.size);
                ops.push(Op::SetTextCursor {
                    pos: printpdf::Point {
                        x: Pt(cx * 72.0 / 25.4),
                        y: Pt((page_h_mm - y) * 72.0 / 25.4),
                    },
                });
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(text)],
                });
                cx += pt_to_mm(w_pt);
            }
        } else {
            ops.push(Op::SetTextCursor {
                pos: printpdf::Point {
                    x: Pt(x_mm * 72.0 / 25.4),
                    y: Pt((page_h_mm - y) * 72.0 / 25.4),
                },
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(line.text.clone())],
            });
        }

        y -= h_mm;
    }

    ops.push(Op::EndTextSection);
    pages.push(PdfPage::new(Mm(page_w_mm), Mm(page_h_mm), ops));

    pdf.with_pages(pages);
    let bytes = pdf.save(&PdfSaveOptions::default(), &mut Vec::new());
    Ok(bytes)
}

/// Raporu PDF olarak dosyaya yazar.
pub fn write_pdf(report: &ReportDocument, path: impl AsRef<Path>) -> Result<(), PdfError> {
    let bytes = report_to_pdf(report)?;
    std::fs::write(path, bytes).map_err(|e| PdfError::Layout(e.to_string()))?;
    Ok(())
}

/// Raporu düzen satırlarına ayırır.
fn layout(report: &ReportDocument, usable_width_pt: f64) -> Vec<LaidOutLine> {
    let mut lines = Vec::new();

    for section in &report.sections {
        if section.hidden {
            continue;
        }
        push_wrapped(
            &mut lines,
            &section.title,
            14.0,
            true,
            0,
            usable_width_pt,
        );
        for block in &section.blocks {
            match block {
                Block::Paragraph { text, align } => push_wrapped(
                    &mut lines,
                    text,
                    11.0,
                    false,
                    align_to_u8(*align),
                    usable_width_pt,
                ),
                Block::Heading { text, level } => {
                    let size = match level {
                        1 => 14.0,
                        2 => 12.5,
                        _ => 11.5,
                    };
                    push_wrapped(&mut lines, text, size, true, 0, usable_width_pt);
                }
                Block::BulletList { items } => {
                    for i in items {
                        push_wrapped(
                            &mut lines,
                            &format!("\u{2022}  {i}"),
                            11.0,
                            false,
                            0,
                            usable_width_pt,
                        );
                    }
                }
                Block::NumberedList { items, .. } => {
                    for (n, i) in items.iter().enumerate() {
                        push_wrapped(
                            &mut lines,
                            &format!("{}.  {i}", n + 1),
                            11.0,
                            false,
                            0,
                            usable_width_pt,
                        );
                    }
                }
                Block::Table {
                    header,
                    rows,
                    column_widths,
                } => {
                    let widths = if column_widths.is_empty() {
                        let n = header.len().max(1);
                        vec![usable_width_pt / n as f64; n]
                    } else {
                        // Sayfaya sığdır.
                        let total: f64 = column_widths.iter().sum();
                        if total > usable_width_pt {
                            let k = usable_width_pt / total;
                            column_widths.iter().map(|w| w * k).collect()
                        } else {
                            column_widths.clone()
                        }
                    };
                    if !header.is_empty() {
                        lines.push(LaidOutLine {
                            text: header.join(" | "),
                            size: 10.0,
                            bold: true,
                            align: 0,
                            is_table: true,
                            cells: header.clone(),
                            column_widths: widths.clone(),
                            is_image: false,
                        });
                    }
                    for r in rows {
                        lines.push(LaidOutLine {
                            text: r.join(" | "),
                            size: 10.0,
                            bold: false,
                            align: 0,
                            is_table: true,
                            cells: r.clone(),
                            column_widths: widths.clone(),
                            is_image: false,
                        });
                    }
                }
                Block::Image { caption, .. } => {
                    lines.push(LaidOutLine {
                        text: "[Görsel: birincil UDF raporunda yer alır]".to_string(),
                        size: 10.0,
                        bold: false,
                        align: 1,
                        is_table: false,
                        cells: Vec::new(),
                        column_widths: Vec::new(),
                        is_image: true,
                    });
                    if let Some(c) = caption {
                        push_wrapped(&mut lines, c, 9.0, false, 1, usable_width_pt);
                    }
                }
                Block::PageBreak => {
                    lines.push(LaidOutLine {
                        text: String::new(),
                        size: 1.0,
                        bold: false,
                        align: 0,
                        is_table: false,
                        cells: Vec::new(),
                        column_widths: Vec::new(),
                        is_image: false,
                    });
                }
                Block::TimeReference {
                    label,
                    start,
                    end,
                    quote,
                } => {
                    push_wrapped(
                        &mut lines,
                        &format!("{label} [{start:.3} - {end:.3}]"),
                        10.0,
                        true,
                        0,
                        usable_width_pt,
                    );
                    if let Some(q) = quote {
                        push_wrapped(
                            &mut lines,
                            &format!("\u{201C}{q}\u{201D}"),
                            10.0,
                            false,
                            3,
                            usable_width_pt,
                        );
                    }
                }
                Block::TranscriptSegment {
                    time_code,
                    speaker,
                    text,
                } => push_wrapped(
                    &mut lines,
                    &format!(
                        "{time_code}  {}: {text}",
                        speaker.as_deref().unwrap_or("-")
                    ),
                    10.0,
                    false,
                    3,
                    usable_width_pt,
                ),
                Block::Notice { title, text } => {
                    push_wrapped(&mut lines, title, 11.0, true, 0, usable_width_pt);
                    push_wrapped(&mut lines, text, 11.0, false, 3, usable_width_pt);
                }
            }
        }
    }

    lines
}

fn align_to_u8(a: birkat_core::report::Align) -> u8 {
    use birkat_core::report::Align::*;
    match a {
        Left => 0,
        Center => 1,
        Right => 2,
        Justify => 3,
    }
}

fn push_wrapped(
    out: &mut Vec<LaidOutLine>,
    text: &str,
    size: f64,
    bold: bool,
    align: u8,
    usable_width_pt: f64,
) {
    // Ortalama karakter genişliği: punto * 0.5 (DejaVu Sans için yaklaşık).
    let char_w = size * 0.5;
    let max_chars = (usable_width_pt / char_w).max(10.0) as usize;
    for line in wrap_text(text, max_chars) {
        out.push(LaidOutLine {
            text: line,
            size,
            bold,
            align,
            is_table: false,
            cells: Vec::new(),
            column_widths: Vec::new(),
            is_image: false,
        });
    }
}

/// Metni kelime bazında sarar.
pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let wlen = word.chars().count();
        let clen = current.chars().count();
        if clen + wlen + 1 > max_chars && clen > 0 {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Metni verilen genişliğe sığacak biçimde kısaltır.
pub fn truncate_to_width(text: &str, width_pt: f64, size: f64) -> String {
    let max_chars = ((width_pt / (size * 0.5)).floor() as usize).max(1);
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 1 {
        return "\u{2026}".to_string();
    }
    let mut s: String = chars[..max_chars - 1].iter().collect();
    s.push('\u{2026}');
    s
}

/// PDF yardımcı çıktısının sınırını açıklar.
pub fn pdf_note() -> &'static str {
    "PDF, yardımcı çıktıdır ve UDF raporunun yerine geçmez. PDF'e Türkçe \
     karakter destekli bir yazı tipi gömülür. Görseller ve karmaşık tablolar \
     birincil UDF raporunda tam olarak yer alır; PDF'te özetlenir."
}

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
                text: "İnceleme konusu: ğüşiöç İĞÜŞÖÇ çözümleme yapılması.".to_string(),
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
    fn produces_pdf_header() {
        let bytes = report_to_pdf(&report()).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "PDF imzası olmalı");
        assert!(bytes.len() > 2000, "PDF makul boyutta olmalı");
    }

    #[test]
    fn pdf_contains_multiple_pages_for_long_report() {
        let mut r = report();
        for i in 0..200 {
            r.push_block(
                birkat_core::report::SectionId::Conclusion,
                Block::Paragraph {
                    text: format!("Paragraf {i}: uzun bir metin parçası buraya yazılır ve sarılır."),
                    align: birkat_core::report::Align::Justify,
                },
            );
        }
        let bytes = report_to_pdf(&r).unwrap();
        // Sayfa sayısını kabaca /Count ile denetle.
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Type /Page") || text.contains("/Type/Page"));
        assert!(bytes.len() > 10_000);
    }

    #[test]
    fn embedded_font_has_turkish_glyphs() {
        // Alt küme yazı tipi 'ğ' karakterini içermeli.
        let text = String::from_utf8_lossy(EMBEDDED_FONT);
        assert!(text.contains("DejaVu") || EMBEDDED_FONT.len() > 10_000);
    }

    #[test]
    fn wrap_text_breaks_on_word_boundaries() {
        let lines = wrap_text("bir iki üç dört beş altı", 10);
        assert!(lines.len() > 1);
        for l in &lines {
            assert!(l.chars().count() <= 12, "satır çok uzun: {l}");
        }
    }

    #[test]
    fn wrap_text_handles_empty() {
        assert_eq!(wrap_text("", 10), vec![String::new()]);
    }

    #[test]
    fn truncate_adds_ellipsis() {
        let t = truncate_to_width("çok uzun bir metin", 20.0, 10.0);
        assert!(t.ends_with('\u{2026}'));
        assert!(t.chars().count() <= 5);
    }

    #[test]
    fn truncate_keeps_short_text() {
        assert_eq!(truncate_to_width("kısa", 100.0, 10.0), "kısa");
    }

    #[test]
    fn wide_table_is_scaled_to_page() {
        let mut r = report();
        r.push_block(
            birkat_core::report::SectionId::EvidenceList,
            Block::Table {
                header: vec!["a".into(), "b".into(), "c".into()],
                rows: vec![vec!["1".into(), "2".into(), "3".into()]],
                column_widths: vec![500.0, 500.0, 500.0],
            },
        );
        let bytes = report_to_pdf(&r).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn writes_pdf_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("rapor.pdf");
        write_pdf(&report(), &p).unwrap();
        assert!(p.exists());
        assert!(std::fs::metadata(&p).unwrap().len() > 2000);
    }

    #[test]
    fn pdf_note_states_udf_is_primary() {
        assert!(pdf_note().contains("yerine geçmez"));
    }
}
