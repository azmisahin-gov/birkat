//! Yardımcı çıktı biçimleri (15.6).
//!
//! Birincil rapor UDF'dir. PDF, DOCX, TXT, CSV/XLSX, JSON, SRT/VTT yardımcı
//! çıktılardır ve UDF'nin yerine geçmez.
//!
//! Bu modül JSON, TXT, CSV ve SRT/VTT üretimini içerir. PDF ve DOCX ayrı
//! modüllerde üretilir.

use birkat_core::report::{Block, ReportDocument, SectionId};
use birkat_core::time_util::Timestamp;

/// Raporun yardımcı çıktı biçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuxiliaryFormat {
    /// Düz metin.
    Txt,
    /// JSON (kanonik veri).
    Json,
    /// CSV (tablolar).
    Csv,
    /// SRT altyazı.
    Srt,
    /// WebVTT altyazı.
    Vtt,
    /// Markdown.
    Markdown,
}

impl AuxiliaryFormat {
    /// Dosya uzantısı.
    pub fn extension(&self) -> &'static str {
        match self {
            AuxiliaryFormat::Txt => "txt",
            AuxiliaryFormat::Json => "json",
            AuxiliaryFormat::Csv => "csv",
            AuxiliaryFormat::Srt => "srt",
            AuxiliaryFormat::Vtt => "vtt",
            AuxiliaryFormat::Markdown => "md",
        }
    }

    /// Bu biçimin UDF'nin yerine geçmediğini açıklar.
    pub fn note(&self) -> &'static str {
        "Bu dosya yardımcı çıktıdır; UDF raporunun yerine geçmez."
    }
}

/// Raporu düz metne çevirir.
pub fn to_text(report: &ReportDocument) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} — {}\n",
        report.kind.label(),
        report.report_date.to_rfc3339()
    ));
    out.push_str(&format!("Sürüm: {}\n", report.version));
    out.push_str(&format!("Durum: {}\n\n", report.status.label()));

    for s in &report.sections {
        if s.hidden {
            continue;
        }
        out.push_str(&format!("== {} ==\n", s.title));
        for b in &s.blocks {
            match b {
                Block::Paragraph { text, .. } => out.push_str(&format!("{text}\n")),
                Block::Heading { text, level } => {
                    out.push_str(&format!("{} {text}\n", "#".repeat(*level as usize)))
                }
                Block::BulletList { items } => {
                    for i in items {
                        out.push_str(&format!("- {i}\n"));
                    }
                }
                Block::NumberedList { items, .. } => {
                    for (n, i) in items.iter().enumerate() {
                        out.push_str(&format!("{}. {i}\n", n + 1));
                    }
                }
                Block::Table { header, rows, .. } => {
                    out.push_str(&header.join(" | "));
                    out.push('\n');
                    out.push_str(&"-".repeat(40));
                    out.push('\n');
                    for r in rows {
                        out.push_str(&r.join(" | "));
                        out.push('\n');
                    }
                }
                Block::Image { caption, .. } => out.push_str(&format!(
                    "[Görsel{}]\n",
                    caption
                        .as_deref()
                        .map(|c| format!(": {c}"))
                        .unwrap_or_default()
                )),
                Block::PageBreak => out.push_str("\n--- sayfa sonu ---\n"),
                Block::TimeReference {
                    label,
                    start,
                    end,
                    quote,
                } => {
                    out.push_str(&format!("{label} [{start:.3} - {end:.3}]\n"));
                    if let Some(q) = quote {
                        out.push_str(&format!("  \"{q}\"\n"));
                    }
                }
                Block::TranscriptSegment {
                    time_code,
                    speaker,
                    text,
                } => {
                    out.push_str(&format!(
                        "[{time_code}] {}: {text}\n",
                        speaker.as_deref().unwrap_or("-")
                    ));
                }
                Block::Notice { title, text } => {
                    out.push_str(&format!("!!! {title}: {text}\n"));
                }
            }
            out.push('\n');
        }
    }
    out
}

/// Raporu JSON'a çevirir (kanonik veri).
pub fn to_json(report: &ReportDocument) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

/// Rapor tablolarını CSV'ye çevirir.
///
/// Her tablo `## <bölüm başlığı>` yorumuyla ayrılır.
pub fn to_csv(report: &ReportDocument) -> String {
    let mut out = String::new();
    for s in &report.sections {
        if s.hidden {
            continue;
        }
        for b in &s.blocks {
            if let Block::Table { header, rows, .. } = b {
                out.push_str(&format!("# {}\n", s.title));
                out.push_str(&csv_row(header));
                for r in rows {
                    out.push_str(&csv_row(r));
                }
                out.push('\n');
            }
        }
    }
    out
}

fn csv_row(cells: &[String]) -> String {
    cells
        .iter()
        .map(|c| {
            if c.contains(';') || c.contains('"') || c.contains('\n') {
                format!("\"{}\"", c.replace('"', "\"\""))
            } else {
                c.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(";")
        + "\n"
}

/// Saniyeyi SRT zaman damgasına çevirir (`HH:MM:SS,mmm`).
pub fn srt_timestamp(seconds: f64) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let s = total_secs % 60;
    let m = (total_secs / 60) % 60;
    let h = total_secs / 3600;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

/// Saniyeyi WebVTT zaman damgasına çevirir (`HH:MM:SS.mmm`).
pub fn vtt_timestamp(seconds: f64) -> String {
    srt_timestamp(seconds).replace(',', ".")
}

/// Raporun transkript bölümlerinden SRT üretir.
pub fn to_srt(report: &ReportDocument) -> String {
    let mut out = String::new();
    let mut idx = 1;
    for s in &report.sections {
        if s.hidden {
            continue;
        }
        for b in &s.blocks {
            if let Block::TimeReference {
                start,
                end,
                quote,
                label,
            } = b
            {
                out.push_str(&format!("{idx}\n"));
                out.push_str(&format!(
                    "{} --> {}\n",
                    srt_timestamp(*start),
                    srt_timestamp(*end)
                ));
                let text = quote.clone().unwrap_or_else(|| label.clone());
                out.push_str(&format!("{text}\n\n"));
                idx += 1;
            }
        }
    }
    out
}

/// Raporun transkript bölümlerinden WebVTT üretir.
pub fn to_vtt(report: &ReportDocument) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for s in &report.sections {
        if s.hidden {
            continue;
        }
        for b in &s.blocks {
            if let Block::TimeReference {
                start,
                end,
                quote,
                label,
            } = b
            {
                out.push_str(&format!(
                    "{} --> {}\n",
                    vtt_timestamp(*start),
                    vtt_timestamp(*end)
                ));
                let text = quote.clone().unwrap_or_else(|| label.clone());
                out.push_str(&format!("{text}\n\n"));
            }
        }
    }
    out
}

/// Raporu Markdown'a çevirir.
pub fn to_markdown(report: &ReportDocument) -> String {
    to_text(report)
}

/// Raporu istenen yardımcı biçime çevirir.
pub fn export(report: &ReportDocument, format: AuxiliaryFormat) -> String {
    match format {
        AuxiliaryFormat::Txt => to_text(report),
        AuxiliaryFormat::Json => to_json(report),
        AuxiliaryFormat::Csv => to_csv(report),
        AuxiliaryFormat::Srt => to_srt(report),
        AuxiliaryFormat::Vtt => to_vtt(report),
        AuxiliaryFormat::Markdown => to_markdown(report),
    }
}

/// Yardımcı çıktıların birincil UDF ile ilişkisini açıklar.
pub fn auxiliary_note() -> &'static str {
    "Birincil rapor BILIRKISI_RAPORU.udf dosyasıdır. PDF, DOCX, TXT, CSV/XLSX, \
     JSON ve SRT/VTT dosyaları yardımcı çıktılardır ve UDF'nin yerine geçmez. \
     Ses dosyaları kontrolsüz biçimde UDF'nin içine gömülmez; ses, ayrı delil \
     materyali olarak korunur ve raporda dosya kimliği, hash, süre ve zaman \
     referansları bulunur."
}

/// Rapor tarihini biçimlendirir.
pub fn formatted_report_date(report: &ReportDocument) -> String {
    report.report_date.to_rfc3339()
}

/// Bilinmeyen tarih için yer tutucu.
pub fn unknown_date() -> Timestamp {
    Timestamp::default()
}

/// Bölüm başlığı listesini döndürür (içindekiler için).
pub fn table_of_contents(report: &ReportDocument) -> Vec<(SectionId, String)> {
    report
        .sections
        .iter()
        .filter(|s| !s.hidden)
        .map(|s| (s.id, s.title.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::ids::CaseId;
    use birkat_core::report::ReportKind;

    fn report() -> ReportDocument {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.push_block(
            SectionId::Subject,
            Block::Paragraph {
                text: "İnceleme konusu.".to_string(),
                align: birkat_core::report::Align::Justify,
            },
        );
        r.push_block(
            SectionId::AudioInventory,
            Block::Table {
                header: vec!["Dosya".into(), "Codec".into()],
                rows: vec![vec!["a.wav".into(), "PCM".into()]],
                column_widths: vec![100.0, 100.0],
            },
        );
        r.push_block(
            SectionId::TimeCodes,
            Block::TimeReference {
                label: "kayit1.wav".into(),
                start: 1.5,
                end: 3.25,
                quote: Some("Merhaba dünya".into()),
            },
        );
        r
    }

    #[test]
    fn text_export_contains_sections() {
        let t = to_text(&report());
        assert!(t.contains("İnceleme konusu."));
        assert!(t.contains("a.wav | PCM"));
    }

    #[test]
    fn json_export_round_trips() {
        let j = to_json(&report());
        let back: ReportDocument = serde_json::from_str(&j).unwrap();
        assert_eq!(back.sections.len(), report().sections.len());
    }

    #[test]
    fn csv_export_uses_semicolon_and_headers() {
        let c = to_csv(&report());
        assert!(c.contains("Dosya;Codec"));
        assert!(c.contains("a.wav;PCM"));
    }

    #[test]
    fn csv_quotes_special_characters() {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.push_block(
            SectionId::AudioInventory,
            Block::Table {
                header: vec!["a;b".into()],
                rows: vec![vec!["c\"d".into()]],
                column_widths: vec![100.0],
            },
        );
        let c = to_csv(&r);
        assert!(c.contains("\"a;b\""));
        assert!(c.contains("\"c\"\"d\""));
    }

    #[test]
    fn srt_timestamp_format() {
        assert_eq!(srt_timestamp(0.0), "00:00:00,000");
        assert_eq!(srt_timestamp(3661.5), "01:01:01,500");
    }

    #[test]
    fn vtt_timestamp_uses_dot() {
        assert_eq!(vtt_timestamp(1.5), "00:00:01.500");
    }

    #[test]
    fn srt_export_produces_blocks() {
        let s = to_srt(&report());
        assert!(s.contains("1\n"));
        assert!(s.contains("00:00:01,500 --> 00:00:03,250"));
        assert!(s.contains("Merhaba dünya"));
    }

    #[test]
    fn vtt_export_has_header() {
        let v = to_vtt(&report());
        assert!(v.starts_with("WEBVTT"));
        assert!(v.contains("00:00:01.500 --> 00:00:03.250"));
    }

    #[test]
    fn export_dispatches_by_format() {
        let r = report();
        assert!(export(&r, AuxiliaryFormat::Json)
            .trim_start()
            .starts_with('{'));
        assert!(export(&r, AuxiliaryFormat::Csv).contains(';'));
        assert!(export(&r, AuxiliaryFormat::Vtt).starts_with("WEBVTT"));
    }

    #[test]
    fn auxiliary_note_states_udf_is_primary() {
        let n = auxiliary_note();
        assert!(n.contains("BILIRKISI_RAPORU.udf"));
        assert!(n.contains("yerine geçmez"));
    }

    #[test]
    fn hidden_sections_are_skipped() {
        let mut r = report();
        r.section_mut(SectionId::Subject).unwrap().hidden = true;
        assert!(!to_text(&r).contains("İnceleme konusu."));
    }

    #[test]
    fn toc_lists_visible_sections() {
        let mut r = report();
        r.section_mut(SectionId::Cover).unwrap().hidden = true;
        let toc = table_of_contents(&r);
        assert!(!toc.iter().any(|(id, _)| *id == SectionId::Cover));
        assert!(toc.iter().any(|(id, _)| *id == SectionId::Subject));
    }
}
