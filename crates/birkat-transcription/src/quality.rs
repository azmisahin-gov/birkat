//! Otomatik transkripsiyon kalite kontrolü (11).
//!
//! Transkripsiyon tamamlandığında otomatik kalite kontrolü çalışır ve
//! şüpheli bölümleri bilirkişiye **inceleme önceliği** olarak gösterir.
//!
//! ## Dürüstlük sınırı
//!
//! Model log olasılık veya güven göstergeleri **gerçek doğruluk yüzdesi gibi
//! sunulmaz** (bkz. [`confidence_note`]). Hiçbir kalite göstergesi bilirkişi
//! onayının yerine geçmez.

use crate::hallucination;
use birkat_core::audio::{
    QualityFinding, QualityFindingKind, QualityReport, QualitySeverity,
};
use birkat_core::time_util::Timestamp;

/// Kalite kontrol girdisi — tek bir segment.
#[derive(Debug, Clone)]
pub struct QualitySegment {
    /// Segment indeksi.
    pub index: usize,
    /// Başlangıç (saniye).
    pub start: f64,
    /// Bitiş (saniye).
    pub end: f64,
    /// Metin.
    pub text: String,
    /// Ses düzeyi (dBFS).
    pub level_dbfs: Option<f32>,
    /// Konuşma var mı (VAD benzeri işaret)?
    pub has_speech: Option<bool>,
    /// Segmentte örtüşen konuşma var mı?
    pub overlapping_speech: bool,
    /// Segmentin dili (varsa).
    pub language: Option<String>,
}

/// Kalite kontrol girdisi.
#[derive(Debug, Clone)]
pub struct QualityControlInput {
    /// Segmentler.
    pub segments: Vec<QualitySegment>,
    /// Ses toplam süresi (saniye).
    pub audio_duration: f64,
    /// Beklenen dil (varsa).
    pub expected_language: Option<String>,
    /// Parça sınırları (saniye); sınır artefaktlarını kontrol için.
    pub chunk_boundaries: Vec<f64>,
    /// Sessizlik eşiği (dBFS).
    pub silence_threshold_dbfs: f32,
    /// Uzun sessizlik eşiği (saniye).
    pub long_silence_secs: f64,
}

impl Default for QualityControlInput {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            audio_duration: 0.0,
            expected_language: None,
            chunk_boundaries: Vec::new(),
            silence_threshold_dbfs: -60.0,
            long_silence_secs: 10.0,
        }
    }
}

/// Kalite kontrolünü çalıştırır.
pub fn run_quality_control(input: &QualityControlInput) -> QualityReport {
    let mut findings: Vec<QualityFinding> = Vec::new();

    check_time_codes(input, &mut findings);
    check_gaps_and_silence(input, &mut findings);
    check_overlaps(input, &mut findings);
    check_low_audio(input, &mut findings);
    check_empty_transcript(input, &mut findings);
    check_language_changes(input, &mut findings);
    check_chunk_boundaries(input, &mut findings);
    check_numeric_and_negation(input, &mut findings);
    check_proper_nouns(input, &mut findings);
    check_hallucinations(input, &mut findings);

    QualityReport {
        findings,
        completed_at: Timestamp::now(),
        segments_checked: input.segments.len(),
    }
}

fn check_time_codes(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    let mut prev_end = 0.0f64;
    for s in &input.segments {
        if s.end < s.start {
            findings.push(finding(
                QualityFindingKind::TimeCodeOverlap,
                QualitySeverity::High,
                s,
                format!(
                    "Zaman kodu tutarsız: başlangıç {:.3} > bitiş {:.3}.",
                    s.start, s.end
                ),
            ));
        }
        if s.start + 0.5 < prev_end {
            findings.push(finding(
                QualityFindingKind::TimeCodeOverlap,
                QualitySeverity::Medium,
                s,
                format!(
                    "Önceki segmentle zaman kodu çakışması ({:.3} < {:.3}).",
                    s.start, prev_end
                ),
            ));
        }
        if s.end > input.audio_duration + 0.5 && input.audio_duration > 0.0 {
            findings.push(finding(
                QualityFindingKind::TimeCodeOverlap,
                QualitySeverity::High,
                s,
                format!(
                    "Segment ses süresini aşıyor: {:.3} > {:.3}.",
                    s.end, input.audio_duration
                ),
            ));
        }
        prev_end = prev_end.max(s.end);
    }
}

fn check_gaps_and_silence(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    let mut prev_end = 0.0f64;
    for s in &input.segments {
        let gap = s.start - prev_end;
        if gap >= input.long_silence_secs {
            findings.push(QualityFinding {
                kind: QualityFindingKind::LongSilence,
                severity: QualitySeverity::Medium,
                segment_index: Some(s.index),
                start: Some(prev_end),
                end: Some(s.start),
                description: format!(
                    "Segmentler arasında {gap:.1} saniyelik uzun sessizlik var."
                ),
            });
        }
        if gap > 2.0 && gap < input.long_silence_secs {
            findings.push(QualityFinding {
                kind: QualityFindingKind::TimeCodeGap,
                severity: QualitySeverity::Low,
                segment_index: Some(s.index),
                start: Some(prev_end),
                end: Some(s.start),
                description: format!("Zaman kodu boşluğu: {gap:.1} saniye."),
            });
        }
        prev_end = s.end;
    }
    // Başta sessizlik
    if let Some(first) = input.segments.first() {
        if first.start >= input.long_silence_secs {
            findings.push(QualityFinding {
                kind: QualityFindingKind::LongSilence,
                severity: QualitySeverity::Low,
                segment_index: Some(first.index),
                start: Some(0.0),
                end: Some(first.start),
                description: format!("Kaydın başında {:.1} saniye sessizlik var.", first.start),
            });
        }
    }
}

fn check_overlaps(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    for s in &input.segments {
        if s.overlapping_speech {
            findings.push(finding(
                QualityFindingKind::OverlappingSpeech,
                QualitySeverity::High,
                s,
                "Örtüşen konuşma tespit edildi; transkript bu bölümde güvenilir olmayabilir."
                    .to_string(),
            ));
        }
    }
}

fn check_low_audio(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    for s in &input.segments {
        if let Some(level) = s.level_dbfs {
            if level < input.silence_threshold_dbfs && !s.text.trim().is_empty() {
                findings.push(finding(
                    QualityFindingKind::PossibleHallucination,
                    QualitySeverity::High,
                    s,
                    format!("Sessizlikte metin: ses düzeyi {level:.1} dBFS."),
                ));
            } else if level < -40.0 {
                findings.push(finding(
                    QualityFindingKind::VeryLowAudio,
                    QualitySeverity::Medium,
                    s,
                    format!("Çok düşük ses düzeyi: {level:.1} dBFS."),
                ));
            }
        }
    }
}

fn check_empty_transcript(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    for s in &input.segments {
        if s.has_speech == Some(true) && s.text.trim().is_empty() {
            findings.push(finding(
                QualityFindingKind::SpeechButEmptyTranscript,
                QualitySeverity::High,
                s,
                "Konuşma var ancak transkript boş.".to_string(),
            ));
        }
    }
    if !input.segments.is_empty()
        && input.audio_duration > 1.0
        && input.segments.iter().all(|s| s.text.trim().is_empty())
    {
        findings.push(QualityFinding {
            kind: QualityFindingKind::SpeechButEmptyTranscript,
            severity: QualitySeverity::High,
            segment_index: None,
            start: None,
            end: None,
            description: "Ses dosyası için hiç metin üretilmedi.".to_string(),
        });
    }
}

fn check_language_changes(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    let Some(expected) = &input.expected_language else {
        return;
    };
    for s in &input.segments {
        if let Some(lang) = &s.language {
            if lang != expected && !s.text.trim().is_empty() {
                findings.push(finding(
                    QualityFindingKind::UnexpectedLanguageChange,
                    QualitySeverity::Medium,
                    s,
                    format!("Beklenmeyen dil: {lang} (beklenen {expected})."),
                ));
            }
        }
    }
}

fn check_chunk_boundaries(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    for &boundary in &input.chunk_boundaries {
        // Sınır civarında çok yakın iki segment varsa parça sınırı artefaktı olabilir.
        let near: Vec<&QualitySegment> = input
            .segments
            .iter()
            .filter(|s| (s.start - boundary).abs() < 1.0 || (s.end - boundary).abs() < 1.0)
            .collect();
        if near.len() >= 2 {
            let texts: Vec<&str> = near.iter().map(|s| s.text.trim()).collect();
            if texts.len() >= 2 && !texts[0].is_empty() && texts[0] == texts[texts.len() - 1] {
                findings.push(QualityFinding {
                    kind: QualityFindingKind::ChunkBoundaryArtifact,
                    severity: QualitySeverity::Medium,
                    segment_index: near[0].index.into(),
                    start: Some(boundary - 1.0),
                    end: Some(boundary + 1.0),
                    description: format!(
                        "Parça sınırında ({boundary:.1} sn) tekrar veya kayıp olabilir."
                    ),
                });
            }
        }
    }
}

fn check_numeric_and_negation(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    for s in &input.segments {
        if contains_digits(&s.text) {
            findings.push(finding(
                QualityFindingKind::NumberDateAmount,
                QualitySeverity::Medium,
                s,
                "Sayı, tarih veya tutar içeriyor; doğruluğu ses üzerinden kontrol edilmelidir."
                    .to_string(),
            ));
        }
        if contains_negation(&s.text) {
            findings.push(finding(
                QualityFindingKind::Negation,
                QualitySeverity::High,
                s,
                "Olumsuzluk sözcüğü içeriyor; anlam kritik olabilir.".to_string(),
            ));
        }
    }
}

fn check_proper_nouns(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    for s in &input.segments {
        // Cümle başı dışında büyük harfle başlayan kelimeler özel isim olabilir.
        let mut words = s.text.split_whitespace();
        words.next(); // ilk kelimeyi atla (cümle başı)
        for w in words {
            let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
            if clean.chars().count() >= 2
                && clean.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                && clean.chars().skip(1).any(|c| c.is_lowercase())
            {
                findings.push(finding(
                    QualityFindingKind::DifficultProperNoun,
                    QualitySeverity::Medium,
                    s,
                    format!("Olası özel isim: \"{clean}\"; yazımı ses üzerinden doğrulanmalıdır."),
                ));
                break; // segment başına bir bulgu yeterli
            }
        }
    }
}

fn check_hallucinations(input: &QualityControlInput, findings: &mut Vec<QualityFinding>) {
    let segs: Vec<(usize, String, f64, f64, Option<f32>)> = input
        .segments
        .iter()
        .map(|s| (s.index, s.text.clone(), s.start, s.end, s.level_dbfs))
        .collect();
    for flag in hallucination::scan(&segs, input.silence_threshold_dbfs, 8.0, 3) {
        let severity = match flag.reason {
            hallucination::HallucinationReason::TextInSilence => QualitySeverity::High,
            hallucination::HallucinationReason::KnownPattern => QualitySeverity::High,
            _ => QualitySeverity::Medium,
        };
        findings.push(QualityFinding {
            kind: QualityFindingKind::PossibleHallucination,
            severity,
            segment_index: Some(flag.segment_index),
            start: None,
            end: None,
            description: flag.description,
        });
    }
}

fn finding(
    kind: QualityFindingKind,
    severity: QualitySeverity,
    s: &QualitySegment,
    description: String,
) -> QualityFinding {
    QualityFinding {
        kind,
        severity,
        segment_index: Some(s.index),
        start: Some(s.start),
        end: Some(s.end),
        description,
    }
}

fn contains_digits(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
}

fn contains_negation(text: &str) -> bool {
    let lowered = text.to_lowercase();
    // Türkçe olumsuzluk ekleri ve sözcükleri.
    const NEG: &[&str] = &[
        " değil", " yok", "olmadı", "etmedi", "yapmadı", "bilmiyor", "gelmedi", "gitmedi",
        "söylemedi", "duymadı", "görmedi", "hiçbir", "asla", "hayır",
    ];
    NEG.iter().any(|n| lowered.contains(n))
}

/// Güven göstergelerinin nasıl sunulacağını açıklar.
pub fn confidence_note() -> &'static str {
    "Model log olasılık veya güven göstergeleri, gerçek doğruluk yüzdesi \
     olarak sunulmaz. Bu göstergeler yalnızca inceleme önceliği belirlemek \
     için kullanılır. Hiçbir kalite göstergesi bilirkişi onayının yerine \
     geçmez."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(i: usize, s: f64, e: f64, t: &str) -> QualitySegment {
        QualitySegment {
            index: i,
            start: s,
            end: e,
            text: t.to_string(),
            level_dbfs: Some(-20.0),
            has_speech: Some(true),
            overlapping_speech: false,
            language: None,
        }
    }

    #[test]
    fn clean_transcript_has_few_findings() {
        let input = QualityControlInput {
            segments: vec![
                seg(0, 0.0, 2.0, "Merhaba dünya"),
                seg(1, 2.0, 4.0, "Bu bir denemedir"),
            ],
            audio_duration: 4.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert_eq!(r.segments_checked, 2);
        // Zaman kodu sorunu olmamalı.
        assert!(!r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::TimeCodeOverlap));
    }

    #[test]
    fn detects_time_code_overlap() {
        let input = QualityControlInput {
            segments: vec![seg(0, 0.0, 5.0, "a"), seg(1, 3.0, 7.0, "b")],
            audio_duration: 10.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::TimeCodeOverlap));
    }

    #[test]
    fn detects_inverted_time_code() {
        let input = QualityControlInput {
            segments: vec![seg(0, 5.0, 2.0, "a")],
            audio_duration: 10.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::TimeCodeOverlap));
    }

    #[test]
    fn detects_segment_beyond_audio_duration() {
        let input = QualityControlInput {
            segments: vec![seg(0, 0.0, 100.0, "a")],
            audio_duration: 5.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.description.contains("ses süresini aşıyor")));
    }

    #[test]
    fn detects_long_silence() {
        let input = QualityControlInput {
            segments: vec![seg(0, 0.0, 2.0, "a"), seg(1, 20.0, 22.0, "b")],
            audio_duration: 22.0,
            long_silence_secs: 10.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::LongSilence));
    }

    #[test]
    fn detects_overlapping_speech() {
        let mut s = seg(0, 0.0, 2.0, "a");
        s.overlapping_speech = true;
        let input = QualityControlInput {
            segments: vec![s],
            audio_duration: 2.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::OverlappingSpeech));
    }

    #[test]
    fn detects_speech_but_empty_transcript() {
        let mut s = seg(0, 0.0, 2.0, "   ");
        s.has_speech = Some(true);
        let input = QualityControlInput {
            segments: vec![s],
            audio_duration: 2.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::SpeechButEmptyTranscript));
    }

    #[test]
    fn detects_text_in_silence_as_high_severity() {
        let mut s = seg(0, 0.0, 2.0, "bir şeyler");
        s.level_dbfs = Some(-80.0);
        let input = QualityControlInput {
            segments: vec![s],
            audio_duration: 2.0,
            silence_threshold_dbfs: -60.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        let f = r
            .findings
            .iter()
            .find(|f| f.description.contains("Sessizlikte metin"));
        assert!(f.is_some());
        assert_eq!(f.unwrap().severity, QualitySeverity::High);
    }

    #[test]
    fn detects_negation_as_high_severity() {
        let input = QualityControlInput {
            segments: vec![seg(0, 0.0, 2.0, "Bunu ben yapmadım, olmadı.")],
            audio_duration: 2.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::Negation && f.severity == QualitySeverity::High));
    }

    #[test]
    fn detects_numbers() {
        let input = QualityControlInput {
            segments: vec![seg(0, 0.0, 2.0, "2025 yılında 1500 TL ödedim.")],
            audio_duration: 2.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::NumberDateAmount));
    }

    #[test]
    fn detects_unexpected_language() {
        let mut s = seg(0, 0.0, 2.0, "hello world");
        s.language = Some("en".to_string());
        let input = QualityControlInput {
            segments: vec![s],
            audio_duration: 2.0,
            expected_language: Some("tr".to_string()),
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::UnexpectedLanguageChange));
    }

    #[test]
    fn detects_proper_nouns() {
        let input = QualityControlInput {
            segments: vec![seg(0, 0.0, 2.0, "Ahmet ile Mehmet konuştu.")],
            audio_duration: 2.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r
            .findings
            .iter()
            .any(|f| f.kind == QualityFindingKind::DifficultProperNoun));
    }

    #[test]
    fn report_prioritises_and_flags_review() {
        let input = QualityControlInput {
            segments: vec![seg(0, 0.0, 2.0, "Bunu yapmadım, 2025 yılında olmadı.")],
            audio_duration: 2.0,
            ..Default::default()
        };
        let r = run_quality_control(&input);
        assert!(r.needs_expert_review());
        let p = r.review_priorities();
        assert!(!p.is_empty());
        // Yüksek önemdekiler önce gelmeli.
        assert_eq!(p[0].severity, QualitySeverity::High);
    }

    #[test]
    fn confidence_note_is_honest() {
        let n = confidence_note();
        assert!(n.contains("gerçek doğruluk yüzdesi"));
        assert!(n.contains("bilirkişi onayının yerine"));
    }
}
