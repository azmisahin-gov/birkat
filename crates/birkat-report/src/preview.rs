//! UDF önizleme (15.7).
//!
//! Uygulama kendi ürettiği UDF'nin sayfa görünümünü gösterebilmelidir:
//! sayfa sayfa, yazdırma düzeninde, aranabilir, sayfa taşmalarını
//! gösterebilen ve tablo/görsel sorunlarını işaretleyebilen bir önizleme.
//!
//! **Bu önizleme, gerçek UYAP Editör görünümünü doğrulamanın yerine geçmez**;
//! bir ön kontrol katmanıdır (bkz. [`preview_note`]).

use birkat_udf::model::*;
use serde::{Deserialize, Serialize};

/// Önizleme sayfası.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewPage {
    /// Sayfa numarası (1 tabanlı).
    pub number: usize,
    /// Sayfadaki satırlar (metin ve biçim bilgisi).
    pub lines: Vec<PreviewLine>,
    /// Sayfa uyarıları (taşma, tablo sorunu).
    pub warnings: Vec<String>,
}

/// Önizleme satırı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewLine {
    /// Metin.
    pub text: String,
    /// Kalın mı?
    pub bold: bool,
    /// Punto.
    pub size: f64,
    /// Hizalama.
    pub alignment: u8,
    /// Satır bir başlık mı?
    pub is_heading: bool,
    /// Tablo satırı mı?
    pub is_table_row: bool,
    /// Görsel yer tutucusu mu?
    pub is_image: bool,
}

/// Önizleme sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfPreview {
    /// Sayfalar.
    pub pages: Vec<PreviewPage>,
    /// Sayfa genişliği (pt).
    pub page_width_pt: f64,
    /// Sayfa yüksekliği (pt).
    pub page_height_pt: f64,
    /// Kullanılabilir metin genişliği (pt).
    pub usable_width_pt: f64,
    /// Tüm belge uyarıları.
    pub warnings: Vec<String>,
}

impl UdfPreview {
    /// Belgede aranabilir mi? Metin düzleştirilmiş hâlde tutulur.
    pub fn search(&self, needle: &str) -> Vec<(usize, usize)> {
        let mut hits = Vec::new();
        let lowered = needle.to_lowercase();
        for page in &self.pages {
            for (i, line) in page.lines.iter().enumerate() {
                if line.text.to_lowercase().contains(&lowered) {
                    hits.push((page.number, i));
                }
            }
        }
        hits
    }

    /// Toplam sayfa sayısı.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

/// Yaklaşık satır yüksekliği (punto * satır aralığı).
const LINE_HEIGHT_FACTOR: f64 = 1.35;

/// UDF belgesinden yazdırma düzeninde önizleme üretir.
pub fn build_preview(doc: &UdfDocument) -> UdfPreview {
    let usable_width = birkat_udf::A4_WIDTH_PT - doc.page.left_margin - doc.page.right_margin;
    let usable_height = birkat_udf::A4_HEIGHT_PT - doc.page.top_margin - doc.page.bottom_margin;

    let mut pages: Vec<PreviewPage> = Vec::new();
    let mut current = PreviewPage {
        number: 1,
        lines: Vec::new(),
        warnings: Vec::new(),
    };
    let mut used_height = 0.0f64;
    let mut warnings: Vec<String> = Vec::new();

    let new_page = |pages: &mut Vec<PreviewPage>, current: PreviewPage| {
        pages.push(current);
        PreviewPage {
            number: pages.len() + 1,
            lines: Vec::new(),
            warnings: Vec::new(),
        }
    };

    for block in &doc.body {
        match block {
            UdfBlock::Paragraph(p) => {
                let text = p.plain_text();
                let size = p
                    .runs
                    .iter()
                    .map(|r| match r {
                        Run::Text { style, .. } => style.size,
                        _ => birkat_udf::DEFAULT_FONT_SIZE,
                    })
                    .fold(birkat_udf::DEFAULT_FONT_SIZE, f64::max);
                let bold = p.runs.iter().any(|r| match r {
                    Run::Text { style, .. } => style.bold,
                    _ => false,
                });
                let is_image = p.runs.iter().any(|r| matches!(r, Run::Image { .. }));
                let is_heading = bold && size >= 12.5;

                // Uzun metni satırlara böl.
                let line_chars = (usable_width / (size * 0.5)).max(20.0) as usize;
                let wrapped = wrap_text(&text, line_chars);

                for (li, line) in wrapped.iter().enumerate() {
                    let h = size * LINE_HEIGHT_FACTOR;
                    if used_height + h > usable_height {
                        current = new_page(&mut pages, current);
                        used_height = 0.0;
                    }
                    current.lines.push(PreviewLine {
                        text: line.clone(),
                        bold: bold && li == 0,
                        size,
                        alignment: p.alignment,
                        is_heading: is_heading && li == 0,
                        is_table_row: false,
                        is_image: is_image && li == 0,
                    });
                    used_height += h;
                }
                if wrapped.is_empty() {
                    used_height += size * LINE_HEIGHT_FACTOR;
                }
            }
            UdfBlock::Table(t) => {
                let total_width: f64 = t.column_spans.iter().sum();
                if total_width > usable_width + 1.0 {
                    warnings.push(format!(
                        "Tablo genişliği ({total_width:.1} pt) kullanılabilir alanı \
                         ({usable_width:.1} pt) aşıyor; yazdırmada taşma olabilir."
                    ));
                    current.warnings.push(
                        "Tablo sayfa genişliğini aşıyor; sütun genişliklerini \
                         küçültün."
                            .to_string(),
                    );
                }
                for row in &t.rows {
                    let row_text = row
                        .cells
                        .iter()
                        .map(cell_text)
                        .collect::<Vec<_>>()
                        .join(" | ");
                    let h = row.height.max(12.0);
                    if used_height + h > usable_height {
                        current = new_page(&mut pages, current);
                        used_height = 0.0;
                    }
                    current.lines.push(PreviewLine {
                        text: row_text,
                        bold: row.row_type == "headerRow",
                        size: birkat_udf::DEFAULT_FONT_SIZE,
                        alignment: 0,
                        is_heading: false,
                        is_table_row: true,
                        is_image: false,
                    });
                    used_height += h;
                }
            }
            UdfBlock::PageBreak => {
                current = new_page(&mut pages, current);
                used_height = 0.0;
            }
        }
    }

    pages.push(current);

    // Boş sayfa denetimi.
    for p in &pages {
        if p.lines.is_empty() {
            warnings.push(format!(
                "{} numaralı sayfa boş görünüyor; gereksiz sayfa sonu olabilir.",
                p.number
            ));
        }
    }

    UdfPreview {
        pages,
        page_width_pt: birkat_udf::A4_WIDTH_PT,
        page_height_pt: birkat_udf::A4_HEIGHT_PT,
        usable_width_pt: usable_width,
        warnings,
    }
}

fn cell_text(cell: &Cell) -> String {
    let mut s = String::new();
    for b in &cell.blocks {
        if let UdfBlock::Paragraph(p) = b {
            s.push_str(&p.plain_text());
        }
    }
    s
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        if current.chars().count() + word.chars().count() + 1 > max_chars && !current.is_empty() {
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
    lines
}

/// Önizlemenin sınırını açıklar.
pub fn preview_note() -> &'static str {
    "Bu önizleme, uygulamanın kendi ürettiği UDF'nin ön kontrol katmanıdır. \
     Gerçek UYAP Editör görünümünü doğrulamanın yerine geçmez; nihai uyumluluk \
     kontrolü resmî UYAP Editör ile yapılmalıdır."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_paragraphs(n: usize) -> UdfDocument {
        let mut d = UdfDocument::new();
        for i in 0..n {
            d.push(UdfBlock::Paragraph(Paragraph::text(format!(
                "Paragraf {i}: Bu uzun bir metindir ve sayfaya sığdırılmaya çalışılır."
            ))));
        }
        d
    }

    #[test]
    fn simple_document_fits_on_one_page() {
        let d = doc_with_paragraphs(3);
        let p = build_preview(&d);
        assert_eq!(p.page_count(), 1);
        assert!(p.pages[0].lines.len() >= 3);
    }

    #[test]
    fn long_document_spans_multiple_pages() {
        let d = doc_with_paragraphs(200);
        let p = build_preview(&d);
        assert!(
            p.page_count() > 1,
            "uzun belge birden fazla sayfaya yayılmalı"
        );
    }

    #[test]
    fn page_breaks_create_new_pages() {
        let mut d = UdfDocument::new();
        d.push(UdfBlock::Paragraph(Paragraph::text("Birinci")));
        d.push(UdfBlock::PageBreak);
        d.push(UdfBlock::Paragraph(Paragraph::text("İkinci")));
        let p = build_preview(&d);
        assert_eq!(p.page_count(), 2);
        assert!(p.pages[0].lines[0].text.contains("Birinci"));
        assert!(p.pages[1].lines[0].text.contains("İkinci"));
    }

    #[test]
    fn search_finds_text_across_pages() {
        let mut d = UdfDocument::new();
        d.push(UdfBlock::Paragraph(Paragraph::text("birinci sayfa metni")));
        d.push(UdfBlock::PageBreak);
        d.push(UdfBlock::Paragraph(Paragraph::text("aranan kelime burada")));
        let p = build_preview(&d);
        let hits = p.search("aranan");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 2);
    }

    #[test]
    fn search_is_case_insensitive() {
        let mut d = UdfDocument::new();
        d.push(UdfBlock::Paragraph(Paragraph::text("Türkçe Karakter")));
        let p = build_preview(&d);
        assert_eq!(p.search("türkçe").len(), 1);
    }

    #[test]
    fn table_wider_than_page_produces_warning() {
        let mut d = UdfDocument::new();
        let mut t = Table::new(vec![400.0, 400.0, 400.0]);
        t.push_row(Row::header(&["a".into(), "b".into(), "c".into()]));
        d.push(UdfBlock::Table(t));
        let p = build_preview(&d);
        assert!(!p.warnings.is_empty());
        assert!(p.warnings.iter().any(|w| w.contains("aşıyor")));
    }

    #[test]
    fn table_rows_appear_as_lines() {
        let mut d = UdfDocument::new();
        let mut t = Table::new(vec![150.0, 150.0]);
        t.push_row(Row::header(&["Başlık1".into(), "Başlık2".into()]));
        t.push_row(Row::from_texts(&["a".into(), "b".into()]));
        d.push(UdfBlock::Table(t));
        let p = build_preview(&d);
        let table_lines: Vec<&PreviewLine> =
            p.pages[0].lines.iter().filter(|l| l.is_table_row).collect();
        assert_eq!(table_lines.len(), 2);
        assert!(table_lines[0].bold, "başlık satırı kalın olmalı");
    }

    #[test]
    fn long_text_wraps_into_multiple_lines() {
        let mut d = UdfDocument::new();
        let long = "kelime ".repeat(200);
        d.push(UdfBlock::Paragraph(Paragraph::text(long.trim())));
        let p = build_preview(&d);
        assert!(
            p.pages[0].lines.len() > 10,
            "uzun metin birden fazla satıra yayılmalı, satır sayısı: {}",
            p.pages[0].lines.len()
        );
    }

    #[test]
    fn very_long_text_spans_multiple_pages() {
        let mut d = UdfDocument::new();
        for _ in 0..60 {
            d.push(UdfBlock::Paragraph(Paragraph::text("kelime ".repeat(120))));
        }
        let p = build_preview(&d);
        assert!(
            p.page_count() > 1,
            "çok uzun belge birden fazla sayfaya yayılmalı"
        );
    }

    #[test]
    fn preview_note_disclaims_editor_replacement() {
        let n = preview_note();
        assert!(n.contains("yerine geçmez"));
        assert!(n.contains("UYAP Editör"));
    }

    #[test]
    fn headings_are_marked() {
        let mut d = UdfDocument::new();
        d.push(UdfBlock::Paragraph(Paragraph::styled(
            "BÜYÜK BAŞLIK",
            TextStyle::bold().with_size(14.0),
        )));
        let p = build_preview(&d);
        assert!(p.pages[0].lines[0].is_heading);
    }
}
