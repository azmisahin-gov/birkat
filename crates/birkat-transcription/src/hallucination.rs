//! Olası halüsinasyon ve tekrar tespiti (11).
//!
//! Whisper ailesi modeller sessizlikte veya düşük sesli bölümlerde metin
//! üretebilir (halüsinasyon). Bu modül, böyle bölümleri ve aşırı tekrarları
//! işaretler. **Hiçbir bulgu bilirkişi onayının yerine geçmez.**

use serde::{Deserialize, Serialize};

/// Halüsinasyon şüphesi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationFlag {
    /// Segment indeksi.
    pub segment_index: usize,
    /// Neden.
    pub reason: HallucinationReason,
    /// Açıklama.
    pub description: String,
}

/// Halüsinasyon nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HallucinationReason {
    /// Sessizlikte metin üretilmiş.
    TextInSilence,
    /// Aynı metin çok kez tekrarlanmış.
    RepeatedText,
    /// Çok kısa segmentte çok uzun metin.
    TooMuchTextForSegment,
    /// Bilinen halüsinasyon kalıbı (ör. altyazı/teşekkür kalıpları).
    KnownPattern,
    /// Konuşma dışı metin (ör. müzik/sessizlik işaretleri).
    NonSpeechText,
}

impl HallucinationReason {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            HallucinationReason::TextInSilence => "Sessizlikte metin üretilmiş",
            HallucinationReason::RepeatedText => "Aynı metin çok kez tekrarlanmış",
            HallucinationReason::TooMuchTextForSegment => "Kısa segmentte aşırı uzun metin",
            HallucinationReason::KnownPattern => "Bilinen halüsinasyon kalıbı",
            HallucinationReason::NonSpeechText => "Konuşma dışı metin",
        }
    }
}

/// Bilinen halüsinasyon kalıpları (Türkçe altyazı/eğitim verisi artıkları).
pub const KNOWN_HALLUCINATION_PATTERNS: &[&str] = &[
    "altyazı",
    "abone ol",
    "kanalıma abone",
    "izlediğiniz için teşekkür",
    "teşekkürler",
    "alt yazı m.k",
    "www.",
    "http",
    "çeviri:",
    "translated by",
    "subtitles by",
    "amara.org",
];

/// Bir segmentin sessizlikte metin üretip üretmediğini değerlendirir.
///
/// `level_dbfs` çok düşükse ve metin varsa şüphe işaretlenir.
pub fn detect_silence_hallucination(
    segment_index: usize,
    text: &str,
    level_dbfs: Option<f32>,
    silence_threshold_dbfs: f32,
) -> Option<HallucinationFlag> {
    if text.trim().is_empty() {
        return None;
    }
    match level_dbfs {
        Some(level) if level < silence_threshold_dbfs => Some(HallucinationFlag {
            segment_index,
            reason: HallucinationReason::TextInSilence,
            description: format!(
                "Ses düzeyi {level:.1} dBFS (eşik {silence_threshold_dbfs:.1}); \
                 sessizlikte metin üretilmiş olabilir."
            ),
        }),
        _ => None,
    }
}

/// Bilinen halüsinasyon kalıplarını arar.
pub fn detect_known_pattern(segment_index: usize, text: &str) -> Option<HallucinationFlag> {
    let lowered = text.to_lowercase();
    for p in KNOWN_HALLUCINATION_PATTERNS {
        if lowered.contains(p) {
            return Some(HallucinationFlag {
                segment_index,
                reason: HallucinationReason::KnownPattern,
                description: format!("Bilinen halüsinasyon kalıbı bulundu: \"{p}\""),
            });
        }
    }
    None
}

/// Aşırı metin yoğunluğunu değerlendirir.
///
/// Bir segmentte saniye başına çok fazla kelime varsa (ör. 8+ kelime/saniye)
/// şüphe işaretlenir.
pub fn detect_text_density(
    segment_index: usize,
    text: &str,
    duration_secs: f64,
    max_words_per_sec: f64,
) -> Option<HallucinationFlag> {
    if duration_secs <= 0.0 {
        return None;
    }
    let words = text.split_whitespace().count() as f64;
    let density = words / duration_secs;
    if density > max_words_per_sec {
        Some(HallucinationFlag {
            segment_index,
            reason: HallucinationReason::TooMuchTextForSegment,
            description: format!(
                "Saniyede {density:.1} kelime (eşik {max_words_per_sec:.1}); \
                 metin yoğunluğu olağan dışı."
            ),
        })
    } else {
        None
    }
}

/// Ardışık tekrarları bulur.
///
/// Aynı metin `min_repeats` kez üst üste geliyorsa işaretlenir.
pub fn detect_repetitions(texts: &[String], min_repeats: usize) -> Vec<HallucinationFlag> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < texts.len() {
        let mut j = i + 1;
        while j < texts.len() && texts[j].trim() == texts[i].trim() && !texts[i].trim().is_empty() {
            j += 1;
        }
        let count = j - i;
        if count >= min_repeats {
            for k in i..j {
                out.push(HallucinationFlag {
                    segment_index: k,
                    reason: HallucinationReason::RepeatedText,
                    description: format!("Aynı metin {count} kez üst üste tekrarlandı."),
                });
            }
        }
        i = j;
    }
    out
}

/// Tüm halüsinasyon kontrollerini çalıştırır.
pub fn scan(
    segments: &[(usize, String, f64, f64, Option<f32>)],
    silence_threshold_dbfs: f32,
    max_words_per_sec: f64,
    min_repeats: usize,
) -> Vec<HallucinationFlag> {
    let mut flags = Vec::new();
    let mut texts = Vec::new();

    for (idx, text, start, end, level) in segments {
        texts.push(text.clone());
        if let Some(f) = detect_silence_hallucination(*idx, text, *level, silence_threshold_dbfs) {
            flags.push(f);
        }
        if let Some(f) = detect_known_pattern(*idx, text) {
            flags.push(f);
        }
        if let Some(f) = detect_text_density(*idx, text, (end - start).max(0.0), max_words_per_sec)
        {
            flags.push(f);
        }
    }

    flags.extend(detect_repetitions(&texts, min_repeats));
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_text_in_silence() {
        let f = detect_silence_hallucination(0, "merhaba", Some(-70.0), -60.0);
        assert!(f.is_some());
        assert_eq!(f.unwrap().reason, HallucinationReason::TextInSilence);
    }

    #[test]
    fn normal_level_is_not_flagged() {
        assert!(detect_silence_hallucination(0, "merhaba", Some(-18.0), -60.0).is_none());
    }

    #[test]
    fn empty_text_is_not_flagged() {
        assert!(detect_silence_hallucination(0, "   ", Some(-90.0), -60.0).is_none());
    }

    #[test]
    fn detects_known_patterns() {
        let f = detect_known_pattern(0, "Altyazı M.K.").unwrap();
        assert_eq!(f.reason, HallucinationReason::KnownPattern);
        assert!(detect_known_pattern(0, "normal konuşma").is_none());
    }

    #[test]
    fn detects_excessive_density() {
        let text = "bir iki üç dört beş altı yedi sekiz dokuz on";
        let f = detect_text_density(0, text, 1.0, 8.0).unwrap();
        assert_eq!(f.reason, HallucinationReason::TooMuchTextForSegment);
        assert!(detect_text_density(0, "kısa metin", 5.0, 8.0).is_none());
    }

    #[test]
    fn detects_repeated_segments() {
        let texts = vec![
            "teşekkürler".to_string(),
            "teşekkürler".to_string(),
            "teşekkürler".to_string(),
            "farklı".to_string(),
        ];
        let flags = detect_repetitions(&texts, 3);
        assert_eq!(flags.len(), 3);
        assert!(flags
            .iter()
            .all(|f| f.reason == HallucinationReason::RepeatedText));
    }

    #[test]
    fn short_repetition_below_threshold_is_not_flagged() {
        let texts = vec!["a".to_string(), "a".to_string(), "b".to_string()];
        assert!(detect_repetitions(&texts, 3).is_empty());
    }

    #[test]
    fn empty_repetitions_are_ignored() {
        let texts = vec!["".to_string(), "".to_string(), "".to_string()];
        assert!(detect_repetitions(&texts, 2).is_empty());
    }

    #[test]
    fn scan_combines_all_checks() {
        let segs = vec![
            (0usize, "altyazı".to_string(), 0.0, 1.0, Some(-70.0)),
            (1usize, "normal".to_string(), 1.0, 3.0, Some(-20.0)),
        ];
        let flags = scan(&segs, -60.0, 8.0, 3);
        assert!(flags
            .iter()
            .any(|f| f.reason == HallucinationReason::TextInSilence));
        assert!(flags
            .iter()
            .any(|f| f.reason == HallucinationReason::KnownPattern));
    }
}
