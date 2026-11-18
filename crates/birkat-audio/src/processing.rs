//! Türetilmiş ses işlemleri (8).
//!
//! Gürültü azaltma, filtreleme, normalizasyon, kanal birleştirme veya başka
//! bir ses işlemi yapılırsa türetilmiş dosya **ayrı tutulur**; işlem
//! parametreleri kaydedilir ve işlenmiş sesin orijinal kayıt olduğu izlenimi
//! verilmez.
//!
//! Orijinal ses hiçbir koşulda değiştirilmez.

use crate::decode::PcmBuffer;
use serde::{Deserialize, Serialize};

/// Uygulanan işlemin parametreleri.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type")]
pub enum ProcessingStep {
    /// Tepe normalizasyonu.
    PeakNormalize {
        /// Hedef tepe genlik (0.0 - 1.0).
        target_peak: f32,
    },
    /// Basit tek kutuplu alçak geçiren filtre.
    LowPass {
        /// Kesim frekansı (Hz).
        cutoff_hz: f32,
        /// Örnekleme frekansı (Hz).
        sample_rate: u32,
    },
    /// Basit gürültü kapısı.
    NoiseGate {
        /// Eşik (genlik).
        threshold: f32,
        /// Yumuşatma katsayısı (0.0 - 1.0).
        smoothing: f32,
    },
    /// Kanal birleştirme (mono).
    MixToMono,
}

impl ProcessingStep {
    /// İnsan okuyabilir Türkçe açıklama.
    pub fn describe(&self) -> String {
        match self {
            ProcessingStep::PeakNormalize { target_peak } => {
                format!("Tepe normalizasyonu (hedef tepe {target_peak:.2})")
            }
            ProcessingStep::LowPass {
                cutoff_hz,
                sample_rate,
            } => format!("Alçak geçiren filtre ({cutoff_hz:.0} Hz, {sample_rate} Hz)"),
            ProcessingStep::NoiseGate {
                threshold,
                smoothing,
            } => format!("Gürültü kapısı (eşik {threshold:.4}, yumuşatma {smoothing:.2})"),
            ProcessingStep::MixToMono => "Kanalları mono'ya birleştirme".to_string(),
        }
    }
}

/// Türetilmiş ses kaydı.
#[derive(Debug, Clone)]
pub struct DerivedAudio {
    /// İşlenmiş PCM.
    pub pcm: PcmBuffer,
    /// Uygulanan adımlar.
    pub steps: Vec<ProcessingStep>,
    /// Kaynak dosya kimliği (orijinalin yerine geçmez).
    pub source_note: String,
}

impl DerivedAudio {
    /// Bu çıktının orijinal kayıt olmadığını belirten etiket.
    pub fn label(&self) -> String {
        format!(
            "Türetilmiş ses (orijinal kayıt değildir). Uygulanan işlemler: {}",
            self.steps
                .iter()
                .map(|s| s.describe())
                .collect::<Vec<_>>()
                .join("; ")
        )
    }

    /// İşlem zincirini uygular.
    pub fn apply(mut pcm: PcmBuffer, steps: &[ProcessingStep]) -> DerivedAudio {
        for step in steps {
            pcm = apply_step(pcm, step);
        }
        DerivedAudio {
            pcm,
            steps: steps.to_vec(),
            source_note: "Orijinal ses değiştirilmedi; bu çıktı türetilmiştir."
                .to_string(),
        }
    }
}

fn apply_step(pcm: PcmBuffer, step: &ProcessingStep) -> PcmBuffer {
    match step {
        ProcessingStep::PeakNormalize { target_peak } => peak_normalize(pcm, *target_peak),
        ProcessingStep::LowPass {
            cutoff_hz,
            sample_rate,
        } => low_pass(pcm, *cutoff_hz, *sample_rate),
        ProcessingStep::NoiseGate {
            threshold,
            smoothing,
        } => noise_gate(pcm, *threshold, *smoothing),
        ProcessingStep::MixToMono => {
            let mono = pcm.to_mono();
            PcmBuffer {
                sample_rate: pcm.sample_rate,
                channels: 1,
                samples: vec![mono],
            }
        }
    }
}

/// Tepe normalizasyonu uygular.
pub fn peak_normalize(pcm: PcmBuffer, target_peak: f32) -> PcmBuffer {
    let peak = pcm
        .samples
        .iter()
        .flat_map(|c| c.iter())
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    if peak <= 0.0 {
        return pcm;
    }
    let gain = target_peak / peak;
    PcmBuffer {
        sample_rate: pcm.sample_rate,
        channels: pcm.channels,
        samples: pcm
            .samples
            .into_iter()
            .map(|c| c.into_iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect())
            .collect(),
    }
}

/// Tek kutuplu alçak geçiren filtre uygular.
pub fn low_pass(pcm: PcmBuffer, cutoff_hz: f32, sample_rate: u32) -> PcmBuffer {
    if sample_rate == 0 || cutoff_hz <= 0.0 {
        return pcm;
    }
    let dt = 1.0 / sample_rate as f32;
    let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
    let alpha = dt / (rc + dt);
    let samples = pcm
        .samples
        .into_iter()
        .map(|ch| {
            let mut prev = 0.0f32;
            ch.into_iter()
                .map(|s| {
                    prev += alpha * (s - prev);
                    prev
                })
                .collect()
        })
        .collect();
    PcmBuffer {
        sample_rate: pcm.sample_rate,
        channels: pcm.channels,
        samples,
    }
}

/// Gürültü kapısı uygular.
pub fn noise_gate(pcm: PcmBuffer, threshold: f32, smoothing: f32) -> PcmBuffer {
    let smoothing = smoothing.clamp(0.0, 0.99);
    let samples = pcm
        .samples
        .into_iter()
        .map(|ch| {
            let mut gain = 1.0f32;
            ch.into_iter()
                .map(|s| {
                    let target = if s.abs() < threshold { 0.0 } else { 1.0 };
                    gain = gain * smoothing + target * (1.0 - smoothing);
                    s * gain
                })
                .collect()
        })
        .collect();
    PcmBuffer {
        sample_rate: pcm.sample_rate,
        channels: pcm.channels,
        samples,
    }
}

/// Türetilmiş sesin rapor içinde nasıl sunulacağını açıklar.
pub fn derived_audio_note() -> &'static str {
    "İşlenmiş (türetilmiş) ses dosyaları orijinal kayıttan ayrı tutulur. \
     İşlem parametreleri kaydedilir. İşlenmiş sesin orijinal kayıt olduğu \
     izlenimi verilmez; raporda orijinal dosya kimliği, hash'i ve zaman \
     referansları esas alınır."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(ch: Vec<Vec<f32>>, sr: u32) -> PcmBuffer {
        PcmBuffer {
            sample_rate: sr,
            channels: ch.len() as u16,
            samples: ch,
        }
    }

    #[test]
    fn peak_normalize_scales_to_target() {
        let p = pcm(vec![vec![0.25, -0.5, 0.1]], 1000);
        let out = peak_normalize(p, 1.0);
        let peak = out.samples[0].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!((peak - 1.0).abs() < 1e-6);
    }

    #[test]
    fn peak_normalize_of_silence_is_unchanged() {
        let p = pcm(vec![vec![0.0, 0.0]], 1000);
        let out = peak_normalize(p.clone(), 1.0);
        assert_eq!(out.samples, p.samples);
    }

    #[test]
    fn low_pass_reduces_high_frequency_energy() {
        // Nyquist'e yakın alternatif +1/-1 sinyal alçak geçirenle sönmeli.
        let sr = 8000u32;
        let ch: Vec<f32> = (0..200).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let p = pcm(vec![ch], sr);
        let out = low_pass(p, 200.0, sr);
        let last = out.samples[0][199].abs();
        assert!(last < 0.2, "yüksek frekans sönmeliydi, son={last}");
    }

    #[test]
    fn noise_gate_silences_below_threshold() {
        let p = pcm(vec![vec![0.001, 0.001, 0.9, 0.001]], 1000);
        let out = noise_gate(p, 0.01, 0.0);
        assert!(out.samples[0][0].abs() < 1e-9);
        assert!(out.samples[0][2].abs() > 0.5);
    }

    #[test]
    fn mix_to_mono_produces_one_channel() {
        let p = pcm(vec![vec![0.5; 10], vec![0.5; 10]], 1000);
        let d = DerivedAudio::apply(p, &[ProcessingStep::MixToMono]);
        assert_eq!(d.pcm.channels, 1);
    }

    #[test]
    fn derived_label_marks_not_original() {
        let p = pcm(vec![vec![0.1; 10]], 1000);
        let d = DerivedAudio::apply(
            p,
            &[ProcessingStep::PeakNormalize { target_peak: 0.9 }],
        );
        let l = d.label();
        assert!(l.contains("orijinal kayıt değildir"));
        assert!(l.contains("Tepe normalizasyonu"));
    }

    #[test]
    fn steps_are_recorded() {
        let p = pcm(vec![vec![0.1; 10]], 8000);
        let steps = vec![
            ProcessingStep::LowPass {
                cutoff_hz: 3000.0,
                sample_rate: 8000,
            },
            ProcessingStep::PeakNormalize { target_peak: 0.95 },
        ];
        let d = DerivedAudio::apply(p, &steps);
        assert_eq!(d.steps.len(), 2);
        assert!(d.steps[0].describe().contains("3000 Hz"));
    }

    #[test]
    fn derived_note_explains_separation() {
        assert!(derived_audio_note().contains("ayrı tutulur"));
    }
}
