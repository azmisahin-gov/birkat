//! Spektrogram (8).
//!
//! Kısa zamanlı Fourier dönüşümü (STFT) ile frekans-zaman gösterimi üretir.
//!
//! ## Dürüstlük sınırı
//!
//! Spektrogram tek başına montaj veya orijinallik hakkında sonuç olarak
//! gösterilmez (bkz. [`crate::waveform::interpretation_note`]).

use crate::decode::PcmBuffer;
use serde::{Deserialize, Serialize};

/// Spektrogram verisi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spectrogram {
    /// Zaman kareleri; her kare `bins` uzunluğunda dB ölçekli genlik.
    pub frames: Vec<Vec<f32>>,
    /// FFT boyutu.
    pub fft_size: usize,
    /// Atlama boyutu.
    pub hop_size: usize,
    /// Örnekleme frekansı.
    pub sample_rate: u32,
    /// Frekans bin sayısı.
    pub bins: usize,
}

impl Spectrogram {
    /// Her bin'in frekans merkezini (Hz) döndürür.
    pub fn bin_frequencies(&self) -> Vec<f32> {
        let half = self.fft_size / 2;
        (0..self.bins)
            .map(|i| i as f32 * self.sample_rate as f32 / self.fft_size as f32)
            .take(half + 1)
            .collect()
    }

    /// Kare zamanı (saniye).
    pub fn frame_time(&self, index: usize) -> f64 {
        index as f64 * self.hop_size as f64 / self.sample_rate as f64
    }

    /// Tüm spektrogramdaki en büyük dB değeri.
    pub fn max_db(&self) -> f32 {
        self.frames
            .iter()
            .flat_map(|f| f.iter())
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }
}

/// Basit Hann penceresi üretir.
pub fn hann_window(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let x = std::f32::consts::PI * i as f32 / (n.max(2) - 1) as f32;
            x.sin().powi(2)
        })
        .collect()
}

/// İteratif (radix-2) Cooley-Tukey FFT.
///
/// `re` ve `im` yerinde dönüştürülür. Girdi uzunluğu 2'nin kuvveti olmalıdır.
pub fn fft_in_place(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    assert_eq!(n, im.len());
    if n <= 1 {
        return;
    }
    debug_assert!(n.is_power_of_two(), "FFT boyutu 2'nin kuvveti olmalıdır");

    // Bit tersine permütasyon.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Kelebek işlemleri.
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        let mut i = 0usize;
        while i < n {
            let mut cur_r = 1.0f32;
            let mut cur_i = 0.0f32;
            for k in 0..len / 2 {
                let ur = re[i + k];
                let ui = im[i + k];
                let vr = re[i + k + len / 2] * cur_r - im[i + k + len / 2] * cur_i;
                let vi = re[i + k + len / 2] * cur_i + im[i + k + len / 2] * cur_r;
                re[i + k] = ur + vr;
                im[i + k] = ui + vi;
                re[i + k + len / 2] = ur - vr;
                im[i + k + len / 2] = ui - vi;
                let next_r = cur_r * wr - cur_i * wi;
                let next_i = cur_r * wi + cur_i * wr;
                cur_r = next_r;
                cur_i = next_i;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// Spektrogram hesaplar.
///
/// `fft_size` 2'nin kuvveti olmalıdır. Sonuç dB ölçeğindedir.
pub fn compute_spectrogram(pcm: &PcmBuffer, fft_size: usize, hop_size: usize) -> Spectrogram {
    let fft_size = fft_size.next_power_of_two();
    let hop = hop_size.max(1);
    let window = hann_window(fft_size);
    let mono = pcm.to_mono();
    let bins = fft_size / 2 + 1;

    let mut frames = Vec::new();
    let mut pos = 0usize;
    while pos + fft_size <= mono.len() {
        let mut re = vec![0.0f32; fft_size];
        let mut im = vec![0.0f32; fft_size];
        for i in 0..fft_size {
            re[i] = mono[pos + i] * window[i];
        }
        fft_in_place(&mut re, &mut im);
        let mut mags = Vec::with_capacity(bins);
        for k in 0..bins {
            let m = (re[k] * re[k] + im[k] * im[k]).sqrt();
            // dB ölçeği (referans 1.0)
            mags.push(20.0 * (m + 1e-12).log10());
        }
        frames.push(mags);
        pos += hop;
    }

    Spectrogram {
        frames,
        fft_size,
        hop_size: hop,
        sample_rate: pcm.sample_rate,
        bins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn sine_pcm(freq: f32, sr: u32, secs: f32) -> PcmBuffer {
        let n = (sr as f32 * secs) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * freq * i as f32 / sr as f32).sin())
            .collect();
        PcmBuffer {
            sample_rate: sr,
            channels: 1,
            samples: vec![samples],
        }
    }

    #[test]
    fn hann_window_has_expected_shape() {
        let w = hann_window(8);
        assert_eq!(w.len(), 8);
        assert!(w[0].abs() < 1e-6);
        assert!(w[7].abs() < 1e-6);
        assert!(w[3] > 0.5);
    }

    #[test]
    fn fft_of_impulse_is_flat() {
        let n = 8;
        let mut re = vec![0.0f32; n];
        let mut im = vec![0.0f32; n];
        re[0] = 1.0;
        fft_in_place(&mut re, &mut im);
        for k in 0..n {
            assert!((re[k] - 1.0).abs() < 1e-5, "k={k} re={}", re[k]);
            assert!(im[k].abs() < 1e-5);
        }
    }

    #[test]
    fn fft_of_dc_is_single_bin() {
        let n = 8;
        let mut re = vec![1.0f32; n];
        let mut im = vec![0.0f32; n];
        fft_in_place(&mut re, &mut im);
        assert!((re[0] - 8.0).abs() < 1e-4);
        for (k, v) in re.iter().enumerate().skip(1) {
            assert!(v.abs() < 1e-4, "k={k} re={v}");
        }
    }

    #[test]
    fn spectrogram_peaks_at_input_frequency() {
        let p = sine_pcm(1000.0, 8000, 0.5);
        let s = compute_spectrogram(&p, 512, 256);
        assert!(!s.frames.is_empty());
        // İlk karede en yüksek bin 1000 Hz civarında olmalı.
        let frame = &s.frames[1];
        let (peak_bin, _) = frame
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        let freq = peak_bin as f32 * s.sample_rate as f32 / s.fft_size as f32;
        assert!((freq - 1000.0).abs() < 40.0, "tepe frekansı: {freq}");
    }

    #[test]
    fn bin_frequencies_are_correct() {
        let p = sine_pcm(440.0, 8000, 0.1);
        let s = compute_spectrogram(&p, 256, 128);
        let f = s.bin_frequencies();
        assert_eq!(f.len(), s.bins);
        assert_eq!(f[0], 0.0);
        // Nyquist bin
        let last = *f.last().unwrap();
        assert!((last - 4000.0).abs() < 1.0);
    }

    #[test]
    fn frame_times_are_monotonic() {
        let p = sine_pcm(440.0, 8000, 0.2);
        let s = compute_spectrogram(&p, 256, 128);
        for i in 1..s.frames.len() {
            assert!(s.frame_time(i) > s.frame_time(i - 1));
        }
    }

    #[test]
    fn empty_input_yields_empty_spectrogram() {
        let p = PcmBuffer {
            sample_rate: 8000,
            channels: 1,
            samples: vec![vec![]],
        };
        let s = compute_spectrogram(&p, 256, 128);
        assert!(s.frames.is_empty());
    }

    #[test]
    fn fft_size_is_forced_to_power_of_two() {
        let p = sine_pcm(440.0, 8000, 0.1);
        let s = compute_spectrogram(&p, 300, 100);
        assert!(s.fft_size.is_power_of_two());
    }
}
