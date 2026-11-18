//! UDF yazıcı (15.1).
//!
//! Yazıcı, belge modelini tek bir metin havuzu ve offset tabanlı
//! `<elements>` ağacına dönüştürür.
//!
//! ## Offset hesabı
//!
//! Tüm metin tek bir CDATA havuzunda toplanır. Her `<content>`, `<image>`,
//! `<tab>` ve `<space>` elemanı havuzdaki konumunu `startOffset` ve `length`
//! ile bildirir. Offset ve length **karakter (rune)** cinsindendir, bayt
//! değil.
//!
//! Havuz sırası: **üstbilgi → gövde → altbilgi**.
//!
//! ## Paragraf ayırıcı tercihi
//!
//! Format belgeleri havuzda paragrafların `\n` ile ayrıldığını yazar. Ancak
//! üretimde kullanılan ve UYAP Editör'de açıldığı bildirilen açık kaynak
//! dönüştürücüler ayırıcı eklemez; paragraf yapısı yalnızca `<elements>`
//! ağacından kurulur. Bu yazıcı varsayılan olarak bu davranışı izler
//! ([`UdfWriter::with_paragraph_separators`] ile değiştirilebilir).
//!
//! ## Kaçış (escaping)
//!
//! CDATA bloğu `]]>` dizisini içeremez; metinde bu dizi geçerse güvenli
//! biçimde bölünür. XML öznitelik değerleri kaçışlanır.

use crate::model::*;
use crate::FORMAT_ID;
use std::fmt::Write as _;

/// UDF yazıcı.
pub struct UdfWriter {
    doc: UdfDocument,
    pool: String,
    xml: String,
    paragraph_separators: bool,
}

impl UdfWriter {
    /// Yeni yazıcı. Belge hemen işlenir.
    pub fn new(doc: UdfDocument) -> Self {
        Self::with_options(doc, false)
    }

    /// Paragraf aralarına `\n` ayırıcı ekleyerek yeni yazıcı oluşturur.
    pub fn with_paragraph_separators(doc: UdfDocument, enabled: bool) -> Self {
        Self::with_options(doc, enabled)
    }

    fn with_options(doc: UdfDocument, paragraph_separators: bool) -> Self {
        let mut w = Self {
            doc,
            pool: String::new(),
            xml: String::new(),
            paragraph_separators,
        };
        w.xml = w.render();
        w
    }

    /// Belgeyi UDF `content.xml` metnine dönüştürür.
    pub fn to_content_xml(self) -> String {
        self.xml
    }

    /// Belgeyi `content.xml` metnine çevirir (iç kullanım).
    fn render(&mut self) -> String {
        let header = self.doc.header.clone();
        let body = self.doc.body.clone();
        let footer = self.doc.footer.clone();

        let mut header_xml = String::new();
        for b in &header {
            header_xml.push_str(&self.render_block(b));
        }
        let mut body_xml = String::new();
        for b in &body {
            body_xml.push_str(&self.render_block(b));
        }
        let mut footer_xml = String::new();
        for b in &footer {
            footer_xml.push_str(&self.render_block(b));
        }

        let mut out = String::with_capacity(self.pool.len() * 2 + 4096);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>");
        let _ = write!(out, "<template format_id=\"{FORMAT_ID}\">");

        out.push_str("<content><![CDATA[");
        out.push_str(&cdata_safe(&self.pool));
        out.push_str("]]></content>");

        out.push_str("<properties>");
        out.push_str(&self.render_page_format());
        if let Some(bg) = &self.doc.background {
            out.push_str(&render_bg_image(bg));
        }
        out.push_str("</properties>");

        out.push_str("<elements resolver=\"hvl-default\">");
        if !header.is_empty() {
            out.push_str("<header>");
            out.push_str(&header_xml);
            out.push_str("</header>");
        }
        out.push_str(&body_xml);
        if !footer.is_empty() || self.doc.footer_page_number.is_some() {
            out.push_str(&render_footer_open(self.doc.footer_page_number.as_ref()));
            out.push_str(&footer_xml);
            out.push_str("</footer>");
        }
        out.push_str("</elements>");

        out.push_str("<styles>");
        for s in &self.doc.styles {
            out.push_str(&render_style(s));
        }
        out.push_str("</styles>");

        out.push_str("</template>");
        out
    }

    /// Tek bir bloğu yazar ve havuzu günceller.
    fn render_block(&mut self, block: &UdfBlock) -> String {
        match block {
            UdfBlock::Paragraph(p) => self.render_paragraph(p),
            UdfBlock::Table(t) => self.render_table(t),
            UdfBlock::PageBreak => {
                let offset = self.char_count();
                self.pool.push(crate::EMPTY_PARAGRAPH_PLACEHOLDER);
                format!(
                    "<page-break><paragraph Alignment=\"0\" LeftIndent=\"0.0\" RightIndent=\"0.0\"><content startOffset=\"{offset}\" length=\"1\" /></paragraph></page-break>"
                )
            }
        }
    }

    /// Paragrafı yazar.
    fn render_paragraph(&mut self, p: &Paragraph) -> String {
        let attrs = self.paragraph_attributes(p);
        let mut inner = String::new();
        if p.runs.is_empty() {
            let offset = self.char_count();
            self.pool.push(crate::EMPTY_PARAGRAPH_PLACEHOLDER);
            let _ = write!(
                inner,
                "<content startOffset=\"{offset}\" length=\"1\" family=\"{}\" size=\"{}\" />",
                xml_attr(&self.doc.default_text_style.family),
                fmt_num(self.doc.default_text_style.size)
            );
        } else {
            for run in &p.runs {
                inner.push_str(&self.render_run(run));
            }
        }
        let xml = format!("<paragraph {attrs}>{inner}</paragraph>");
        self.maybe_paragraph_separator();
        xml
    }

    fn maybe_paragraph_separator(&mut self) {
        if self.paragraph_separators {
            self.pool.push(crate::PARAGRAPH_SEPARATOR);
        }
    }

    fn paragraph_attributes(&self, p: &Paragraph) -> String {
        let mut attrs = format!(
            "Alignment=\"{}\" LeftIndent=\"{}\" RightIndent=\"{}\"",
            p.alignment,
            fmt_num(p.left_indent),
            fmt_num(p.right_indent)
        );
        if let Some(v) = p.first_line_indent {
            let _ = write!(attrs, " FirstLineIndent=\"{}\"", fmt_num(v));
        }
        if let Some(v) = p.line_spacing {
            let _ = write!(attrs, " LineSpacing=\"{}\"", fmt_num(v));
        }
        if let Some(v) = p.space_above {
            let _ = write!(attrs, " SpaceAbove=\"{}\"", fmt_num(v));
        }
        if let Some(v) = p.space_below {
            let _ = write!(attrs, " SpaceBelow=\"{}\"", fmt_num(v));
        }
        if !p.tab_set.is_empty() {
            let tabs: Vec<String> = p
                .tab_set
                .iter()
                .map(|(pos, align, leader)| format!("{}:{}:{}", fmt_num(*pos), align, leader))
                .collect();
            let _ = write!(attrs, " TabSet=\"{}\"", tabs.join(","));
        }
        if p.bulleted {
            attrs.push_str(" Bulleted=\"true\"");
            if let Some(bt) = &p.bullet_type {
                let _ = write!(attrs, " BulletType=\"{}\"", xml_attr(bt));
            }
        }
        if p.numbered {
            attrs.push_str(" Numbered=\"true\"");
            if let Some(nt) = &p.number_type {
                let _ = write!(attrs, " NumberType=\"{}\"", xml_attr(nt));
            }
        }
        if let Some(id) = &p.list_id {
            let _ = write!(attrs, " ListId=\"{}\"", xml_attr(id));
        }
        if let Some(level) = p.list_level {
            let _ = write!(attrs, " ListLevel=\"{level}\"");
        }
        if let Some(style) = &p.style {
            let _ = write!(attrs, " style=\"{}\"", xml_attr(style));
        }
        attrs
    }

    /// Satır içi öğeyi yazar ve havuzu günceller.
    fn render_run(&mut self, run: &Run) -> String {
        match run {
            Run::Text { text, style } => {
                if text.is_empty() {
                    return String::new();
                }
                let offset = self.char_count();
                let len = text.chars().count();
                self.pool.push_str(text);
                let mut s = format!(
                    "<content startOffset=\"{offset}\" length=\"{len}\" family=\"{}\" size=\"{}\"",
                    xml_attr(&style.family),
                    fmt_num(style.size)
                );
                if style.bold {
                    s.push_str(" bold=\"true\"");
                }
                if style.italic {
                    s.push_str(" italic=\"true\"");
                }
                if style.underline {
                    s.push_str(" underline=\"true\"");
                }
                if style.foreground != crate::DEFAULT_FOREGROUND {
                    let _ = write!(s, " foreground=\"{}\"", style.foreground);
                }
                if let Some(bg) = style.background {
                    let _ = write!(s, " background=\"{bg}\"");
                }
                s.push_str(" />");
                s
            }
            Run::Tab => {
                let offset = self.char_count();
                self.pool.push(crate::TAB_CHAR);
                format!(
                    "<tab startOffset=\"{offset}\" length=\"1\" family=\"{}\" size=\"{}\" />",
                    xml_attr(&self.doc.default_text_style.family),
                    fmt_num(self.doc.default_text_style.size)
                )
            }
            Run::Image {
                data_base64,
                width,
                height,
            } => {
                let offset = self.char_count();
                self.pool.push(crate::IMAGE_PLACEHOLDER);
                format!(
                    "<image imageData=\"{}\" startOffset=\"{offset}\" length=\"1\" width=\"{}\" height=\"{}\" />",
                    xml_attr(data_base64),
                    fmt_num(*width),
                    fmt_num(*height)
                )
            }
            Run::Space => {
                let offset = self.char_count();
                self.pool.push(' ');
                format!("<space startOffset=\"{offset}\" length=\"1\" />")
            }
        }
    }

    /// Tabloyu yazar.
    fn render_table(&mut self, t: &Table) -> String {
        let spans: Vec<String> = t.column_spans.iter().map(|v| fmt_num(*v)).collect();
        let mut s = format!(
            "<table columnCount=\"{}\" columnSpans=\"{}\" border=\"{}\"",
            t.column_count,
            spans.join(","),
            xml_attr(&t.border)
        );
        if let Some(name) = &t.name {
            let _ = write!(s, " tableName=\"{}\"", xml_attr(name));
        }
        s.push('>');

        let rows = t.rows.clone();
        for row in &rows {
            let mut ra = format!(
                "rowType=\"{}\" border=\"{}\" height=\"{}\"",
                xml_attr(&row.row_type),
                xml_attr(&row.border),
                fmt_num(row.height)
            );
            if let Some(name) = &row.name {
                let _ = write!(ra, " rowName=\"{}\"", xml_attr(name));
            }
            let _ = write!(s, "<row {ra}>");
            for cell in &row.cells {
                s.push_str(&self.render_cell(cell));
            }
            s.push_str("</row>");
        }
        s.push_str("</table>");
        s
    }

    /// Hücreyi yazar.
    fn render_cell(&mut self, cell: &Cell) -> String {
        let mut ca = format!(
            "colspan=\"{}\" align=\"{}\" border=\"{}\" borderStyle=\"{}\" borderWidth=\"{}\" borderColor=\"{}\" borderSpec=\"{}\"",
            cell.colspan,
            xml_attr(&cell.align),
            xml_attr(&cell.border),
            xml_attr(&cell.border_style),
            fmt_num(cell.border_width),
            cell.border_color,
            cell.border_spec
        );
        if let Some(fc) = cell.fill_color {
            let _ = write!(ca, " fillColor=\"{fc}\"");
        }
        let mut inner = String::new();
        if cell.blocks.is_empty() {
            let offset = self.char_count();
            self.pool.push(crate::EMPTY_PARAGRAPH_PLACEHOLDER);
            let _ = write!(
                inner,
                "<paragraph Alignment=\"0\" LeftIndent=\"0.0\" RightIndent=\"0.0\"><content startOffset=\"{offset}\" length=\"1\" /></paragraph>"
            );
        } else {
            let blocks = cell.blocks.clone();
            for b in &blocks {
                inner.push_str(&self.render_block(b));
            }
        }
        format!("<cell {ca}>{inner}</cell>")
    }

    /// `pageFormat` elemanı.
    fn render_page_format(&self) -> String {
        let p = &self.doc.page;
        format!(
            "<pageFormat mediaSizeName=\"{}\" leftMargin=\"{}\" rightMargin=\"{}\" topMargin=\"{}\" bottomMargin=\"{}\" paperOrientation=\"{}\" headerFOffset=\"{}\" footerFOffset=\"{}\" />",
            xml_attr(&p.media_size_name),
            fmt_num(p.left_margin),
            fmt_num(p.right_margin),
            fmt_num(p.top_margin),
            fmt_num(p.bottom_margin),
            p.paper_orientation,
            fmt_num(p.header_f_offset),
            fmt_num(p.footer_f_offset)
        )
    }

    /// Havuzdaki karakter sayısı (offset hesabı için).
    fn char_count(&self) -> usize {
        self.pool.chars().count()
    }

    /// Belge modelini döndürür.
    pub fn document(&self) -> &UdfDocument {
        &self.doc
    }

    /// Metin havuzunu döndürür.
    pub fn pool(&self) -> &str {
        &self.pool
    }
}

/// Altbilgi açılış etiketi.
fn render_footer_open(style: Option<&PageNumberStyle>) -> String {
    match style {
        Some(p) => format!(
            "<footer pageNumber-spec=\"{}\" pageNumber-fontBold=\"{}\" pageNumber-fontItalic=\"{}\" pageNumber-fontFace=\"{}\" pageNumber-fontSize=\"{}\" pageNumber-color=\"{}\">",
            xml_attr(&p.spec),
            p.bold,
            p.italic,
            xml_attr(&p.font_face),
            fmt_num(p.font_size),
            p.color
        ),
        None => "<footer>".to_string(),
    }
}

/// Arka plan resmi elemanı.
fn render_bg_image(bg: &BackgroundImage) -> String {
    format!(
        "<bgImage bgImageSource=\"{}\" bgImageData=\"{}\" bgImageBottomMargin=\"{}\" bgImageUpMargin=\"{}\" bgImageRigtMargin=\"{}\" bgImageLeftMargin=\"{}\" />",
        xml_attr(&bg.source),
        xml_attr(&bg.data_base64),
        fmt_num(bg.bottom_margin),
        fmt_num(bg.up_margin),
        fmt_num(bg.right_margin),
        fmt_num(bg.left_margin)
    )
}

/// Stil elemanı.
fn render_style(s: &StyleDef) -> String {
    let mut out = format!(
        "<style name=\"{}\" family=\"{}\" size=\"{}\" bold=\"{}\" italic=\"{}\"",
        xml_attr(&s.name),
        xml_attr(&s.family),
        fmt_num(s.size),
        s.bold,
        s.italic
    );
    if let Some(d) = &s.description {
        let _ = write!(out, " description=\"{}\"", xml_attr(d));
    }
    if let Some(fg) = s.foreground {
        let _ = write!(out, " foreground=\"{fg}\"");
    }
    if let Some(k) = &s.font_attribute_key {
        let _ = write!(out, " FONT_ATTRIBUTE_KEY=\"{}\"", xml_attr(k));
    }
    out.push_str(" />");
    out
}

/// Sayıyı UDF biçiminde yazar.
pub fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

/// XML öznitelik değerini kaçışlar.
pub fn xml_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// CDATA bloğu içinde `]]>` dizisini güvenli hale getirir.
pub fn cdata_safe(text: &str) -> String {
    text.replace("]]>", "]]]]><![CDATA[>")
}

/// UDF paketleme hataları.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// ZIP yazma hatası.
    #[error("UDF paketleme hatası: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// Girdi/çıktı hatası.
    #[error("girdi/çıktı hatası: {0}")]
    Io(#[from] std::io::Error),
}

/// Belgeyi `content.xml` metnine dönüştürür.
pub fn write_udf(doc: UdfDocument) -> String {
    UdfWriter::new(doc).to_content_xml()
}

/// Belgeyi Deflate sıkıştırmalı UDF paketi (ZIP) olarak üretir.
pub fn write_udf_to_bytes(doc: UdfDocument) -> Result<Vec<u8>, WriteError> {
    let xml = UdfWriter::new(doc).to_content_xml();
    pack_udf(&xml)
}

/// `content.xml` metnini Deflate sıkıştırmalı ZIP paketine koyar.
pub fn pack_udf(content_xml: &str) -> Result<Vec<u8>, WriteError> {
    use std::io::Write as _;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options: zip::write::SimpleFileOptions = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);
        zip.start_file(crate::CONTENT_XML, options)?;
        zip.write_all(content_xml.as_bytes())?;
        zip.finish()?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_num_keeps_one_decimal_for_integers() {
        assert_eq!(fmt_num(42.0), "42.0");
        assert_eq!(fmt_num(0.0), "0.0");
        assert_eq!(fmt_num(1.5), "1.5");
    }

    #[test]
    fn xml_attr_escapes_specials() {
        assert_eq!(xml_attr("a&b<c>\"d'"), "a&amp;b&lt;c&gt;&quot;d&apos;");
    }

    #[test]
    fn cdata_safe_splits_terminator() {
        let out = cdata_safe("a]]>b");
        assert!(out.contains("]]]]><![CDATA[>"));
        assert!(!out.contains("]]>b"));
    }

    #[test]
    fn offsets_are_character_based_and_contiguous() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("Merhaba")));
        doc.push(UdfBlock::Paragraph(Paragraph::text("Dünya ğüşiöç")));
        let xml = write_udf(doc);
        assert!(xml.contains("startOffset=\"0\" length=\"7\""));
        assert!(xml.contains("startOffset=\"7\" length=\"12\""));
    }

    #[test]
    fn turkish_characters_count_as_one() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("İĞÜŞÖÇığüşöç")));
        let xml = write_udf(doc);
        assert!(xml.contains("length=\"12\""), "xml: {xml}");
    }

    #[test]
    fn content_xml_has_required_structure() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("Deneme")));
        let xml = write_udf(doc);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>"));
        assert!(xml.contains("<template format_id=\"1.8\">"));
        assert!(xml.contains("<content><![CDATA["));
        assert!(xml.contains("]]></content>"));
        assert!(xml.contains("<properties>"));
        assert!(xml.contains("<pageFormat"));
        assert!(xml.contains("<elements resolver=\"hvl-default\">"));
        assert!(xml.contains("<styles>"));
        assert!(xml.ends_with("</template>"));
    }

    #[test]
    fn table_renders_rows_and_cells() {
        let mut t = Table::new(vec![150.0, 200.0]);
        t.push_row(Row::header(&["Soru".into(), "Yanıt".into()]));
        t.push_row(Row::from_texts(&["1".into(), "Yanıt metni".into()]));
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Table(t));
        let xml = write_udf(doc);
        assert!(xml.contains("columnCount=\"2\""));
        assert!(xml.contains("columnSpans=\"150.0,200.0\""));
        assert_eq!(xml.matches("<row ").count(), 2);
        assert_eq!(xml.matches("<cell ").count(), 4);
    }

    #[test]
    fn image_placeholder_is_used() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph {
            runs: vec![Run::Image {
                data_base64: "AAAA".into(),
                width: 100.0,
                height: 50.0,
            }],
            ..Default::default()
        }));
        let xml = write_udf(doc);
        assert!(xml.contains("<image imageData=\"AAAA\""));
        assert!(xml.contains("length=\"1\""));
    }

    #[test]
    fn empty_paragraph_gets_zero_width_space() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::default()));
        let w = UdfWriter::new(doc);
        let pool = w.pool().to_string();
        let xml = w.to_content_xml();
        assert!(xml.contains("length=\"1\""));
        assert!(pool.contains('\u{200B}'));
    }

    #[test]
    fn list_attributes_are_written() {
        let mut p = Paragraph::text("Madde");
        p.bulleted = true;
        p.bullet_type = Some("BULLET_TYPE_ELLIPSE".into());
        p.list_id = Some("1".into());
        p.list_level = Some(0);
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(p));
        let xml = write_udf(doc);
        assert!(xml.contains("Bulleted=\"true\""));
        assert!(xml.contains("BulletType=\"BULLET_TYPE_ELLIPSE\""));
        assert!(xml.contains("ListLevel=\"0\""));
    }

    #[test]
    fn footer_page_number_attributes_are_written() {
        let mut doc = UdfDocument::new();
        doc.footer_page_number = Some(PageNumberStyle::default());
        doc.footer.push(UdfBlock::Paragraph(Paragraph::text("1")));
        let xml = write_udf(doc);
        assert!(xml.contains("pageNumber-spec=\"BSP32_40\""));
        assert!(xml.contains("pageNumber-fontFace=\"Arial\""));
    }

    #[test]
    fn header_is_written_before_body() {
        let mut doc = UdfDocument::new();
        doc.header
            .push(UdfBlock::Paragraph(Paragraph::text("BAŞLIK")));
        doc.push(UdfBlock::Paragraph(Paragraph::text("Gövde")));
        let w = UdfWriter::new(doc);
        let pool = w.pool().to_string();
        let xml = w.to_content_xml();
        assert!(pool.starts_with("BAŞLIK"));
        assert!(xml.contains("<header>"));
        assert!(xml.contains("startOffset=\"6\" length=\"5\""), "xml: {xml}");
    }

    #[test]
    fn background_image_uses_uyap_typo_attribute() {
        let mut doc = UdfDocument::new();
        doc.background = Some(BackgroundImage {
            source: String::new(),
            data_base64: "AAAA".into(),
            bottom_margin: 42.0,
            up_margin: 42.0,
            right_margin: 42.0,
            left_margin: 42.0,
        });
        let xml = write_udf(doc);
        assert!(xml.contains("bgImageRigtMargin="));
    }

    #[test]
    fn pack_udf_produces_readable_zip() {
        let bytes = pack_udf("<template format_id=\"1.8\"></template>").unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(archive.len(), 1);
        assert!(archive.by_name("content.xml").is_ok());
    }

    #[test]
    fn paragraph_separator_option_works() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("a")));
        doc.push(UdfBlock::Paragraph(Paragraph::text("b")));
        let w = UdfWriter::with_paragraph_separators(doc.clone(), true);
        let w2 = UdfWriter::new(doc);
        assert!(w.pool().contains('\n'));
        assert!(!w2.pool().contains('\n'));
    }
}
