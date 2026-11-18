//! Ses dosyası teknik envanteri (7.2).
//!
//! Her ses için kaynak dosya kimliği, dosya adı, boyut, hash, kapsayıcı,
//! codec, süre, örnekleme frekansı, bit derinliği, bit hızı, kanal sayısı,
//! kanal düzeni, metadata etiketleri, kodlayıcı bilgisi, dosya sistemi zaman
//! bilgileri, akış başlangıç/bitiş zamanları, okunabilirlik durumu ve
//! hata/uyarılar kaydedilir.
//!
//! **Kaynak dosyada olmayan metadata üretilmez.**

use birkat_core::audio::AudioMetadata;
use birkat_core::error::{CoreError, Result};
use std::path::Path;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::{FormatOptions, FormatReader, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;

/// Sondaj sonucu.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Teknik metadata.
    pub metadata: AudioMetadata,
    /// Toplam örnek sayısı (biliniyorsa).
    pub total_frames: Option<u64>,
    /// Ses akışı var mı?
    pub has_audio_track: bool,
    /// Akış indeksi.
    pub stream_index: Option<usize>,
}

/// Bir dosyayı Symphonia ile inceler.
pub fn probe_file(path: impl AsRef<Path>) -> Result<ProbeResult> {
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
    let container_name = container_from_hint(path);

    let result = probe_from_format(&mut *format, container_name);
    Ok(result)
}

/// Dosya uzantısından kapsayıcı adı tahmin eder (yalnızca ipucu).
fn container_from_hint(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_uppercase())
}

/// Açılmış bir format okuyucusundan metadata üretir.
pub fn probe_from_format(
    format: &mut dyn FormatReader,
    container_hint: Option<String>,
) -> ProbeResult {
    let mut warnings = Vec::new();
    let mut tags = Vec::new();
    let mut encoder = None;

    // Metadata etiketleri (kaynakta varsa).
    let mut metadata_rev = format.metadata();
    if let Some(rev) = metadata_rev.skip_to_latest() {
        for tag in rev.tags() {
            if let Some(key) = tag.std_key {
                if key == StandardTagKey::EncodedBy {
                    encoder = Some(tag.value.to_string());
                    continue;
                }
                tags.push((format!("{key:?}"), tag.value.to_string()));
            } else {
                tags.push((tag.key.clone(), tag.value.to_string()));
            }
        }
    }

    let audio_track: Option<&Track> = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL);

    let Some(track) = audio_track else {
        return ProbeResult {
            metadata: AudioMetadata {
                container: container_hint.unwrap_or_else(|| "bilinmiyor".to_string()),
                codec: "yok".to_string(),
                duration_secs: None,
                sample_rate: None,
                bit_depth: None,
                bit_rate: None,
                channels: None,
                channel_layout: None,
                tags,
                encoder,
                stream_start: None,
                stream_end: None,
                warnings: vec!["Dosyada ses akışı bulunamadı.".to_string()],
            },
            total_frames: None,
            has_audio_track: false,
            stream_index: None,
        };
    };

    let cp: &CodecParameters = &track.codec_params;
    let codec_name = codec_label(cp.codec);
    let sample_rate = cp.sample_rate;
    let channels = cp
        .channels
        .map(|c| c.count() as u16);
    let bit_depth = cp.bits_per_sample.map(|b| b as u16);
    let bit_rate = cp
        .bits_per_sample
        .zip(cp.sample_rate)
        .map(|(bits, rate)| bits as u64 * rate as u64 * channels.unwrap_or(1) as u64);

    // Süre: n_frames varsa oradan; yoksa time_base'ten.
    let duration_secs = cp
        .n_frames
        .zip(cp.sample_rate)
        .map(|(frames, rate)| frames as f64 / rate as f64)
        .or_else(|| {
            track
                .codec_params
                .time_base
                .zip(track.codec_params.n_frames)
                .map(|(tb, frames)| {
                    let t = tb.calc_time(frames);
                    t.seconds as f64 + t.frac
                })
        });

    if sample_rate.is_none() {
        warnings.push("Örnekleme frekansı dosyada belirtilmemiş.".to_string());
    }
    if bit_depth.is_none() {
        warnings.push("Bit derinliği dosyada belirtilmemiş.".to_string());
    }

    let container = container_hint.unwrap_or_else(|| infer_container_from_codec(cp.codec));

    ProbeResult {
        metadata: AudioMetadata {
            container,
            codec: codec_name,
            duration_secs,
            sample_rate,
            bit_depth,
            bit_rate,
            channels,
            channel_layout: None,
            tags,
            encoder,
            stream_start: None,
            stream_end: duration_secs,
            warnings,
        },
        total_frames: cp.n_frames,
        has_audio_track: true,
        stream_index: Some(track.id as usize),
    }
}

/// Bir `MediaSourceStream` için codec etiketini Türkçe/okunabilir biçimde verir.
pub fn codec_label(codec: symphonia::core::codecs::CodecType) -> String {
    use symphonia::core::codecs as c;
    match codec {
        c::CODEC_TYPE_NULL => "yok".to_string(),
        c::CODEC_TYPE_PCM_S16LE
        | c::CODEC_TYPE_PCM_S16BE
        | c::CODEC_TYPE_PCM_S24LE
        | c::CODEC_TYPE_PCM_S24BE
        | c::CODEC_TYPE_PCM_S32LE
        | c::CODEC_TYPE_PCM_S32BE
        | c::CODEC_TYPE_PCM_F32LE
        | c::CODEC_TYPE_PCM_F32BE
        | c::CODEC_TYPE_PCM_U8
        | c::CODEC_TYPE_PCM_ALAW
        | c::CODEC_TYPE_PCM_MULAW => "PCM".to_string(),
        x if x == c::CODEC_TYPE_MP3 => "MP3".to_string(),
        x if x == c::CODEC_TYPE_AAC => "AAC".to_string(),
        x if x == c::CODEC_TYPE_FLAC => "FLAC".to_string(),
        x if x == c::CODEC_TYPE_VORBIS => "Vorbis".to_string(),
        x if x == c::CODEC_TYPE_ALAC => "ALAC".to_string(),
        x if x == c::CODEC_TYPE_ADPCM_MS => "ADPCM (MS)".to_string(),
        x if x == c::CODEC_TYPE_ADPCM_IMA_WAV => "ADPCM (IMA WAV)".to_string(),
        x if x == c::CODEC_TYPE_OPUS => "Opus".to_string(),
        other => format!("{other:?}"),
    }
}

fn infer_container_from_codec(codec: symphonia::core::codecs::CodecType) -> String {
    use symphonia::core::codecs as c;
    match codec {
        c::CODEC_TYPE_FLAC => "FLAC".to_string(),
        c::CODEC_TYPE_MP3 => "MP3".to_string(),
        c::CODEC_TYPE_VORBIS | c::CODEC_TYPE_OPUS => "OGG".to_string(),
        c::CODEC_TYPE_AAC | c::CODEC_TYPE_ALAC => "MP4".to_string(),
        _ => "bilinmiyor".to_string(),
    }
}

/// Desteklenen kapsayıcı/codec matrisini döndürür (25. teslim, 22. madde).
pub fn supported_format_matrix() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        ("WAV", "PCM", true),
        ("BWF", "PCM", true),
        ("MP3", "MP3 (MPEG-1/2 Layer III)", true),
        ("M4A", "AAC / ALAC", true),
        ("AAC", "AAC (ADTS)", true),
        ("FLAC", "FLAC", true),
        ("OGG", "Vorbis / Opus", true),
        ("Opus", "Opus", true),
        ("AIFF", "PCM", true),
        ("CAF", "PCM", true),
        ("MP4", "AAC (ses akışı)", true),
        ("MOV", "AAC / PCM (ses akışı)", true),
        ("MKV", "AAC / Vorbis / Opus (ses akışı)", true),
        ("WebM", "Vorbis / Opus (ses akışı)", true),
        (
            "WMA",
            "Windows Media Audio (Windows Media Foundation adaptörü ile)",
            false,
        ),
        (
            "AMR",
            "Adaptif Çok Kanallı (Windows Media Foundation adaptörü ile)",
            false,
        ),
    ]
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Write;

    /// Basit bir WAV dosyası üretir (PCM 16-bit mono).
    pub fn write_test_wav(
        path: &Path,
        sample_rate: u32,
        channels: u16,
        seconds: f32,
    ) -> Vec<i16> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        let n = (sample_rate as f32 * seconds) as usize;
        let mut samples = Vec::with_capacity(n * channels as usize);
        for i in 0..n {
            // 440 Hz sinüs
            let t = i as f32 / sample_rate as f32;
            let v = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            let s = (v * 16000.0) as i16;
            for _ in 0..channels {
                w.write_sample(s).unwrap();
                samples.push(s);
            }
        }
        w.finalize().unwrap();
        samples
    }

    #[test]
    fn probes_wav_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.wav");
        write_test_wav(&p, 44100, 1, 1.0);
        let r = probe_file(&p).unwrap();
        assert!(r.has_audio_track);
        assert_eq!(r.metadata.codec, "PCM");
        assert_eq!(r.metadata.sample_rate, Some(44100));
        assert_eq!(r.metadata.channels, Some(1));
        assert_eq!(r.metadata.bit_depth, Some(16));
        let d = r.metadata.duration_secs.unwrap();
        assert!((d - 1.0).abs() < 0.05, "süre ~1 sn olmalı, {d}");
    }

    #[test]
    fn probes_stereo_wav() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("stereo.wav");
        write_test_wav(&p, 48000, 2, 0.5);
        let r = probe_file(&p).unwrap();
        assert_eq!(r.metadata.channels, Some(2));
        assert_eq!(r.metadata.sample_rate, Some(48000));
    }

    #[test]
    fn unknown_metadata_is_reported_not_invented() {
        let m = AudioMetadata {
            container: "WAV".into(),
            codec: "PCM".into(),
            duration_secs: Some(1.0),
            sample_rate: None,
            bit_depth: None,
            bit_rate: None,
            channels: Some(1),
            channel_layout: None,
            tags: vec![],
            encoder: None,
            stream_start: None,
            stream_end: None,
            warnings: vec![],
        };
        let u = m.unknown_fields();
        assert!(u.contains(&"örnekleme frekansı"));
        assert!(u.contains(&"bit derinliği"));
        assert!(u.contains(&"bit hızı"));
    }

    #[test]
    fn non_audio_file_fails_clearly() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("metin.txt");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(b"bu bir ses dosyasi degil, sadece metin").unwrap();
        let err = probe_file(&p).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn format_matrix_covers_required_containers() {
        let m = supported_format_matrix();
        let names: Vec<&str> = m.iter().map(|(n, _, _)| *n).collect();
        for required in [
            "WAV", "BWF", "MP3", "M4A", "AAC", "FLAC", "OGG", "Opus", "WMA", "AMR", "AIFF",
            "CAF", "MP4", "MOV", "MKV", "WebM",
        ] {
            assert!(names.contains(&required), "eksik: {required}");
        }
    }

    #[test]
    fn codec_labels_are_readable() {
        use symphonia::core::codecs as c;
        assert_eq!(codec_label(c::CODEC_TYPE_PCM_S16LE), "PCM");
        assert_eq!(codec_label(c::CODEC_TYPE_FLAC), "FLAC");
        assert_eq!(codec_label(c::CODEC_TYPE_NULL), "yok");
    }
}
