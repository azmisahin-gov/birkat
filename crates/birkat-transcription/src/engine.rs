//! Transkripsiyon motoru soyutlaması (9).
//!
//! Motor, ses dosyasını metne dönüştürür. Somut uygulamalar:
//!
//! * [`whisper_candle`](crate::whisper_candle) — Whisper large-v3-turbo /
//!   Candle (yerel, çevrimdışı).
//! * Test amaçlı sahte motorlar (bkz. [`engine::MockEngine`]).
//!
//! ## Kelime düzeyi zaman kodu
//!
//! [`TranscriptionOutput::word_timestamps`], modelin gerçekten kelime düzeyi
//! zaman kodu üretip üretmediğini açıkça bildirir. Üretmiyorsa uygulama
//! sahte kelime zamanı üretmez; yalnızca segment zaman kodu gösterilir.

use birkat_core::audio::InferenceParams;
use birkat_core::error::Result;
use birkat_core::ids::AudioId;
use serde::{Deserialize, Serialize};

/// Kelime düzeyi zaman kodu desteği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WordTimestamps {
    /// Motor kelime düzeyi zaman kodu üretir.
    Native,
    /// Motor üretmez; yalnızca segment düzeyi zaman kodu vardır.
    SegmentOnly,
}

/// Motorun yetenekleri.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCapabilities {
    /// Motor adı.
    pub name: String,
    /// Sürüm.
    pub version: String,
    /// Kelime zaman kodu desteği.
    pub word_timestamps: WordTimestamps,
    /// Otomatik dil tespiti.
    pub language_detection: bool,
    /// Desteklenen diller (boşsa çok dilli).
    pub languages: Vec<String>,
    /// Çalıştığı cihazlar.
    pub devices: Vec<String>,
    /// Çevrimdışı çalışabilir mi?
    pub offline: bool,
}

/// Transkripsiyon isteği.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRequest {
    /// Ses varlığı.
    pub audio: AudioId,
    /// Dosya yolu.
    pub path: String,
    /// Kaynak dil (`None` = otomatik).
    pub language: Option<String>,
    /// Çıkarım parametreleri.
    pub params: InferenceParams,
    /// Parça uzunluğu (saniye).
    pub chunk_secs: f32,
    /// Parçalar arası örtüşme (saniye).
    pub overlap_secs: f32,
    /// Konuşmacı ayrımı istensin mi?
    pub diarization: bool,
    /// Kelime zaman kodu istensin mi?
    pub word_timestamps: bool,
}

/// Motorun ürettiği ham segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegmentDraft {
    /// Başlangıç (saniye).
    pub start: f64,
    /// Bitiş (saniye).
    pub end: f64,
    /// Metin.
    pub text: String,
    /// Kelime zaman kodları (motor üretiyorsa).
    pub words: Vec<(String, f64, f64)>,
    /// Konuşmacı etiketi (diarization açıksa).
    pub speaker: Option<u32>,
    /// Model göstergesi (olasılık); doğruluk yüzdesi değildir.
    pub model_indicator: Option<f32>,
    /// Ses düzeyi (dBFS).
    pub level_dbfs: Option<f32>,
}

/// Motor çıktısı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionOutput {
    /// Segmentler.
    pub segments: Vec<TranscriptSegmentDraft>,
    /// Tespit edilen dil.
    pub language: Option<String>,
    /// Dil tespit göstergesi (doğruluk yüzdesi değildir).
    pub language_indicator: Option<f32>,
    /// Kelime zaman kodu desteği.
    pub word_timestamps: WordTimestamps,
    /// İşlenen parça sayısı.
    pub chunks: u32,
    /// İşlem süresi (saniye).
    pub elapsed_secs: f64,
    /// Kullanılan cihaz (CPU/CUDA).
    pub device: String,
    /// Uyarılar.
    pub warnings: Vec<String>,
}

impl TranscriptionOutput {
    /// Tüm metin.
    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Toplam süre.
    pub fn duration_secs(&self) -> f64 {
        self.segments
            .iter()
            .map(|s| s.end)
            .fold(0.0f64, f64::max)
    }
}

/// Transkripsiyon motoru arayüzü.
pub trait TranscriptionEngine: Send + Sync {
    /// Motor yetenekleri.
    fn capabilities(&self) -> EngineCapabilities;

    /// Transkripsiyon yürütür.
    ///
    /// `progress` geri çağrısı 0.0 - 1.0 arasında ilerleme bildirir; `false`
    /// dönerse işlem iptal edilir.
    fn transcribe(
        &self,
        request: &TranscriptionRequest,
        progress: &mut dyn FnMut(f64) -> bool,
    ) -> Result<TranscriptionOutput>;
}

/// Test amaçlı basit motor.
///
/// Gerçek çıkarım yapmaz; kuyruk ve kalite kontrolü testleri için
/// deterministik çıktı üretir.
pub struct MockEngine {
    /// Üretilecek segment metinleri.
    pub segments: Vec<(f64, f64, String)>,
    /// Dil.
    pub language: Option<String>,
    /// Yapay gecikme (saniye); testte 0 kullanılır.
    pub delay_secs: f64,
}

impl Default for MockEngine {
    fn default() -> Self {
        Self {
            segments: vec![
                (0.0, 2.0, "Merhaba, bu bir deneme kaydıdır.".to_string()),
                (2.0, 4.5, "Ses kaydı metne dönüştürülmektedir.".to_string()),
            ],
            language: Some("tr".to_string()),
            delay_secs: 0.0,
        }
    }
}

impl TranscriptionEngine for MockEngine {
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            name: "MockEngine".to_string(),
            version: "0.0.0".to_string(),
            word_timestamps: WordTimestamps::SegmentOnly,
            language_detection: true,
            languages: vec!["tr".to_string()],
            devices: vec!["cpu".to_string()],
            offline: true,
        }
    }

    fn transcribe(
        &self,
        request: &TranscriptionRequest,
        progress: &mut dyn FnMut(f64) -> bool,
    ) -> Result<TranscriptionOutput> {
        let total = self.segments.len().max(1);
        for (i, _) in self.segments.iter().enumerate() {
            let p = (i + 1) as f64 / total as f64;
            if !progress(p) {
                return Err(birkat_core::error::CoreError::Validation(
                    "İşlem kullanıcı tarafından iptal edildi.".to_string(),
                ));
            }
        }
        Ok(TranscriptionOutput {
            segments: self
                .segments
                .iter()
                .map(|(s, e, t)| TranscriptSegmentDraft {
                    start: *s,
                    end: *e,
                    text: t.clone(),
                    words: Vec::new(),
                    speaker: None,
                    model_indicator: Some(0.9),
                    level_dbfs: Some(-18.0),
                })
                .collect(),
            language: request.language.clone().or_else(|| self.language.clone()),
            language_indicator: Some(0.98),
            word_timestamps: WordTimestamps::SegmentOnly,
            chunks: 1,
            elapsed_secs: self.delay_secs,
            device: "cpu".to_string(),
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> TranscriptionRequest {
        TranscriptionRequest {
            audio: AudioId::new(),
            path: "/tmp/a.wav".to_string(),
            language: None,
            params: InferenceParams::default(),
            chunk_secs: 30.0,
            overlap_secs: 2.0,
            diarization: false,
            word_timestamps: false,
        }
    }

    #[test]
    fn mock_engine_produces_segments() {
        let e = MockEngine::default();
        let mut progress = |_p: f64| true;
        let out = e.transcribe(&request(), &mut progress).unwrap();
        assert_eq!(out.segments.len(), 2);
        assert_eq!(out.language.as_deref(), Some("tr"));
        assert!(out.full_text().contains("Merhaba"));
    }

    #[test]
    fn mock_engine_honours_cancellation() {
        let e = MockEngine::default();
        let mut progress = |_p: f64| false;
        assert!(e.transcribe(&request(), &mut progress).is_err());
    }

    #[test]
    fn capabilities_are_described() {
        let c = MockEngine::default().capabilities();
        assert!(c.offline);
        assert!(c.language_detection);
        assert_eq!(c.word_timestamps, WordTimestamps::SegmentOnly);
    }

    #[test]
    fn duration_is_max_end() {
        let out = TranscriptionOutput {
            segments: vec![
                TranscriptSegmentDraft {
                    start: 0.0,
                    end: 1.0,
                    text: "a".into(),
                    words: vec![],
                    speaker: None,
                    model_indicator: None,
                    level_dbfs: None,
                },
                TranscriptSegmentDraft {
                    start: 1.0,
                    end: 3.5,
                    text: "b".into(),
                    words: vec![],
                    speaker: None,
                    model_indicator: None,
                    level_dbfs: None,
                },
            ],
            language: None,
            language_indicator: None,
            word_timestamps: WordTimestamps::SegmentOnly,
            chunks: 1,
            elapsed_secs: 0.0,
            device: "cpu".into(),
            warnings: vec![],
        };
        assert_eq!(out.duration_secs(), 3.5);
    }
}
