//! Oynatma ve inceleme çekirdeği (8).
//!
//! Bütünleşik ses inceleme işlevleri: oynat/duraklat, başlat/bitir, zaman
//! koduna gitme, ileri/geri sarma, hızlı/yavaş oynatma, döngüsel oynatma,
//! seçili bölümü tekrar dinleme, kanal seçimi, sol/sağ kanal ayrı dinleme.
//!
//! Bu modül, çalma listesini ve konumu yönetir. Gerçek ses çıkışı platform
//! katmanındadır; burada deterministik, test edilebilir bir denetleyici
//! sunulur.

use serde::{Deserialize, Serialize};

/// Kanal dinleme seçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChannelSelection {
    /// Tüm kanallar.
    #[default]
    Both,
    /// Yalnızca sol.
    Left,
    /// Yalnızca sağ.
    Right,
    /// Belirli bir kanal indeksi.
    Index(u16),
}

/// Oynatma durumu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackState {
    /// Oynatılıyor mu?
    pub playing: bool,
    /// Geçerli konum (saniye).
    pub position: f64,
    /// Toplam süre (saniye).
    pub duration: f64,
    /// Oynatma hızı.
    pub speed: f32,
    /// Döngü açık mı?
    pub looping: bool,
    /// Döngü aralığı (varsa).
    pub loop_range: Option<(f64, f64)>,
    /// Seçili kanal.
    pub channel: ChannelSelection,
    /// Ses düzeyi (0.0 - 1.0).
    pub volume: f32,
}

/// Oynatma denetleyicisi.
#[derive(Debug, Clone)]
pub struct PlaybackController {
    state: PlaybackState,
}

impl PlaybackController {
    /// Yeni denetleyici.
    pub fn new(duration: f64) -> Self {
        Self {
            state: PlaybackState {
                playing: false,
                position: 0.0,
                duration: duration.max(0.0),
                speed: 1.0,
                looping: false,
                loop_range: None,
                channel: ChannelSelection::Both,
                volume: 1.0,
            },
        }
    }

    /// Mevcut durum.
    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    /// Oynatmayı başlatır.
    pub fn play(&mut self) {
        self.state.playing = true;
    }

    /// Oynatmayı duraklatır.
    pub fn pause(&mut self) {
        self.state.playing = false;
    }

    /// Oynat/duraklat arasında geçiş yapar.
    pub fn toggle(&mut self) {
        self.state.playing = !self.state.playing;
    }

    /// Başa döner.
    pub fn rewind_to_start(&mut self) {
        self.state.position = 0.0;
    }

    /// Sona gider.
    pub fn seek_to_end(&mut self) {
        self.state.position = self.state.duration;
    }

    /// Zaman koduna gider (sınırlar içinde).
    pub fn seek(&mut self, seconds: f64) {
        self.state.position = seconds.clamp(0.0, self.state.duration);
    }

    /// Zaman koduna gider ve oradan çalmaya başlar.
    pub fn play_from(&mut self, seconds: f64) {
        self.seek(seconds);
        self.play();
    }

    /// İleri sarar (hız ile orantılı).
    pub fn skip_forward(&mut self, seconds: f64) {
        self.seek(self.state.position + seconds);
    }

    /// Geri sarar.
    pub fn skip_backward(&mut self, seconds: f64) {
        self.seek(self.state.position - seconds);
    }

    /// Oynatma hızını ayarlar (0.25 - 4.0).
    pub fn set_speed(&mut self, speed: f32) {
        self.state.speed = speed.clamp(0.25, 4.0);
    }

    /// Döngüyü açar/kapatır.
    pub fn set_looping(&mut self, on: bool) {
        self.state.looping = on;
        if !on {
            self.state.loop_range = None;
        }
    }

    /// Belirli bir bölümü döngüye alır.
    pub fn loop_section(&mut self, start: f64, end: f64) {
        let (a, b) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        self.state.loop_range = Some((
            a.clamp(0.0, self.state.duration),
            b.clamp(0.0, self.state.duration),
        ));
        self.state.looping = true;
    }

    /// Seçili kanalı ayarlar.
    pub fn set_channel(&mut self, channel: ChannelSelection) {
        self.state.channel = channel;
    }

    /// Ses düzeyini ayarlar.
    pub fn set_volume(&mut self, volume: f32) {
        self.state.volume = volume.clamp(0.0, 1.0);
    }

    /// Zaman ilerletir (çalma döngüsünden çağrılır).
    ///
    /// Döngü aralığı veya döngü açıksa başa döner. Konum süreyi aşarsa
    /// oynatma durur.
    pub fn advance(&mut self, delta_secs: f64) {
        if !self.state.playing {
            return;
        }
        let effective = delta_secs * self.state.speed as f64;
        let mut pos = self.state.position + effective;

        if let Some((start, end)) = self.state.loop_range {
            if pos >= end {
                pos = start + (pos - end) % (end - start).max(1e-9);
            }
        } else if self.state.looping {
            if pos >= self.state.duration && self.state.duration > 0.0 {
                pos %= self.state.duration;
            }
        } else if pos >= self.state.duration {
            pos = self.state.duration;
            self.state.playing = false;
        }
        self.state.position = pos.max(0.0);
    }

    /// Geçerli konumun zaman kodu metni (`ss.ddd` veya `mm:ss.ddd`).
    pub fn time_code(&self) -> String {
        format_time_code(self.state.position)
    }

    /// İlerleme oranı (0.0 - 1.0).
    pub fn progress(&self) -> f64 {
        if self.state.duration <= 0.0 {
            0.0
        } else {
            (self.state.position / self.state.duration).clamp(0.0, 1.0)
        }
    }
}

/// Saniyeyi `mm:ss.ddd` biçiminde zaman koduna çevirir.
pub fn format_time_code(seconds: f64) -> String {
    let s = seconds.max(0.0);
    let total_ms = (s * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let secs = total_secs % 60;
    let mins = total_secs / 60;
    format!("{mins:02}:{secs:02}.{ms:03}")
}

/// Zaman kodunu saniyeye çevirir.
pub fn parse_time_code(text: &str) -> Option<f64> {
    let parts: Vec<&str> = text.split(':').collect();
    match parts.len() {
        1 => parts[0].parse::<f64>().ok(),
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let s: f64 = parts[1].parse().ok()?;
            Some(m * 60.0 + s)
        }
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts[2].parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_code_formatting_and_parsing() {
        assert_eq!(format_time_code(0.0), "00:00.000");
        assert_eq!(format_time_code(61.5), "01:01.500");
        assert_eq!(parse_time_code("01:01.500"), Some(61.5));
        assert_eq!(parse_time_code("2:00"), Some(120.0));
        assert_eq!(parse_time_code("1:00:00"), Some(3600.0));
        assert_eq!(parse_time_code("5.5"), Some(5.5));
        assert_eq!(parse_time_code("abc"), None);
    }

    #[test]
    fn play_pause_toggle() {
        let mut c = PlaybackController::new(10.0);
        assert!(!c.state().playing);
        c.play();
        assert!(c.state().playing);
        c.pause();
        assert!(!c.state().playing);
        c.toggle();
        assert!(c.state().playing);
    }

    #[test]
    fn seek_is_clamped() {
        let mut c = PlaybackController::new(10.0);
        c.seek(100.0);
        assert_eq!(c.state().position, 10.0);
        c.seek(-5.0);
        assert_eq!(c.state().position, 0.0);
    }

    #[test]
    fn advance_respects_speed() {
        let mut c = PlaybackController::new(10.0);
        c.play();
        c.set_speed(2.0);
        c.advance(1.0);
        assert!((c.state().position - 2.0).abs() < 1e-9);
    }

    #[test]
    fn playback_stops_at_end_without_loop() {
        let mut c = PlaybackController::new(5.0);
        c.play();
        c.advance(10.0);
        assert_eq!(c.state().position, 5.0);
        assert!(!c.state().playing);
    }

    #[test]
    fn looping_wraps_around() {
        let mut c = PlaybackController::new(5.0);
        c.play();
        c.set_looping(true);
        c.advance(7.0);
        assert!(c.state().position < 5.0);
        assert!(c.state().playing);
    }

    #[test]
    fn section_loop_wraps_within_range() {
        let mut c = PlaybackController::new(100.0);
        c.play();
        c.loop_section(10.0, 20.0);
        c.seek(19.5);
        c.advance(1.0);
        assert!(c.state().position >= 10.0 && c.state().position <= 20.0);
    }

    #[test]
    fn speed_is_clamped_to_supported_range() {
        let mut c = PlaybackController::new(10.0);
        c.set_speed(10.0);
        assert_eq!(c.state().speed, 4.0);
        c.set_speed(0.01);
        assert_eq!(c.state().speed, 0.25);
    }

    #[test]
    fn channel_selection() {
        let mut c = PlaybackController::new(10.0);
        c.set_channel(ChannelSelection::Left);
        assert_eq!(c.state().channel, ChannelSelection::Left);
        c.set_channel(ChannelSelection::Right);
        assert_eq!(c.state().channel, ChannelSelection::Right);
    }

    #[test]
    fn progress_is_bounded() {
        let mut c = PlaybackController::new(10.0);
        assert_eq!(c.progress(), 0.0);
        c.seek(5.0);
        assert!((c.progress() - 0.5).abs() < 1e-9);
        c.seek(100.0);
        assert_eq!(c.progress(), 1.0);
    }

    #[test]
    fn zero_duration_does_not_panic() {
        let mut c = PlaybackController::new(0.0);
        c.play();
        c.advance(1.0);
        assert_eq!(c.progress(), 0.0);
    }

    #[test]
    fn paused_advance_does_nothing() {
        let mut c = PlaybackController::new(10.0);
        c.advance(5.0);
        assert_eq!(c.state().position, 0.0);
    }
}
