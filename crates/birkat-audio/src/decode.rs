//! Ses çözümleme (8, 9).
//!
//! Symphonia ile ses akışını PCM'e çözer. Çözülen örnekler `f32` aralığında
//! `[-1.0, 1.0]` tutulur.
//!
//! Çözülemeyen codec sessizce atlanmaz; açık hata nedeni döndürülür.

use birkat_core::error::{CoreError, Result};
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Çözülmüş PCM tamponu.
#[derive(Debug, Clone)]
pub struct PcmBuffer {
    /// Örnekleme frekansı.
    pub sample_rate: u32,
    /// Kanal sayısı.
    pub channels: u16,
    /// Kanallara ayrılmış örnekler (`[-1.0, 1.0]`).
    pub samples: Vec<Vec<f32>>,
}

impl PcmBuffer {
    /// Toplam kare sayısı (kanal başına örnek sayısı).
    pub fn frames(&self) -> usize {
        self.samples.first().map(|c| c.len()).unwrap_or(0)
    }

    /// Süre (saniye).
    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.frames() as f64 / self.sample_rate as f64
    }

    /// Tek kanalı döndürür.
    pub fn channel(&self, index: usize) -> Option<&[f32]> {
        self.samples.get(index).map(|v| v.as_slice())
    }

    /// Kanalları mono'ya indirger (ortalama).
    ///
    /// Bu bir türetilmiş veridir; orijinal kayıt yerine geçmez.
    pub fn to_mono(&self) -> Vec<f32> {
        if self.samples.len() == 1 {
            return self.samples[0].clone();
        }
        let n = self.frames();
        let mut out = vec![0.0f32; n];
        for ch in &self.samples {
            for (i, s) in ch.iter().enumerate() {
                out[i] += s;
            }
        }
        let k = self.samples.len() as f32;
        for v in out.iter_mut() {
            *v /= k;
        }
        out
    }

    /// Belirli bir zaman aralığını saniye cinsinden dilimler.
    pub fn slice_secs(&self, start: f64, end: f64) -> PcmBuffer {
        let sr = self.sample_rate as f64;
        let from = ((start * sr).max(0.0) as usize).min(self.frames());
        let to = ((end * sr).max(0.0) as usize).min(self.frames());
        let samples = self
            .samples
            .iter()
            .map(|ch| ch[from..to].to_vec())
            .collect();
        PcmBuffer {
            sample_rate: self.sample_rate,
            channels: self.channels,
            samples,
        }
    }

    /// Tüm kanalların örnek sayısı tutarlı mı?
    pub fn is_consistent(&self) -> bool {
        let n = self.frames();
        self.samples.iter().all(|c| c.len() == n) && !self.samples.is_empty()
    }
}

/// Bir dosyayı tamamen çözer.
pub fn decode_to_pcm(path: impl AsRef<Path>) -> Result<PcmBuffer> {
    let path = path.as_ref();
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| CoreError::Data(format!("Ses dosyası açılamadı: {e}")))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| CoreError::Data("Dosyada ses akışı bulunamadı.".to_string()))?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| CoreError::Data(format!("Codec çözülemedi: {e}")))?;

    let mut sample_rate = codec_params.sample_rate.unwrap_or(0);
    let mut channels: u16 = codec_params.channels.map(|c| c.count() as u16).unwrap_or(1);
    let mut ch_buf: Vec<Vec<f32>> = vec![Vec::new(); channels as usize];

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(CoreError::Data(format!("Paket okunamadı: {e}"))),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                sample_rate = spec.rate;
                channels = spec.channels.count() as u16;
                if ch_buf.len() != channels as usize {
                    ch_buf = vec![Vec::new(); channels as usize];
                }
                append_buffer(&decoded, &mut ch_buf);
            }
            Err(symphonia::core::errors::Error::DecodeError(msg)) => {
                // Bozuk çerçeve: kaydı raporla, çözmeye devam et.
                tracing_warn(msg);
                continue;
            }
            Err(e) => return Err(CoreError::Data(format!("Çözme hatası: {e}"))),
        }
    }

    Ok(PcmBuffer {
        sample_rate,
        channels,
        samples: ch_buf,
    })
}

/// Dosyayı mono'ya indirgeyerek çözer.
pub fn decode_to_mono(path: impl AsRef<Path>) -> Result<Vec<f32>> {
    Ok(decode_to_pcm(path)?.to_mono())
}

fn append_buffer(buf: &AudioBufferRef<'_>, ch_buf: &mut [Vec<f32>]) {
    use symphonia::core::audio::AudioBufferRef as A;
    let frames = buf.frames();
    match buf {
        A::U8(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push((b.chan(c)[i] as f32 - 128.0) / 128.0);
                }
            }
        }
        A::U16(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push((b.chan(c)[i] as f32 - 32768.0) / 32768.0);
                }
            }
        }
        A::U24(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push(b.chan(c)[i].inner() as f32 / 8_388_608.0 - 1.0);
                }
            }
        }
        A::U32(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push((b.chan(c)[i] as f64 / 2_147_483_648.0 - 1.0) as f32);
                }
            }
        }
        A::S8(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push(b.chan(c)[i] as f32 / 128.0);
                }
            }
        }
        A::S16(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push(b.chan(c)[i] as f32 / 32768.0);
                }
            }
        }
        A::S24(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push(b.chan(c)[i].inner() as f32 / 8_388_608.0);
                }
            }
        }
        A::S32(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push(b.chan(c)[i] as f64 as f32 / 2_147_483_648.0);
                }
            }
        }
        A::F32(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push(b.chan(c)[i]);
                }
            }
        }
        A::F64(b) => {
            for (c, out) in ch_buf.iter_mut().enumerate() {
                for i in 0..frames {
                    out.push(b.chan(c)[i] as f32);
                }
            }
        }
    }
}

fn tracing_warn(msg: &str) {
    // Kütüphane katmanı günlükçüye bağımlı değildir; çağıran katman
    // uyarıları toplayabilir. Burada sessizce devam edilir.
    let _ = msg;
}

/// Ses düzeyini dBFS cinsinden hesaplar.
pub fn level_dbfs(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return f32::NEG_INFINITY;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    if rms <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * rms.log10()
    }
}

/// Sessizlik oranını (örnek bazında) hesaplar.
pub fn silence_ratio(samples: &[f32], threshold: f32) -> f32 {
    if samples.is_empty() {
        return 1.0;
    }
    let silent = samples.iter().filter(|s| s.abs() < threshold).count();
    silent as f32 / samples.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::tests::write_test_wav;

    #[test]
    fn decodes_wav_to_pcm() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.wav");
        write_test_wav(&p, 44100, 1, 0.25);
        let buf = decode_to_pcm(&p).unwrap();
        assert_eq!(buf.sample_rate, 44100);
        assert_eq!(buf.channels, 1);
        assert!(buf.is_consistent());
        assert!((buf.duration_secs() - 0.25).abs() < 0.02);
    }

    #[test]
    fn decodes_stereo_to_two_channels() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.wav");
        write_test_wav(&p, 48000, 2, 0.1);
        let buf = decode_to_pcm(&p).unwrap();
        assert_eq!(buf.channels, 2);
        assert_eq!(buf.samples.len(), 2);
    }

    #[test]
    fn mono_downmix_halves_two_equal_channels() {
        let buf = PcmBuffer {
            sample_rate: 1000,
            channels: 2,
            samples: vec![vec![1.0, 0.5], vec![1.0, 0.5]],
        };
        let m = buf.to_mono();
        assert!((m[0] - 1.0).abs() < 1e-6);
        assert!((m[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn slice_secs_extracts_range() {
        let buf = PcmBuffer {
            sample_rate: 10,
            channels: 1,
            samples: vec![(0..100).map(|i| i as f32).collect()],
        };
        let s = buf.slice_secs(1.0, 2.0);
        assert_eq!(s.frames(), 10);
        assert_eq!(s.samples[0][0], 10.0);
    }

    #[test]
    fn dbfs_of_full_scale_is_zero() {
        let s = vec![1.0f32; 100];
        assert!((level_dbfs(&s) - 0.0).abs() < 0.01);
    }

    #[test]
    fn dbfs_of_silence_is_neg_infinity() {
        assert_eq!(level_dbfs(&[0.0; 100]), f32::NEG_INFINITY);
    }

    #[test]
    fn silence_ratio_is_one_for_silence() {
        assert!((silence_ratio(&[0.0; 50], 0.001) - 1.0).abs() < 1e-6);
        assert!(silence_ratio(&[0.5; 50], 0.001) < 0.01);
    }

    #[test]
    fn undecodable_file_reports_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.mp3");
        std::fs::write(&p, b"bu gecerli bir mp3 degil").unwrap();
        assert!(decode_to_pcm(&p).is_err());
    }
}
