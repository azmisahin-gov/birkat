//! UDF belge modeli.
//!
//! Bu model, [`crate::writer`] ve [`crate::reader`] arasındaki ortak dildir.
//! `birkat-report` paketi kendi kanonik rapor modelini bu modele çevirir;
//! UDF katmanı rapor kavramlarını bilmez.

use serde::{Deserialize, Serialize};

/// UDF belgesi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfDocument {
    /// Sayfa yapısı.
    pub page: PageFormat,
    /// Arka plan resmi (varsa).
    pub background: Option<BackgroundImage>,
    /// Üstbilgi.
    pub header: Vec<UdfBlock>,
    /// Gövde elemanları.
    pub body: Vec<UdfBlock>,
    /// Altbilgi.
    pub footer: Vec<UdfBlock>,
    /// Altbilgi sayfa numarası özellikleri (varsa).
    pub footer_page_number: Option<PageNumberStyle>,
    /// Stil tanımları.
    pub styles: Vec<StyleDef>,
    /// Varsayılan metin stili.
    pub default_text_style: TextStyle,
}

impl Default for UdfDocument {
    fn default() -> Self {
        Self {
            page: PageFormat::default(),
            background: None,
            header: Vec::new(),
            body: Vec::new(),
            footer: Vec::new(),
            footer_page_number: None,
            styles: vec![
                StyleDef {
                    name: "default".to_string(),
                    description: Some("Geçerli".to_string()),
                    family: "Dialog".to_string(),
                    size: 12.0,
                    bold: false,
                    italic: false,
                    foreground: Some(-13421773),
                    font_attribute_key: Some(
                        "javax.swing.plaf.FontUIResource[family=Dialog,name=Dialog,style=plain,size=12]"
                            .to_string(),
                    ),
                },
                StyleDef {
                    name: "hvl-default".to_string(),
                    description: Some("Gövde".to_string()),
                    family: crate::DEFAULT_FONT.to_string(),
                    size: 12.0,
                    bold: false,
                    italic: false,
                    foreground: None,
                    font_attribute_key: None,
                },
            ],
            default_text_style: TextStyle::default(),
        }
    }
}

impl UdfDocument {
    /// Yeni boş belge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gövdeye blok ekler.
    pub fn push(&mut self, block: UdfBlock) {
        self.body.push(block);
    }

    /// Tüm gövde paragraflarının düz metnini döndürür (okuma doğrulaması için).
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for b in &self.body {
            collect_text(b, &mut out);
            out.push('\n');
        }
        out
    }
}

fn collect_text(block: &UdfBlock, out: &mut String) {
    match block {
        UdfBlock::Paragraph(p) => {
            for run in &p.runs {
                match run {
                    Run::Text { text, .. } => out.push_str(text),
                    Run::Tab => out.push('\t'),
                    Run::Image { .. } => out.push('\u{FFFC}'),
                    Run::Space => out.push(' '),
                }
            }
        }
        UdfBlock::Table(t) => {
            for row in &t.rows {
                for cell in &row.cells {
                    for b in &cell.blocks {
                        collect_text(b, out);
                        out.push(' ');
                    }
                }
            }
        }
        UdfBlock::PageBreak => out.push('\n'),
    }
}

/// Sayfa yapısı (`pageFormat`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageFormat {
    /// Kâğıt boyutu (`mediaSizeName`).
    pub media_size_name: String,
    /// Sol kenar boşluğu (pt).
    pub left_margin: f64,
    /// Sağ kenar boşluğu (pt).
    pub right_margin: f64,
    /// Üst kenar boşluğu (pt).
    pub top_margin: f64,
    /// Alt kenar boşluğu (pt).
    pub bottom_margin: f64,
    /// Yön (0 = yatay, 1 = dikey).
    pub paper_orientation: u8,
    /// Üstbilgi offset (pt).
    pub header_f_offset: f64,
    /// Altbilgi offset (pt).
    pub footer_f_offset: f64,
}

impl Default for PageFormat {
    fn default() -> Self {
        Self {
            media_size_name: "1".to_string(),
            left_margin: 42.52,
            right_margin: 28.35,
            top_margin: 14.17,
            bottom_margin: 14.17,
            paper_orientation: 1,
            header_f_offset: 20.0,
            footer_f_offset: 20.0,
        }
    }
}

/// Arka plan resmi (`bgImage`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundImage {
    /// Kaynak yolu (varsa).
    pub source: String,
    /// Base64 kodlu veri.
    pub data_base64: String,
    /// Alt kenar boşluğu.
    pub bottom_margin: f64,
    /// Üst kenar boşluğu.
    pub up_margin: f64,
    /// Sağ kenar boşluğu (`bgImageRigtMargin` — UYAP uyumluluğu için yazım korunur).
    pub right_margin: f64,
    /// Sol kenar boşluğu.
    pub left_margin: f64,
}

/// Altbilgi sayfa numarası stili.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageNumberStyle {
    /// Biçim belirteci.
    pub spec: String,
    /// Kalın.
    pub bold: bool,
    /// İtalik.
    pub italic: bool,
    /// Yazı tipi.
    pub font_face: String,
    /// Punto.
    pub font_size: f64,
    /// Renk.
    pub color: i32,
}

impl Default for PageNumberStyle {
    fn default() -> Self {
        Self {
            spec: "BSP32_40".to_string(),
            bold: false,
            italic: false,
            font_face: "Arial".to_string(),
            font_size: 11.0,
            color: crate::DEFAULT_FOREGROUND,
        }
    }
}

/// Stil tanımı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDef {
    /// Stil adı.
    pub name: String,
    /// Açıklama.
    pub description: Option<String>,
    /// Yazı tipi ailesi.
    pub family: String,
    /// Punto.
    pub size: f64,
    /// Kalın.
    pub bold: bool,
    /// İtalik.
    pub italic: bool,
    /// Renk.
    pub foreground: Option<i32>,
    /// Swing font anahtarı.
    pub font_attribute_key: Option<String>,
}

/// Metin stili (bir `<content>` parçasına uygulanır).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    /// Yazı tipi ailesi.
    pub family: String,
    /// Punto.
    pub size: f64,
    /// Kalın.
    pub bold: bool,
    /// İtalik.
    pub italic: bool,
    /// Altı çizili.
    pub underline: bool,
    /// Metin rengi.
    pub foreground: i32,
    /// Arka plan rengi.
    pub background: Option<i32>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            family: crate::DEFAULT_FONT.to_string(),
            size: crate::DEFAULT_FONT_SIZE,
            bold: false,
            italic: false,
            underline: false,
            foreground: crate::DEFAULT_FOREGROUND,
            background: None,
        }
    }
}

impl TextStyle {
    /// Kalın stil.
    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Default::default()
        }
    }

    /// Belirli punto ile stil.
    pub fn sized(size: f64) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }
}

/// Gövde bloğu.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type")]
pub enum UdfBlock {
    /// Paragraf.
    Paragraph(Paragraph),
    /// Tablo.
    Table(Table),
    /// Sayfa sonu.
    PageBreak,
}

/// Paragraf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    /// Hizalama (0=sol, 1=orta, 2=sağ, 3=iki yana yasla).
    pub alignment: u8,
    /// Sol girinti (pt).
    pub left_indent: f64,
    /// Sağ girinti (pt).
    pub right_indent: f64,
    /// İlk satır girintisi (pt).
    pub first_line_indent: Option<f64>,
    /// Satır aralığı.
    pub line_spacing: Option<f64>,
    /// Paragraf öncesi boşluk (pt).
    pub space_above: Option<f64>,
    /// Paragraf sonrası boşluk (pt).
    pub space_below: Option<f64>,
    /// Sekme durakları `(pozisyon, hizalama, öncü)`.
    pub tab_set: Vec<(f64, u8, u8)>,
    /// Metin parçaları ve diğer satır içi öğeler.
    pub runs: Vec<Run>,
    /// Madde işaretli liste.
    pub bulleted: bool,
    /// Madde işareti türü.
    pub bullet_type: Option<String>,
    /// Numaralı liste.
    pub numbered: bool,
    /// Numara türü.
    pub number_type: Option<String>,
    /// Liste kimliği.
    pub list_id: Option<String>,
    /// Liste seviyesi.
    pub list_level: Option<u32>,
    /// Stil adı.
    pub style: Option<String>,
}

impl Default for Paragraph {
    fn default() -> Self {
        Self {
            alignment: 0,
            left_indent: 0.0,
            right_indent: 0.0,
            first_line_indent: None,
            line_spacing: None,
            space_above: None,
            space_below: None,
            tab_set: Vec::new(),
            runs: Vec::new(),
            bulleted: false,
            bullet_type: None,
            numbered: false,
            number_type: None,
            list_id: None,
            list_level: None,
            style: None,
        }
    }
}

impl Paragraph {
    /// Düz metin paragrafı.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            runs: vec![Run::text(text)],
            ..Default::default()
        }
    }

    /// Verilen stille düz metin paragrafı.
    pub fn styled(text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            runs: vec![Run::Text {
                text: text.into(),
                style,
            }],
            ..Default::default()
        }
    }

    /// Paragrafın düz metni.
    pub fn plain_text(&self) -> String {
        let mut s = String::new();
        for r in &self.runs {
            match r {
                Run::Text { text, .. } => s.push_str(text),
                Run::Tab => s.push(crate::TAB_CHAR),
                Run::Image { .. } => s.push(crate::IMAGE_PLACEHOLDER),
                Run::Space => s.push(' '),
            }
        }
        if s.is_empty() {
            s.push(crate::EMPTY_PARAGRAPH_PLACEHOLDER);
        }
        s
    }

    /// Paragraf boş mu (yalnızca boş paragraf işaretçisi içeriyor)?
    pub fn is_effectively_empty(&self) -> bool {
        let text = self.plain_text();
        text.chars()
            .all(|c| c == crate::EMPTY_PARAGRAPH_PLACEHOLDER || c == ' ')
    }
}

/// Satır içi öğe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "kind")]
pub enum Run {
    /// Biçimli metin.
    Text {
        /// Metin.
        text: String,
        /// Stil.
        style: TextStyle,
    },
    /// Sekme.
    Tab,
    /// Resim.
    Image {
        /// Base64 kodlu veri.
        data_base64: String,
        /// Genişlik (pt).
        width: f64,
        /// Yükseklik (pt).
        height: f64,
    },
    /// Boşluk.
    Space,
}

impl Run {
    /// Varsayılan stille metin.
    pub fn text(text: impl Into<String>) -> Self {
        Run::Text {
            text: text.into(),
            style: TextStyle::default(),
        }
    }
}

/// Tablo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Tablo adı.
    pub name: Option<String>,
    /// Sütun sayısı.
    pub column_count: usize,
    /// Sütun genişlikleri (pt).
    pub column_spans: Vec<f64>,
    /// Kenarlık türü.
    pub border: String,
    /// Satırlar.
    pub rows: Vec<Row>,
}

impl Table {
    /// Yeni tablo.
    pub fn new(column_spans: Vec<f64>) -> Self {
        Self {
            name: None,
            column_count: column_spans.len(),
            column_spans,
            border: "borderCell".to_string(),
            rows: Vec::new(),
        }
    }

    /// Satır ekler.
    pub fn push_row(&mut self, row: Row) {
        self.rows.push(row);
    }
}

/// Tablo satırı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    /// Satır adı.
    pub name: Option<String>,
    /// Satır türü.
    pub row_type: String,
    /// Kenarlık.
    pub border: String,
    /// Yükseklik (pt); ayırıcı satırlar için 0.0.
    pub height: f64,
    /// Hücreler.
    pub cells: Vec<Cell>,
}

impl Default for Row {
    fn default() -> Self {
        Self {
            name: None,
            row_type: "dataRow".to_string(),
            border: "borderTable".to_string(),
            height: 20.0,
            cells: Vec::new(),
        }
    }
}

impl Row {
    /// Paragraflardan oluşan hücrelerle veri satırı.
    pub fn from_texts(texts: &[String]) -> Self {
        Self {
            cells: texts
                .iter()
                .map(|t| Cell {
                    blocks: vec![UdfBlock::Paragraph(Paragraph::text(t.clone()))],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    /// Başlık satırı.
    pub fn header(texts: &[String]) -> Self {
        let mut r = Self::from_texts(texts);
        r.row_type = "headerRow".to_string();
        r
    }
}

/// Tablo hücresi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// Sütun birleştirme sayısı.
    pub colspan: u32,
    /// Dikey hizalama (`top`, `vcenter`, `bottom`).
    pub align: String,
    /// Dolgu rengi.
    pub fill_color: Option<i32>,
    /// Kenarlık.
    pub border: String,
    /// Kenarlık stili.
    pub border_style: String,
    /// Kenarlık kalınlığı (pt).
    pub border_width: f64,
    /// Kenarlık rengi.
    pub border_color: i32,
    /// Bit düzeyinde kenarlık belirteci.
    pub border_spec: u8,
    /// İçerik blokları.
    pub blocks: Vec<UdfBlock>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            colspan: 1,
            align: "top".to_string(),
            fill_color: None,
            border: "borderCell".to_string(),
            border_style: "borderStyle-solid".to_string(),
            border_width: 1.0,
            border_color: crate::DEFAULT_FOREGROUND,
            border_spec: 15,
            blocks: Vec::new(),
        }
    }
}

/// İşaretli ARGB rengi `#RRGGBB` onaltılık metne çevirir.
pub fn argb_to_hex(value: i32) -> String {
    let v = value as u32;
    format!("#{:02X}{:02X}{:02X}", (v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF)
}

/// `#RRGGBB` onaltılık metni işaretli ARGB tam sayısına çevirir.
pub fn hex_to_argb(hex: &str) -> Option<i32> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u32::from_str_radix(&h[0..2], 16).ok()?;
    let g = u32::from_str_radix(&h[2..4], 16).ok()?;
    let b = u32::from_str_radix(&h[4..6], 16).ok()?;
    Some((0xFF00_0000u32 | (r << 16) | (g << 8) | b) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_is_default_foreground() {
        assert_eq!(argb_to_hex(-16777216), "#000000");
    }

    #[test]
    fn white_argb_round_trips() {
        assert_eq!(argb_to_hex(-1), "#FFFFFF");
        assert_eq!(hex_to_argb("#FFFFFF"), Some(-1));
    }

    #[test]
    fn red_argb_is_correct() {
        assert_eq!(argb_to_hex(-65536), "#FF0000");
        assert_eq!(hex_to_argb("#FF0000"), Some(-65536));
    }

    #[test]
    fn empty_paragraph_gets_placeholder() {
        let p = Paragraph::default();
        assert_eq!(p.plain_text(), "\u{200B}");
        assert!(p.is_effectively_empty());
    }

    #[test]
    fn paragraph_text_concatenates_runs() {
        let p = Paragraph {
            runs: vec![
                Run::text("Merhaba "),
                Run::Text {
                    text: "dünya".into(),
                    style: TextStyle::bold(),
                },
                Run::Tab,
            ],
            ..Default::default()
        };
        assert_eq!(p.plain_text(), "Merhaba dünya\t");
        assert!(!p.is_effectively_empty());
    }

    #[test]
    fn default_document_has_two_styles() {
        let d = UdfDocument::new();
        assert_eq!(d.styles.len(), 2);
        assert_eq!(d.page.media_size_name, "1");
    }

    #[test]
    fn plain_text_walks_tables() {
        let mut t = Table::new(vec![100.0, 100.0]);
        t.push_row(Row::header(&["A".into(), "B".into()]));
        let mut d = UdfDocument::new();
        d.push(UdfBlock::Table(t));
        let text = d.plain_text();
        assert!(text.contains('A'));
        assert!(text.contains('B'));
    }
}
