//! Yerel Whisper (Candle) transkripsiyon motoru (9.1, 9.2).
//!
//! Ana model Whisper large-v3-turbo'dur ve çıkarım yerel makinede yapılır.
//! Model servislerine, bulut API'lerine veya ücretli çevrimiçi transkripsiyon
//! servislerine ihtiyaç duyulmaz.
//!
//! ## CPU/GPU
//!
//! CPU üzerinde çalışmak zorundadır. GPU hızlandırması derleme zamanında
//! etkinleştirildiyse (candle `cuda`/`metal` özellikleri) otomatik kullanılır;
//! başlatılamazsa işlem kaybedilmeden CPU'ya geçilir ve bu durum etikete ve
//! işlem kaydına yansıtılır.
//!
//! ## Mel filtresi
//!
//! Whisper, resmî `mel_filters.npz` dosyasındaki mel filtre bankasını
//! kullanır. Bu motor, filtreleri sırasıyla şuralardan yükler:
//!
//! 1. Model paketindeki `melfilters.bytes` (resmî filtreler; önerilen).
//! 2. Filtre dosyası yoksa analitik olarak hesaplanan mel filtre bankası.
//!
//! İkinci yol, resmî filtrelerle sayısal olarak çok yakın sonuç verir ancak
//! birebir aynı olmayabilir; bu nedenle resmî filtre dosyasının model
//! paketinde bulunması önerilir (bkz. [`MelFilterSource`]).
//!
//! ## Dürüstlük sınırları
//!
//! * Bu motor kelime düzeyi zaman kodu üretmez; segment düzeyi zaman kodu
//!   üretir ([`WordTimestamps::SegmentOnly`]).
//! * Windows GPU performansı gerçek sistemlerde test edilmeden GPU desteği
//!   garanti edilmiş gibi sunulmaz.

use crate::engine::{
    EngineCapabilities, TranscriptSegmentDraft, TranscriptionEngine, TranscriptionOutput,
    TranscriptionRequest, WordTimestamps,
};
use birkat_core::error::{CoreError, Result};
use candle_core::{DType, Device, IndexOp, Tensor, D};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, audio, Config};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

/// Mel filtresi kaynağı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MelFilterSource {
    /// Model paketindeki resmî filtre dosyası.
    OfficialFile(PathBuf),
    /// Analitik olarak hesaplanan filtre bankası.
    Analytic,
}

/// Whisper/Candle motoru.
pub struct WhisperCandleEngine {
    config: Config,
    tokenizer: Tokenizer,
    model_path: PathBuf,
    mel_filters: Vec<f32>,
    mel_source: MelFilterSource,
    device: Device,
    device_label: String,
    model_name: String,
    model_version: String,
}

impl WhisperCandleEngine {
    /// Motoru başlatır.
    ///
    /// Model dosyaları kurulum paketinde bulunmalıdır; internetten
    /// indirilmez.
    pub fn load(
        model_path: impl AsRef<Path>,
        config_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        mel_filters_path: Option<&Path>,
        model_name: impl Into<String>,
        model_version: impl Into<String>,
        prefer_gpu: bool,
    ) -> Result<Self> {
        let config_raw: Config = serde_json::from_str(
            &std::fs::read_to_string(config_path.as_ref())
                .map_err(|e| CoreError::Data(format!("Model yapılandırması okunamadı: {e}")))?,
        )
        .map_err(|e| CoreError::Data(format!("Model yapılandırması çözülemedi: {e}")))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| CoreError::Data(format!("Tokenizer yüklenemedi: {e}")))?;

        let (mel_filters, mel_source) = match mel_filters_path {
            Some(p) if p.exists() => {
                let bytes = std::fs::read(p)
                    .map_err(|e| CoreError::Data(format!("Mel filtresi okunamadı: {e}")))?;
                (
                    bytes_to_f32(&bytes),
                    MelFilterSource::OfficialFile(p.to_path_buf()),
                )
            }
            _ => (
                analytic_mel_filters(config_raw.num_mel_bins),
                MelFilterSource::Analytic,
            ),
        };

        let (device, device_label) = select_device(prefer_gpu);

        Ok(Self {
            config: config_raw,
            tokenizer,
            model_path: model_path.as_ref().to_path_buf(),
            mel_filters,
            mel_source,
            device,
            device_label,
            model_name: model_name.into(),
            model_version: model_version.into(),
        })
    }

    /// Kullanılan cihaz etiketi.
    pub fn device_label(&self) -> &str {
        &self.device_label
    }

    /// Mel filtresi kaynağı.
    pub fn mel_filter_source(&self) -> &MelFilterSource {
        &self.mel_source
    }

    /// Model ağırlıklarını yükler.
    fn build_model(&self) -> Result<m::model::Whisper> {
        let data = std::fs::read(&self.model_path)
            .map_err(|e| CoreError::Data(format!("Model ağırlıkları okunamadı: {e}")))?;
        let vb = VarBuilder::from_buffered_safetensors(data, DType::F32, &self.device)
            .map_err(|e| CoreError::Data(format!("Model ağırlıkları yüklenemedi: {e}")))?;
        m::model::Whisper::load(&vb, self.config.clone())
            .map_err(|e| CoreError::Data(format!("Whisper modeli kurulamadı: {e}")))
    }

    /// Belirteç kimliğini bulur.
    fn token_id(&self, token: &str) -> Result<u32> {
        self.tokenizer
            .token_to_id(token)
            .ok_or_else(|| CoreError::Data(format!("Belirteç bulunamadı: {token}")))
    }

    /// Tek bir mel parçasını çözer (açgözlü üretim).
    fn decode_chunk(
        &self,
        model: &mut m::model::Whisper,
        mel: &Tensor,
        language: Option<&str>,
    ) -> Result<String> {
        let audio_features = model
            .encoder
            .forward(mel, true)
            .map_err(|e| CoreError::Data(format!("Kodlayıcı hatası: {e}")))?;

        let sot = self.token_id(m::SOT_TOKEN)?;
        let transcribe = self.token_id(m::TRANSCRIBE_TOKEN)?;
        let no_ts = self.token_id(m::NO_TIMESTAMPS_TOKEN)?;
        let eot = self.token_id(m::EOT_TOKEN)?;

        let mut tokens: Vec<u32> = vec![sot];
        if let Some(lang) = language {
            if let Ok(t) = self.token_id(&format!("<|{lang}|>")) {
                tokens.push(t);
            }
        }
        tokens.push(transcribe);
        tokens.push(no_ts);

        let mut out_ids: Vec<u32> = Vec::new();
        // İlk adımda istem belirteçlerinin tamamı beslenir ve KV önbelleği
        // kurulur. Sonraki adımlarda yalnızca son belirteç beslenir; önbellek
        // sayesinde önceki belirteçler yeniden hesaplanmaz. Bu, çıkarımı
        // belirgin biçimde hızlandırır.
        let mut ilk_adim = true;
        for _ in 0..224 {
            let input = if ilk_adim {
                Tensor::new(tokens.as_slice(), &self.device)
                    .map_err(|e| CoreError::Data(format!("Belirteç tensörü hatası: {e}")))?
                    .unsqueeze(0)
                    .map_err(|e| CoreError::Data(format!("Tensör hatası: {e}")))?
            } else {
                let son = *tokens
                    .last()
                    .ok_or_else(|| CoreError::Data("Belirteç listesi boş.".to_string()))?;
                Tensor::new(&[son], &self.device)
                    .map_err(|e| CoreError::Data(format!("Belirteç tensörü hatası: {e}")))?
                    .unsqueeze(0)
                    .map_err(|e| CoreError::Data(format!("Tensör hatası: {e}")))?
            };
            // Kodçözücü çıktısı **gizli durumdur**; sözcük dağarcığı
            // logitleri için `final_linear` uygulanmalıdır. Doğrudan
            // argmax almak tek harflik çıktı üretir.
            let gizli = model
                .decoder
                .forward(&input, &audio_features, ilk_adim)
                .map_err(|e| CoreError::Data(format!("Kodçözücü hatası: {e}")))?;
            ilk_adim = false;
            let seq = gizli
                .dim(1)
                .map_err(|e| CoreError::Data(format!("Tensör hatası: {e}")))?;
            let logits = model
                .decoder
                .final_linear(
                    &gizli
                        .i((.., seq - 1.., ..))
                        .map_err(|e| CoreError::Data(format!("Tensör hatası: {e}")))?,
                )
                .map_err(|e| CoreError::Data(format!("Son doğrusal katman hatası: {e}")))?
                .i(0)
                .map_err(|e| CoreError::Data(format!("Tensör hatası: {e}")))?
                .i(0)
                .map_err(|e| CoreError::Data(format!("Tensör hatası: {e}")))?
                .to_dtype(DType::F32)
                .map_err(|e| CoreError::Data(format!("Tensör hatası: {e}")))?;
            let next = logits
                .argmax(D::Minus1)
                .map_err(|e| CoreError::Data(format!("Argmax hatası: {e}")))?
                .to_scalar::<u32>()
                .map_err(|e| CoreError::Data(format!("Skaler hatası: {e}")))?;
            if next == eot {
                break;
            }
            tokens.push(next);
            out_ids.push(next);
        }

        self.tokenizer
            .decode(&out_ids, true)
            .map_err(|e| CoreError::Data(format!("Belirteç çözme hatası: {e}")))
    }

    /// Mel spektrogram hesaplar.
    /// 30 saniyelik sabit pencere için log-mel spektrogram hesaplar.
    ///
    /// Whisper kodlayıcısı **sabit** `N_FRAMES` (3000) mel karesi bekler.
    /// Kısa kayıtlar sıfırla doldurulur; candle'ın mel fonksiyonunun eklediği
    /// fazla dolgu kareleri kırpılır. Bu adım atlanırsa kodlayıcı hatalı
    /// boyutla çalışır ve model anlamsız çıktı üretir.
    fn mel(&self, samples: &[f32]) -> Result<Tensor> {
        let mut pencere = vec![0.0f32; m::N_SAMPLES];
        let n = samples.len().min(m::N_SAMPLES);
        pencere[..n].copy_from_slice(&samples[..n]);

        let mel = audio::pcm_to_mel(&self.config, &pencere, &self.mel_filters);
        let kare = mel.len() / self.config.num_mel_bins;
        if kare < m::N_FRAMES {
            return Err(CoreError::Data(format!(
                "Mel kare sayısı yetersiz: {kare} < {}",
                m::N_FRAMES
            )));
        }
        let t = Tensor::from_vec(mel, (1, self.config.num_mel_bins, kare), &Device::Cpu)
            .map_err(|e| CoreError::Data(format!("Mel tensörü hatası: {e}")))?
            .narrow(2, 0, m::N_FRAMES)
            .map_err(|e| CoreError::Data(format!("Mel kırpma hatası: {e}")))?;
        t.to_device(&self.device)
            .map_err(|e| CoreError::Data(format!("Mel tensörü cihaza taşınamadı: {e}")))
    }
}

impl TranscriptionEngine for WhisperCandleEngine {
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            name: format!("Whisper ({})", self.model_name),
            version: self.model_version.clone(),
            word_timestamps: WordTimestamps::SegmentOnly,
            language_detection: true,
            languages: Vec::new(),
            devices: vec![self.device_label.clone()],
            offline: true,
        }
    }

    fn transcribe(
        &self,
        request: &TranscriptionRequest,
        progress: &mut dyn FnMut(f64) -> bool,
    ) -> Result<TranscriptionOutput> {
        let mut model = self.build_model()?;

        let pcm = birkat_audio::decode_to_pcm(&request.path)?;
        let mono = pcm.to_mono();
        if mono.is_empty() {
            return Err(CoreError::Data("Ses dosyası boş.".to_string()));
        }
        let samples = resample_to_target(&mono, pcm.sample_rate, 16000);
        let total_samples = samples.len();

        let chunk = ((request.chunk_secs * 16000.0) as usize).max(16000);
        let overlap = ((request.overlap_secs * 16000.0) as usize).min(chunk / 2);
        let step = chunk.saturating_sub(overlap).max(16000);

        let mut segments = Vec::new();
        let mut chunks = 0u32;
        let mut pos = 0usize;
        let total_chunks = total_samples.div_ceil(step).max(1);
        let mut warnings = Vec::new();
        if self.mel_source == MelFilterSource::Analytic {
            warnings.push(
                "Resmî mel filtre dosyası bulunamadı; analitik filtre bankası kullanılıyor."
                    .to_string(),
            );
        }
        warnings.push(
            "Bu motor segment düzeyi zaman kodu üretir; kelime düzeyi zaman kodu \
             doğrulanmış bir hizalama yöntemiyle elde edilmedikçe gösterilmez."
                .to_string(),
        );

        while pos < total_samples {
            let end = (pos + chunk).min(total_samples);
            let slice = &samples[pos..end];
            let start_secs = pos as f64 / 16000.0;
            let level = birkat_audio::decode::level_dbfs(slice);

            // Sessiz parçaları çözmez; halüsinasyon riskini azaltır.
            if level >= -60.0 && !slice.is_empty() {
                let mel = self.mel(slice)?;
                let text = self.decode_chunk(&mut model, &mel, request.language.as_deref())?;
                if !text.trim().is_empty() {
                    segments.push(TranscriptSegmentDraft {
                        start: start_secs,
                        end: end as f64 / 16000.0,
                        text: text.trim().to_string(),
                        // Motor kelime düzeyi zaman kodu üretmez.
                        words: Vec::new(),
                        speaker: None,
                        model_indicator: None,
                        level_dbfs: Some(level),
                    });
                }
            }

            chunks += 1;
            pos += step;
            if !progress((chunks as f64 / total_chunks as f64).min(1.0)) {
                return Err(CoreError::Validation(
                    "İşlem kullanıcı tarafından iptal edildi.".to_string(),
                ));
            }
        }

        Ok(TranscriptionOutput {
            segments,
            language: request.language.clone(),
            language_indicator: None,
            word_timestamps: WordTimestamps::SegmentOnly,
            chunks,
            elapsed_secs: 0.0,
            device: self.device_label.clone(),
            warnings,
        })
    }
}

/// Çıkarım için cihaz seçer.
///
/// GPU derleme zamanında etkinleştirilmişse ve başlatılabiliyorsa GPU
/// kullanılır; aksi hâlde CPU'ya düşülür.
pub fn select_device(prefer_gpu: bool) -> (Device, String) {
    if prefer_gpu {
        #[cfg(feature = "cuda")]
        {
            if let Ok(d) = Device::new_cuda(0) {
                return (d, "cuda:0".to_string());
            }
        }
        #[cfg(feature = "metal")]
        {
            if let Ok(d) = Device::new_metal(0) {
                return (d, "metal:0".to_string());
            }
        }
    }
    (Device::Cpu, "cpu".to_string())
}

/// Bayt dizisini `f32` dizisine çevirir (küçük endian).
pub fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Doğrusal yeniden örnekleme.
///
/// Kaynak ve hedef frekans eşitse girdiyi aynen döndürür. Aksi hâlde doğrusal
/// interpolasyon uygular. Orijinal kayıt değiştirilmez; bu yalnızca çıkarım
/// girdisi için yapılan geçici bir dönüşümdür.
pub fn resample_to_target(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if source_rate == 0 || source_rate == target_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = target_rate as f64 / source_rate as f64;
    let out_len = ((samples.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// Analitik mel filtre bankası (Slaney/HTK tarzı, Whisper parametreleri).
///
/// Resmî filtre dosyası yoksa kullanılır.
pub fn analytic_mel_filters(num_mel_bins: usize) -> Vec<f32> {
    // Whisper: 16 kHz, N_FFT=400, mel 80/128 bin.
    let n_fft = m::N_FFT;
    let sample_rate = m::SAMPLE_RATE as f32;
    let n_freqs = n_fft / 2 + 1;
    let f_min = 0.0f32;
    let f_max = sample_rate / 2.0;
    let n_mels = num_mel_bins;

    let hz_to_mel = |f: f32| 2595.0 * (1.0 + f / 700.0).log10();
    let mel_to_hz = |m: f32| 700.0 * (10f32.powf(m / 2595.0) - 1.0);

    let mel_min = hz_to_mel(f_min);
    let mel_max = hz_to_mel(f_max);
    let mel_points: Vec<f32> = (0..n_mels + 2)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();
    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();

    let fft_freqs: Vec<f32> = (0..n_freqs)
        .map(|i| i as f32 * sample_rate / n_fft as f32)
        .collect();

    let mut filters = vec![0.0f32; n_freqs * n_mels];
    for m in 0..n_mels {
        let left = hz_points[m];
        let center = hz_points[m + 1];
        let right = hz_points[m + 2];
        for (i, &f) in fft_freqs.iter().enumerate() {
            let v = if f >= left && f <= center {
                if center > left {
                    (f - left) / (center - left)
                } else {
                    0.0
                }
            } else if f > center && f <= right {
                if right > center {
                    (right - f) / (right - center)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            // Whisper filtresinde normalizasyon uygulanmaz.
            filters[i * n_mels + m] = v;
        }
    }
    filters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_selection_falls_back_to_cpu() {
        let (d, label) = select_device(false);
        assert_eq!(label, "cpu");
        assert!(matches!(d, Device::Cpu));
    }

    #[test]
    fn resample_identity_keeps_samples() {
        let s = vec![0.1f32, 0.2, 0.3];
        assert_eq!(resample_to_target(&s, 16000, 16000), s);
    }

    #[test]
    fn resample_upsamples_length() {
        let s = vec![0.0f32; 100];
        let out = resample_to_target(&s, 8000, 16000);
        assert!((out.len() as i64 - 200).abs() <= 1);
    }

    #[test]
    fn resample_preserves_ramp_shape() {
        let s: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let out = resample_to_target(&s, 8000, 16000);
        assert!(out[0] < 0.02);
        assert!(out[out.len() - 1] > 0.9);
    }

    #[test]
    fn analytic_filters_have_correct_shape() {
        let f = analytic_mel_filters(80);
        assert_eq!(f.len(), (m::N_FFT / 2 + 1) * 80);
    }

    #[test]
    fn analytic_filters_are_normalised_triangles() {
        let n_mels = 8;
        let f = analytic_mel_filters(n_mels);
        let n_freqs = m::N_FFT / 2 + 1;
        // Her mel bandı üçgen olmalı: en büyük değer 1'e yakın (FFT ızgarası
        // tam merkeze denk gelmeyebilir), uçlar sıfıra yakın.
        for m in 0..n_mels {
            let col: Vec<f32> = (0..n_freqs).map(|i| f[i * n_mels + m]).collect();
            let peak = col.iter().copied().fold(0.0f32, f32::max);
            assert!(peak > 0.9 && peak <= 1.0 + 1e-5, "mel {m} tepe={peak}");
            assert!(col[0] < 0.2, "mel {m} ilk değer sıfıra yakın olmalı");
        }
    }

    #[test]
    fn analytic_filters_cover_spectrum_monotonically() {
        let n_mels = 16;
        let f = analytic_mel_filters(n_mels);
        let n_freqs = m::N_FFT / 2 + 1;
        // Band merkezleri artan frekansta olmalı.
        let centers: Vec<usize> = (0..n_mels)
            .map(|m| {
                (0..n_freqs)
                    .max_by(|&a, &b| f[a * n_mels + m].partial_cmp(&f[b * n_mels + m]).unwrap())
                    .unwrap()
            })
            .collect();
        for w in centers.windows(2) {
            assert!(w[1] > w[0], "merkezler artmalı: {centers:?}");
        }
    }

    #[test]
    fn bytes_to_f32_decodes_little_endian() {
        let mut b = Vec::new();
        b.extend_from_slice(&1.5f32.to_le_bytes());
        b.extend_from_slice(&(-2.0f32).to_le_bytes());
        assert_eq!(bytes_to_f32(&b), vec![1.5, -2.0]);
    }
}
