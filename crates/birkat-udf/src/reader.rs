//! UDF okuyucu (15.2).
//!
//! Okuyucu, `content.xml` içindeki metin havuzunu ve `<elements>` ağacını
//! çözer; offset tabanlı metin parçalarını gerçek metne dönüştürür.
//!
//! Okuyucu, UYAP Editör tarafından üretilmiş belgeleri de okuyabilmelidir.
//! Bu nedenle bilinmeyen öğeler ve öznitelikler hataya yol açmaz; atlanır.
//! Havuz sınırını aşan offset'ler [`UdfReadResult::warnings`] içinde
//! bildirilir ve çökme yerine güvenli davranış üretilir.
//!
//! ## Yapı
//!
//! İç içe yapılar (tablo → satır → hücre → paragraf) açık bir kap (container)
//! yığını ile kurulur. Ayrıştırıcı durumu `ParserState` içinde tutulur; bu
//! sayede okuyucu saf, test edilebilir ve thread-güvenlidir.

use crate::model::*;
use birkat_core::error::{CoreError, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

/// Okuma sonucu.
#[derive(Debug, Clone)]
pub struct UdfReadResult {
    /// Çözülen belge.
    pub document: UdfDocument,
    /// Metin havuzu.
    pub pool: String,
    /// Okuma sırasında karşılaşılan uyarılar.
    pub warnings: Vec<String>,
    /// Belge kökündeki `format_id`.
    pub format_id: Option<String>,
}

/// Dosyadan UDF okur.
pub fn read_udf(path: impl AsRef<std::path::Path>) -> Result<UdfReadResult> {
    let bytes = std::fs::read(path.as_ref())?;
    read_udf_from_bytes(&bytes)
}

/// Baytlardan UDF okur.
pub fn read_udf_from_bytes(bytes: &[u8]) -> Result<UdfReadResult> {
    let xml = extract_content_xml(bytes)?;
    parse_content_xml(&xml)
}

/// Paketten `content.xml` metnini çıkarır.
fn extract_content_xml(bytes: &[u8]) -> Result<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| CoreError::Data(format!("UDF paketi açılamadı: {e}")))?;
    let mut file = archive
        .by_name(crate::CONTENT_XML)
        .map_err(|_| CoreError::Data("UDF paketinde content.xml bulunamadı.".to_string()))?;
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut file, &mut xml)
        .map_err(|e| CoreError::Data(format!("content.xml okunamadı: {e}")))?;
    Ok(xml)
}

/// İç içe yapı bağlamı.
enum Container {
    /// Tablo (satırları birikiyor).
    Table(Table),
    /// Satır (hücreleri birikiyor).
    Row(Row),
    /// Hücre (blokları birikiyor).
    Cell(Cell),
}

/// Ayrıştırıcı durumu.
#[derive(Default)]
struct ParserState {
    pool: String,
    format_id: Option<String>,
    warnings: Vec<String>,
    document: UdfDocument,
    stack: Vec<Container>,
    in_content_pool: bool,
    section: Section,
    paragraph: Option<Paragraph>,
    page_break: bool,
}

/// Belgenin hangi bölümünde olduğumuz.
#[derive(Default, Clone, Copy, PartialEq)]
enum Section {
    #[default]
    Body,
    Header,
    Footer,
}

impl ParserState {
    /// Bir bloğu geçerli bağlama yerleştirir.
    fn place_block(&mut self, block: UdfBlock) {
        // En içteki kap hücre ise bloğu oraya koy.
        for c in self.stack.iter_mut().rev() {
            if let Container::Cell(cell) = c {
                cell.blocks.push(block);
                return;
            }
        }
        match self.section {
            Section::Header => self.document.header.push(block),
            Section::Footer => self.document.footer.push(block),
            Section::Body => self.document.body.push(block),
        }
    }

    /// Hücreyi satıra aktarır.
    fn close_cell(&mut self) {
        let cell = match self.stack.pop() {
            Some(Container::Cell(c)) => c,
            Some(other) => {
                self.stack.push(other);
                return;
            }
            None => return,
        };
        if let Some(Container::Row(row)) = self.stack.last_mut() {
            row.cells.push(cell);
        }
    }

    /// Satırı tabloya aktarır.
    fn close_row(&mut self) {
        let row = match self.stack.pop() {
            Some(Container::Row(r)) => r,
            Some(other) => {
                self.stack.push(other);
                return;
            }
            None => return,
        };
        if let Some(Container::Table(t)) = self.stack.last_mut() {
            t.rows.push(row);
        }
    }

    /// Tabloyu belgeye aktarır.
    fn close_table(&mut self) {
        let table = match self.stack.pop() {
            Some(Container::Table(t)) => t,
            Some(other) => {
                self.stack.push(other);
                return;
            }
            None => return,
        };
        self.place_block(UdfBlock::Table(table));
    }

    /// Açık paragrafı kapatır.
    fn close_paragraph(&mut self) {
        if let Some(mut p) = self.paragraph.take() {
            // Boş paragraf işaretçisini boş metne çevir.
            for run in p.runs.iter_mut() {
                if let Run::Text { text, .. } = run {
                    if text == "\u{200B}" {
                        text.clear();
                    }
                }
            }
            self.place_block(UdfBlock::Paragraph(p));
        }
    }
}

/// `content.xml` metnini belge modeline çözer.
pub fn parse_content_xml(xml: &str) -> Result<UdfReadResult> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut st = ParserState::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => handle_start(&mut st, &e, false),
            Ok(Event::Empty(e)) => handle_start(&mut st, &e, true),
            Ok(Event::Text(t)) => {
                if st.in_content_pool {
                    let text = t
                        .unescape()
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(&t).to_string());
                    st.pool.push_str(&text);
                }
            }
            Ok(Event::CData(t)) => {
                if st.in_content_pool {
                    st.pool.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Ok(Event::End(e)) => handle_end(&mut st, local_name(e.name().as_ref()).as_str()),
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(CoreError::Data(format!("content.xml çözümlenemedi: {e}")));
            }
            _ => {}
        }
        buf.clear();
    }

    st.close_paragraph();
    while !st.stack.is_empty() {
        match st.stack.last() {
            Some(Container::Cell(_)) => st.close_cell(),
            Some(Container::Row(_)) => st.close_row(),
            Some(Container::Table(_)) => st.close_table(),
            None => break,
        }
    }

    if !xml.contains("<content") {
        st.warnings
            .push("Belgede metin havuzu bulunamadı.".to_string());
    }

    Ok(UdfReadResult {
        document: st.document,
        pool: st.pool,
        warnings: st.warnings,
        format_id: st.format_id,
    })
}

fn handle_start(st: &mut ParserState, e: &BytesStart<'_>, _empty: bool) {
    match local_name(e.name().as_ref()).as_str() {
        "template" => {
            let m = attrs_map(e);
            st.format_id = m.get("format_id").cloned();
        }
        "content" => {
            if st.paragraph.is_none() {
                // Metin havuzu elemanı.
                st.in_content_pool = true;
            } else if let Some(p) = st.paragraph.as_mut() {
                push_content_run(p, e, &st.pool, &mut st.warnings);
            }
        }
        "pageFormat" => apply_page_format(&mut st.document.page, e),
        "header" => st.section = Section::Header,
        "footer" => {
            st.section = Section::Footer;
            if let Some(s) = read_footer_style(e) {
                st.document.footer_page_number = Some(s);
            }
        }
        "style" => {
            if let Some(s) = read_style(e) {
                st.document.styles.push(s);
            }
        }
        "paragraph" => {
            st.close_paragraph();
            st.paragraph = Some(read_paragraph_attrs(e));
        }
        "table" => {
            st.close_paragraph();
            st.stack.push(Container::Table(read_table_attrs(e)));
        }
        "row" => st.stack.push(Container::Row(read_row_attrs(e))),
        "cell" => st.stack.push(Container::Cell(read_cell_attrs(e))),
        "page-break" => st.page_break = true,
        "tab" => {
            if let Some(p) = st.paragraph.as_mut() {
                p.runs.push(Run::Tab);
            }
        }
        "space" => {
            if let Some(p) = st.paragraph.as_mut() {
                p.runs.push(Run::Space);
            }
        }
        "image" => {
            if let Some(p) = st.paragraph.as_mut() {
                p.runs.push(read_image(e));
            }
        }
        "bgImage" => st.document.background = Some(read_bg_image(e)),
        _ => {}
    }
}

fn handle_end(st: &mut ParserState, name: &str) {
    match name {
        "content" => st.in_content_pool = false,
        "header" | "footer" => st.section = Section::Body,
        "paragraph" => st.close_paragraph(),
        "cell" => st.close_cell(),
        "row" => st.close_row(),
        "table" => st.close_table(),
        "page-break" if st.page_break => {
            st.page_break = false;
            st.place_block(UdfBlock::PageBreak);
        }
        _ => {}
    }
}

// --- öznitelik okuyucuları -------------------------------------------------

fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    match s.rsplit(':').next() {
        Some(n) => n.to_string(),
        None => s.to_string(),
    }
}

fn attrs_map(e: &BytesStart<'_>) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    for a in e.attributes().flatten() {
        let k = local_name(a.key.as_ref());
        let v = a
            .unescape_value()
            .map(|v| v.into_owned())
            .unwrap_or_default();
        m.insert(k, v);
    }
    m
}

fn push_content_run(p: &mut Paragraph, e: &BytesStart<'_>, pool: &str, warnings: &mut Vec<String>) {
    let m = attrs_map(e);
    let start: usize = m
        .get("startOffset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let len: usize = m.get("length").and_then(|v| v.parse().ok()).unwrap_or(0);

    let chars: Vec<char> = pool.chars().collect();
    if start > chars.len() || start.saturating_add(len) > chars.len() {
        warnings.push(format!(
            "Metin referansı havuz sınırlarını aşıyor (startOffset={start}, length={len})."
        ));
        return;
    }
    let text: String = chars[start..start + len].iter().collect();
    if text.is_empty() {
        return;
    }
    let style = TextStyle {
        family: m
            .get("family")
            .cloned()
            .unwrap_or_else(|| crate::DEFAULT_FONT.to_string()),
        size: m
            .get("size")
            .and_then(|v| v.parse().ok())
            .unwrap_or(crate::DEFAULT_FONT_SIZE),
        bold: m.get("bold").map(|v| v == "true").unwrap_or(false),
        italic: m.get("italic").map(|v| v == "true").unwrap_or(false),
        underline: m.get("underline").map(|v| v == "true").unwrap_or(false),
        foreground: m
            .get("foreground")
            .and_then(|v| v.parse().ok())
            .unwrap_or(crate::DEFAULT_FOREGROUND),
        background: m.get("background").and_then(|v| v.parse().ok()),
    };
    p.runs.push(Run::Text { text, style });
}

fn read_paragraph_attrs(e: &BytesStart<'_>) -> Paragraph {
    let m = attrs_map(e);
    let mut p = Paragraph {
        alignment: m.get("Alignment").and_then(|v| v.parse().ok()).unwrap_or(0),
        left_indent: m
            .get("LeftIndent")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
        right_indent: m
            .get("RightIndent")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
        first_line_indent: m.get("FirstLineIndent").and_then(|v| v.parse().ok()),
        line_spacing: m.get("LineSpacing").and_then(|v| v.parse().ok()),
        space_above: m.get("SpaceAbove").and_then(|v| v.parse().ok()),
        space_below: m.get("SpaceBelow").and_then(|v| v.parse().ok()),
        bulleted: m.get("Bulleted").map(|v| v == "true").unwrap_or(false),
        bullet_type: m.get("BulletType").cloned(),
        numbered: m.get("Numbered").map(|v| v == "true").unwrap_or(false),
        number_type: m.get("NumberType").cloned(),
        list_id: m.get("ListId").cloned(),
        list_level: m.get("ListLevel").and_then(|v| v.parse().ok()),
        style: m.get("style").cloned(),
        ..Default::default()
    };
    if let Some(ts) = m.get("TabSet") {
        for part in ts.split(',') {
            let bits: Vec<&str> = part.split(':').collect();
            if bits.len() == 3 {
                if let (Ok(pos), Ok(al), Ok(ld)) = (
                    bits[0].parse::<f64>(),
                    bits[1].parse::<u8>(),
                    bits[2].parse::<u8>(),
                ) {
                    p.tab_set.push((pos, al, ld));
                }
            }
        }
    }
    p
}

fn read_table_attrs(e: &BytesStart<'_>) -> Table {
    let m = attrs_map(e);
    let spans: Vec<f64> = m
        .get("columnSpans")
        .map(|v| {
            v.split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect()
        })
        .unwrap_or_default();
    let column_count = m
        .get("columnCount")
        .and_then(|v| v.parse().ok())
        .unwrap_or(spans.len());
    Table {
        name: m.get("tableName").cloned(),
        column_count,
        column_spans: spans,
        border: m
            .get("border")
            .cloned()
            .unwrap_or_else(|| "borderCell".to_string()),
        rows: Vec::new(),
    }
}

fn read_row_attrs(e: &BytesStart<'_>) -> Row {
    let m = attrs_map(e);
    Row {
        name: m.get("rowName").cloned(),
        row_type: m
            .get("rowType")
            .cloned()
            .unwrap_or_else(|| "dataRow".to_string()),
        border: m
            .get("border")
            .cloned()
            .unwrap_or_else(|| "borderTable".to_string()),
        height: m.get("height").and_then(|v| v.parse().ok()).unwrap_or(20.0),
        cells: Vec::new(),
    }
}

fn read_cell_attrs(e: &BytesStart<'_>) -> Cell {
    let m = attrs_map(e);
    Cell {
        colspan: m.get("colspan").and_then(|v| v.parse().ok()).unwrap_or(1),
        align: m.get("align").cloned().unwrap_or_else(|| "top".to_string()),
        fill_color: m.get("fillColor").and_then(|v| v.parse().ok()),
        border: m
            .get("border")
            .cloned()
            .unwrap_or_else(|| "borderCell".to_string()),
        border_style: m
            .get("borderStyle")
            .cloned()
            .unwrap_or_else(|| "borderStyle-solid".to_string()),
        border_width: m
            .get("borderWidth")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0),
        border_color: m
            .get("borderColor")
            .and_then(|v| v.parse().ok())
            .unwrap_or(crate::DEFAULT_FOREGROUND),
        border_spec: m
            .get("borderSpec")
            .and_then(|v| v.parse().ok())
            .unwrap_or(15),
        blocks: Vec::new(),
    }
}

fn read_image(e: &BytesStart<'_>) -> Run {
    let m = attrs_map(e);
    Run::Image {
        data_base64: m.get("imageData").cloned().unwrap_or_default(),
        width: m.get("width").and_then(|v| v.parse().ok()).unwrap_or(100.0),
        height: m
            .get("height")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100.0),
    }
}

fn read_bg_image(e: &BytesStart<'_>) -> BackgroundImage {
    let m = attrs_map(e);
    BackgroundImage {
        source: m.get("bgImageSource").cloned().unwrap_or_default(),
        data_base64: m.get("bgImageData").cloned().unwrap_or_default(),
        bottom_margin: m
            .get("bgImageBottomMargin")
            .and_then(|v| v.parse().ok())
            .unwrap_or(42.0),
        up_margin: m
            .get("bgImageUpMargin")
            .and_then(|v| v.parse().ok())
            .unwrap_or(42.0),
        // UYAP uyumluluğu için alan adı yazım hatasıyla korunur.
        right_margin: m
            .get("bgImageRigtMargin")
            .and_then(|v| v.parse().ok())
            .unwrap_or(42.0),
        left_margin: m
            .get("bgImageLeftMargin")
            .and_then(|v| v.parse().ok())
            .unwrap_or(42.0),
    }
}

fn read_footer_style(e: &BytesStart<'_>) -> Option<PageNumberStyle> {
    let m = attrs_map(e);
    if !m.contains_key("pageNumber-spec") {
        return None;
    }
    Some(PageNumberStyle {
        spec: m
            .get("pageNumber-spec")
            .cloned()
            .unwrap_or_else(|| "BSP32_40".to_string()),
        bold: m
            .get("pageNumber-fontBold")
            .map(|v| v == "true")
            .unwrap_or(false),
        italic: m
            .get("pageNumber-fontItalic")
            .map(|v| v == "true")
            .unwrap_or(false),
        font_face: m
            .get("pageNumber-fontFace")
            .cloned()
            .unwrap_or_else(|| "Arial".to_string()),
        font_size: m
            .get("pageNumber-fontSize")
            .and_then(|v| v.parse().ok())
            .unwrap_or(11.0),
        color: m
            .get("pageNumber-color")
            .and_then(|v| v.parse().ok())
            .unwrap_or(crate::DEFAULT_FOREGROUND),
    })
}

fn read_style(e: &BytesStart<'_>) -> Option<StyleDef> {
    let m = attrs_map(e);
    let name = m.get("name")?.clone();
    Some(StyleDef {
        name,
        description: m.get("description").cloned(),
        family: m
            .get("family")
            .cloned()
            .unwrap_or_else(|| crate::DEFAULT_FONT.to_string()),
        size: m
            .get("size")
            .and_then(|v| v.parse().ok())
            .unwrap_or(crate::DEFAULT_FONT_SIZE),
        bold: m.get("bold").map(|v| v == "true").unwrap_or(false),
        italic: m.get("italic").map(|v| v == "true").unwrap_or(false),
        foreground: m.get("foreground").and_then(|v| v.parse().ok()),
        font_attribute_key: m.get("FONT_ATTRIBUTE_KEY").cloned(),
    })
}

fn apply_page_format(page: &mut PageFormat, e: &BytesStart<'_>) {
    let m = attrs_map(e);
    if let Some(v) = m.get("mediaSizeName") {
        page.media_size_name = v.clone();
    }
    if let Some(v) = m.get("leftMargin").and_then(|v| v.parse().ok()) {
        page.left_margin = v;
    }
    if let Some(v) = m.get("rightMargin").and_then(|v| v.parse().ok()) {
        page.right_margin = v;
    }
    if let Some(v) = m.get("topMargin").and_then(|v| v.parse().ok()) {
        page.top_margin = v;
    }
    if let Some(v) = m.get("bottomMargin").and_then(|v| v.parse().ok()) {
        page.bottom_margin = v;
    }
    if let Some(v) = m.get("paperOrientation").and_then(|v| v.parse().ok()) {
        page.paper_orientation = v;
    }
    if let Some(v) = m.get("headerFOffset").and_then(|v| v.parse().ok()) {
        page.header_f_offset = v;
    }
    if let Some(v) = m.get("footerFOffset").and_then(|v| v.parse().ok()) {
        page.footer_f_offset = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(doc: UdfDocument) -> UdfReadResult {
        let xml = crate::write_udf(doc);
        parse_content_xml(&xml).expect("çözümlenmeli")
    }

    fn first_paragraph(r: &UdfReadResult) -> &Paragraph {
        match &r.document.body[0] {
            UdfBlock::Paragraph(p) => p,
            _ => panic!("paragraf bekleniyordu"),
        }
    }

    #[test]
    fn simple_paragraph_round_trips() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("Merhaba dünya")));
        let r = round_trip(doc);
        assert_eq!(r.pool, "Merhaba dünya");
        assert_eq!(first_paragraph(&r).plain_text(), "Merhaba dünya");
    }

    #[test]
    fn turkish_characters_survive_round_trip() {
        let text = "İĞÜŞÖÇığüşöç — “tırnak” 2025/123";
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text(text)));
        let r = round_trip(doc);
        assert_eq!(first_paragraph(&r).plain_text(), text);
    }

    #[test]
    fn styles_survive_round_trip() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::styled(
            "Kalın",
            TextStyle::bold(),
        )));
        let r = round_trip(doc);
        match &first_paragraph(&r).runs[0] {
            Run::Text { style, .. } => assert!(style.bold),
            _ => panic!("metin bekleniyordu"),
        }
    }

    #[test]
    fn table_round_trips() {
        let mut t = Table::new(vec![100.0, 200.0]);
        t.push_row(Row::header(&["A".into(), "B".into()]));
        t.push_row(Row::from_texts(&["1".into(), "iki".into()]));
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Table(t));
        let r = round_trip(doc);
        let t = match &r.document.body[0] {
            UdfBlock::Table(t) => t,
            _ => panic!("tablo bekleniyordu"),
        };
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0].cells.len(), 2);
        assert_eq!(t.rows[1].cells.len(), 2);
        assert_eq!(t.column_spans, vec![100.0, 200.0]);
        // Hücre metinleri doğru mu?
        let cell_text = |row: usize, col: usize| -> String {
            match &t.rows[row].cells[col].blocks[0] {
                UdfBlock::Paragraph(p) => p.plain_text(),
                _ => String::new(),
            }
        };
        assert_eq!(cell_text(0, 0), "A");
        assert_eq!(cell_text(1, 1), "iki");
    }

    #[test]
    fn table_inside_body_does_not_leak_into_following_paragraph() {
        let mut t = Table::new(vec![100.0]);
        t.push_row(Row::from_texts(&["Hücre".into()]));
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Table(t));
        doc.push(UdfBlock::Paragraph(Paragraph::text("Sonra")));
        let r = round_trip(doc);
        assert_eq!(r.document.body.len(), 2);
        assert!(matches!(r.document.body[0], UdfBlock::Table(_)));
        assert!(matches!(r.document.body[1], UdfBlock::Paragraph(_)));
    }

    #[test]
    fn page_format_round_trips() {
        let mut doc = UdfDocument::new();
        doc.page.left_margin = 50.0;
        doc.page.paper_orientation = 0;
        let r = round_trip(doc);
        assert_eq!(r.document.page.left_margin, 50.0);
        assert_eq!(r.document.page.paper_orientation, 0);
    }

    #[test]
    fn format_id_is_read() {
        let r = round_trip(UdfDocument::new());
        assert_eq!(r.format_id.as_deref(), Some("1.8"));
    }

    #[test]
    fn styles_section_round_trips() {
        let r = round_trip(UdfDocument::new());
        assert!(r.document.styles.iter().any(|s| s.name == "default"));
        assert!(r.document.styles.iter().any(|s| s.name == "hvl-default"));
    }

    #[test]
    fn footer_page_number_round_trips() {
        let mut doc = UdfDocument::new();
        doc.footer_page_number = Some(PageNumberStyle::default());
        doc.footer.push(UdfBlock::Paragraph(Paragraph::text("1")));
        let r = round_trip(doc);
        assert!(r.document.footer_page_number.is_some());
        assert_eq!(r.document.footer.len(), 1);
    }

    #[test]
    fn header_is_separate_from_body() {
        let mut doc = UdfDocument::new();
        doc.header.push(UdfBlock::Paragraph(Paragraph::text("ÜST")));
        doc.push(UdfBlock::Paragraph(Paragraph::text("GÖVDE")));
        let r = round_trip(doc);
        assert_eq!(r.document.header.len(), 1);
        assert_eq!(r.document.body.len(), 1);
        match &r.document.header[0] {
            UdfBlock::Paragraph(p) => assert_eq!(p.plain_text(), "ÜST"),
            _ => panic!(),
        }
    }

    #[test]
    fn empty_paragraph_round_trips_as_empty() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::default()));
        let r = round_trip(doc);
        assert!(first_paragraph(&r).is_effectively_empty());
    }

    #[test]
    fn out_of_bounds_offset_produces_warning_not_panic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" ?><template format_id="1.8"><content><![CDATA[abc]]></content><elements><paragraph Alignment="0" LeftIndent="0.0" RightIndent="0.0"><content startOffset="99" length="5" /></paragraph></elements></template>"#;
        let r = parse_content_xml(xml).unwrap();
        assert!(!r.warnings.is_empty());
    }

    #[test]
    fn unknown_elements_are_ignored() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" ?><template format_id="1.8"><content><![CDATA[abc]]></content><bilinmeyen x="1" /><elements><paragraph Alignment="0" LeftIndent="0.0" RightIndent="0.0"><content startOffset="0" length="3" /></paragraph></elements></template>"#;
        let r = parse_content_xml(xml).unwrap();
        assert_eq!(r.pool, "abc");
        assert_eq!(r.document.body.len(), 1);
    }

    #[test]
    fn cdata_split_terminator_is_rejoined() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("a]]>b")));
        let xml = crate::write_udf(doc);
        let r = parse_content_xml(&xml).unwrap();
        assert_eq!(r.pool, "a]]>b");
        assert_eq!(first_paragraph(&r).plain_text(), "a]]>b");
    }

    #[test]
    fn read_udf_from_packed_bytes() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("Paket testi")));
        let bytes = crate::write_udf_to_bytes(doc).unwrap();
        let r = read_udf_from_bytes(&bytes).unwrap();
        assert_eq!(r.pool, "Paket testi");
    }

    #[test]
    fn invalid_zip_is_rejected() {
        assert!(read_udf_from_bytes(b"bu bir zip degil").is_err());
    }

    #[test]
    fn image_round_trips() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph {
            runs: vec![Run::Image {
                data_base64: "QUJD".into(),
                width: 120.0,
                height: 60.0,
            }],
            ..Default::default()
        }));
        let r = round_trip(doc);
        match &first_paragraph(&r).runs[0] {
            Run::Image {
                data_base64,
                width,
                height,
            } => {
                assert_eq!(data_base64, "QUJD");
                assert_eq!(*width, 120.0);
                assert_eq!(*height, 60.0);
            }
            _ => panic!("resim bekleniyordu"),
        }
    }

    #[test]
    fn page_break_round_trips() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph::text("Birinci sayfa")));
        doc.push(UdfBlock::PageBreak);
        doc.push(UdfBlock::Paragraph(Paragraph::text("İkinci sayfa")));
        let r = round_trip(doc);
        assert!(r
            .document
            .body
            .iter()
            .any(|b| matches!(b, UdfBlock::PageBreak)));
    }

    #[test]
    fn tab_and_space_runs_round_trip() {
        let mut doc = UdfDocument::new();
        doc.push(UdfBlock::Paragraph(Paragraph {
            runs: vec![Run::text("A"), Run::Tab, Run::Space, Run::text("B")],
            ..Default::default()
        }));
        let r = round_trip(doc);
        let p = first_paragraph(&r);
        assert!(p.runs.iter().any(|r| matches!(r, Run::Tab)));
        assert!(p.runs.iter().any(|r| matches!(r, Run::Space)));
    }

    #[test]
    fn background_image_round_trips() {
        let mut doc = UdfDocument::new();
        doc.background = Some(BackgroundImage {
            source: String::new(),
            data_base64: "QUJD".into(),
            bottom_margin: 10.0,
            up_margin: 11.0,
            right_margin: 12.0,
            left_margin: 13.0,
        });
        let r = round_trip(doc);
        let bg = r.document.background.expect("arka plan bekleniyordu");
        assert_eq!(bg.data_base64, "QUJD");
        assert_eq!(bg.right_margin, 12.0);
    }
}
