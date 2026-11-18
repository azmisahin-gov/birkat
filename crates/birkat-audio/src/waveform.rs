//! Dalga biçimi (8).
//!
//! Dalga biçimi, sesin genlik zarfını gösterir. İnceleme ekranında görsel
//! destek sağlar.
//!
//! ## Dürüstlük sınırı
//!
//! Dalga biçimi tek başına montaj veya orijinallik hakkında sonuç olarak
//! gösterilmez (bkz. [`interpretation_note`]).

use crate::decode::PcmBuffer;

/// Dalga biçimi tepe noktaları.
#[derive(Debug, Clone)]
pub struct WaveformPeaks {
    /// Kanal başına (min, max) çiftleri.
    pub channels: Vec<Vec<(f32, f32)>>,
    /// Örnekleme frekansı.
    pub sample_rate: u32,
    /// Toplam süre (saniye).
    pub duration_secs: f64,
    /// Kova başına kare sayısı.
    pub bucket_frames: usize,
}

impl WaveformPeaks {
    /// Toplam kova sayısı.
    pub fn buckets(&self) -> usize {
        self.channels.first().map(|c| c.len()).unwrap_or(0)
    }

    /// Belirli bir kova indeksinin zaman aralığını verir.
    pub fn bucket_time_range(&self, bucket: usize) -> Option<(f64, f64)> {
        if bucket >= self.buckets() || self.sample_rate == 0 {
            return None;
        }
        let sr = self.sample_rate as f64;
        let start = bucket as f64 * self.bucket_frames as f64 / sr;
        let end = (bucket + 1) as f64 * self.bucket_frames as f64 / sr;
        Some((start, end))
    }

    /// Kanalın tepe genliği.
    pub fn peak_amplitude(&self, channel: usize) -> f32 {
        self.channels
            .get(channel)
            .map(|c| {
                c.iter()
                    .map(|(mn, mx)| mn.abs().max(mx.abs()))
                    .fold(0.0f32, f32::max)
            })
            .unwrap_or(0.0)
    }
}

/// PCM tamponundan dalga biçimi üretir.
///
/// `target_buckets` yaklaşık kova sayısıdır (ekran genişliğine göre seçilir).
pub fn compute_peaks(pcm: &PcmBuffer, target_buckets: usize) -> WaveformPeaks {
    let frames = pcm.frames();
    let target = target_buckets.max(1);
    let bucket_frames = (frames / target).max(1);

    let mut channels = Vec::with_capacity(pcm.samples.len());
    for ch in &pcm.samples {
        let mut peaks = Vec::with_capacity(frames / bucket_frames + 1);
        let mut i = 0;
        while i < ch.len() {
            let end = (i + bucket_frames).min(ch.len());
            let slice = &ch[i..end];
            let mut mn = f32::MAX;
            let mut mx = f32::MIN;
            for &s in slice {
                if s < mn {
                    mn = s;
                }
                if s > mx {
                    mx = s;
                }
            }
            if mn == f32::MAX {
                mn = 0.0;
                mx = 0.0;
            }
            peaks.push((mn, mx));
            i = end;
        }
        channels.push(peaks);
    }

    WaveformPeaks {
        channels,
        sample_rate: pcm.sample_rate,
        duration_secs: pcm.duration_secs(),
        bucket_frames,
    }
}

/// Dalga biçiminin yorumlanma sınırını açıklar.
pub fn interpretation_note() -> &'static str {
    "Dalga biçimi ve spektrogram, sesin genlik ve frekans dağılımını gösterir. \
     Bu görseller tek başına bir kaydın montajlı, sahte veya değiştirilmiş \
     olduğunu göstermez; orijinallik değerlendirmesi adli ses incelemesi \
     (56.11) uzmanlığı gerektirir."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(channels: Vec<Vec<f32>>) -> PcmBuffer {
        PcmBuffer {
            sample_rate: 100,
            channels: channels.len() as u16,
            samples: channels,
        }
    }

    #[test]
    fn peaks_capture_min_and_max() {
        let p = pcm(vec![vec![-1.0, -0.5, 0.5, 1.0, 0.0, 0.0]]);
        let w = compute_peaks(&p, 3);
        assert!(w.buckets() >= 1);
        assert!((w.peak_amplitude(0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn bucket_time_ranges_are_monotonic() {
        let p = pcm(vec![vec![0.0; 1000]]);
        let w = compute_peaks(&p, 10);
        let mut prev = 0.0;
        for i in 0..w.buckets() {
            let (s, _e) = w.bucket_time_range(i).unwrap();
            assert!(s >= prev);
            prev = s;
        }
    }

    #[test]
    fn stereo_produces_two_channels() {
        let p = pcm(vec![vec![0.1; 100], vec![0.2; 100]]);
        let w = compute_peaks(&p, 5);
        assert_eq!(w.channels.len(), 2);
    }

    #[test]
    fn empty_buffer_does_not_panic() {
        let p = pcm(vec![vec![]]);
        let w = compute_peaks(&p, 10);
        assert_eq!(w.duration_secs, 0.0);
    }

    #[test]
    fn interpretation_note_mentions_56_11() {
        assert!(interpretation_note().contains("56.11"));
    }
}
