//! Ses envanteri (7).
//!
//! Dosyaların gerçek içeriği analiz edilerek ses akışları otomatik tespit
//! edilir; yalnızca uzantıya güvenilmez. Desteklenmeyen bir codec varsa
//! dosya atlanmaz; açık hata nedeni gösterilir.
//!
//! ## Yinelenen dosya
//!
//! Hash eşitliği bulunan dosyalar yinelenen içerik olarak işaretlenir. Ancak
//! farklı kaynak konumları ve teslim ilişkileri silinmez; her kaynak ilişkisi
//! korunur.

use crate::probe::{probe_file, ProbeResult};
use birkat_core::audio::{AudioAsset, AudioMetadata, Readability};
use birkat_core::error::Result;
use birkat_core::evidence::DetectedKind;
use birkat_core::ids::{AudioId, FileId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Envanterdeki tek bir ses girdisi.
#[derive(Debug, Clone)]
pub struct AudioInventoryEntry {
    /// Kaynak dosya yolu.
    pub path: PathBuf,
    /// Dosya türü tespiti.
    pub detected: DetectedKind,
    /// Sondaj sonucu (başarılıysa).
    pub probe: Option<ProbeResult>,
    /// SHA-256 (varsa).
    pub sha256: Option<String>,
    /// Okunabilirlik.
    pub readability: Readability,
    /// Yinelenen içerik (varsa, ilk görülen dosya).
    pub duplicate_of: Option<PathBuf>,
    /// Uyarılar.
    pub warnings: Vec<String>,
}

impl AudioInventoryEntry {
    /// Envanter girdisini [`AudioAsset`] nesnesine dönüştürür.
    pub fn to_audio_asset(&self) -> AudioAsset {
        let metadata = self
            .probe
            .as_ref()
            .map(|p| p.metadata.clone())
            .unwrap_or_else(|| AudioMetadata {
                container: self.detected.label(),
                codec: "bilinmiyor".to_string(),
                duration_secs: None,
                sample_rate: None,
                bit_depth: None,
                bit_rate: None,
                channels: None,
                channel_layout: None,
                tags: Vec::new(),
                encoder: None,
                stream_start: None,
                stream_end: None,
                warnings: self.warnings.clone(),
            });

        AudioAsset {
            id: AudioId::new(),
            file_id: FileId::new(),
            file_name: self
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            stream_index: self
                .probe
                .as_ref()
                .and_then(|p| p.stream_index)
                .unwrap_or(0),
            metadata,
            language: None,
            transcript: None,
            duplicate_of: None,
            readability: self.readability.clone(),
            marks: Vec::new(),
            notes: Vec::new(),
        }
    }
}

/// Envanter sonucu.
#[derive(Debug, Clone)]
pub struct InventoryResult {
    /// Girdiler.
    pub entries: Vec<AudioInventoryEntry>,
    /// Toplam taranan dosya.
    pub scanned_files: usize,
    /// Ses içerdiği tespit edilen dosya.
    pub audio_files: usize,
    /// Transkripsiyona uygun dosya.
    pub transcribable: usize,
    /// Yinelenen dosya sayısı.
    pub duplicates: usize,
    /// Hatalı/okunamayan dosya.
    pub unreadable: usize,
    /// Envanter sırasında oluşan uyarılar (erişilemeyen yollar vb.).
    pub warnings: Vec<String>,
}

impl InventoryResult {
    /// Transkripsiyon kuyruğuna alınabilecek girdiler.
    pub fn transcribable_entries(&self) -> Vec<&AudioInventoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.readability.is_transcribable())
            .collect()
    }

    /// Okunamayan girdiler ve nedenleri.
    pub fn unreadable_entries(&self) -> Vec<(&AudioInventoryEntry, String)> {
        self.entries
            .iter()
            .filter(|e| !e.readability.is_transcribable())
            .map(|e| (e, e.readability.label()))
            .collect()
    }
}

/// Bir klasör veya dosya listesinden envanter üretir.
pub fn build_inventory(paths: &[PathBuf], compute_hash: bool) -> Result<InventoryResult> {
    let mut entries = Vec::new();
    let mut scanned = 0usize;
    let mut hash_index: HashMap<String, PathBuf> = HashMap::new();

    let mut warnings: Vec<String> = Vec::new();

    for root in paths {
        if !root.exists() {
            warnings.push(format!("Materyal yolu bulunamadı: {}", root.display()));
            continue;
        }
        let walker: Box<dyn Iterator<Item = walkdir::Result<walkdir::DirEntry>>> = if root.is_dir()
        {
            Box::new(WalkDir::new(root).follow_links(false).into_iter())
        } else {
            Box::new(WalkDir::new(root).into_iter())
        };

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warnings.push(format!("Dizin okunamadı: {e}"));
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            scanned += 1;
            let path = entry.path().to_path_buf();

            let detected = match birkat_evidence_detect(&path) {
                Ok(d) => d,
                Err(e) => {
                    entries.push(AudioInventoryEntry {
                        path: path.clone(),
                        detected: DetectedKind::Unknown,
                        probe: None,
                        sha256: None,
                        readability: Readability::Corrupt(format!("tür tespiti başarısız: {e}")),
                        duplicate_of: None,
                        warnings: Vec::new(),
                    });
                    continue;
                }
            };

            // Ses içerebilecek dosyaları incele.
            let is_audio = detected.may_contain_audio()
                || matches!(
                    detected,
                    DetectedKind::Audio(_) | DetectedKind::Container(_)
                );
            if !is_audio {
                continue;
            }

            let mut warnings = Vec::new();
            let sha256 = if compute_hash {
                match birkat_evidence_hash(&path) {
                    Ok(h) => Some(h),
                    Err(e) => {
                        warnings.push(format!("Hash hesaplanamadı: {e}"));
                        None
                    }
                }
            } else {
                None
            };

            let (probe, readability) = match probe_file(&path) {
                Ok(p) => {
                    let r = if p.has_audio_track {
                        Readability::Ok
                    } else {
                        Readability::UndecodableCodec("ses akışı yok".to_string())
                    };
                    (Some(p), r)
                }
                Err(e) => (None, Readability::UndecodableCodec(format!("{e}"))),
            };

            let mut duplicate_of = None;
            if let Some(h) = &sha256 {
                if let Some(first) = hash_index.get(h) {
                    duplicate_of = Some(first.clone());
                    warnings.push(format!(
                        "Bu dosya yinelenen içerik: {} ile aynı hash.",
                        first.display()
                    ));
                } else {
                    hash_index.insert(h.clone(), path.clone());
                }
            }

            entries.push(AudioInventoryEntry {
                path,
                detected,
                probe,
                sha256,
                readability,
                duplicate_of,
                warnings,
            });
        }
    }

    let audio_files = entries.len();
    let transcribable = entries
        .iter()
        .filter(|e| e.readability.is_transcribable())
        .count();
    let duplicates = entries.iter().filter(|e| e.duplicate_of.is_some()).count();
    let unreadable = entries
        .iter()
        .filter(|e| !e.readability.is_transcribable())
        .count();

    Ok(InventoryResult {
        entries,
        scanned_files: scanned,
        audio_files,
        transcribable,
        duplicates,
        unreadable,
        warnings,
    })
}

// birkat-evidence bağımlılığı olmadan tespit ve hash için ince sarmalayıcılar.
// (birkat-audio, birkat-evidence'a bağımlı olmadığından burada basit bir
// imza tabanlı tespit ve SHA-256 kullanılır.)

fn birkat_evidence_detect(path: &Path) -> std::io::Result<DetectedKind> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 4096];
    let n = std::io::Read::read(&mut f, &mut buf)?;
    buf.truncate(n);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    Ok(detect_signature(&buf, ext.as_deref()))
}

/// İmza tabanlı tespit (birkat-evidence::detect ile aynı kurallar).
pub fn detect_signature(data: &[u8], extension: Option<&str>) -> DetectedKind {
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
        let brand = &data[8..12];
        return match brand {
            b"M4A " | b"M4B " => DetectedKind::Audio("M4A".to_string()),
            b"qt  " => DetectedKind::Container("MOV".to_string()),
            _ => DetectedKind::Container("MP4".to_string()),
        };
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" {
        if &data[8..12] == b"WAVE" {
            return DetectedKind::Audio("WAV".to_string());
        }
        if &data[8..12] == b"AVI " {
            return DetectedKind::Container("AVI".to_string());
        }
    }
    if data.len() >= 4 && &data[..4] == b"OggS" {
        return DetectedKind::Audio("OGG".to_string());
    }
    if data.len() >= 4 && &data[..4] == b"fLaC" {
        return DetectedKind::Audio("FLAC".to_string());
    }
    if data.len() >= 3 && &data[..3] == b"ID3" {
        return DetectedKind::Audio("MP3".to_string());
    }
    if data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
        return DetectedKind::Audio("MP3".to_string());
    }
    if data.len() >= 4 && data[..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return DetectedKind::Container("MKV".to_string());
    }
    if data.len() >= 4 && data[..4] == *b"FORM" && data.len() >= 12 && &data[8..12] == b"AIFF" {
        return DetectedKind::Audio("AIFF".to_string());
    }
    if data.len() >= 4 && &data[..4] == b"caff" {
        return DetectedKind::Audio("CAF".to_string());
    }
    if data.len() >= 5 && &data[..5] == b"#!AMR" {
        return DetectedKind::Audio("AMR".to_string());
    }
    if data.len() >= 16
        && data[..16] == *b"\x30\x26\xB2\x75\x8E\x66\xCF\x11\xA6\xD9\x00\xAA\x00\x62\xCE\x6C"
    {
        return DetectedKind::Audio("WMA/ASF".to_string());
    }

    if let Some(ext) = extension {
        match ext {
            "wav" | "bwf" => return DetectedKind::Audio("WAV".to_string()),
            "mp3" => return DetectedKind::Audio("MP3".to_string()),
            "m4a" => return DetectedKind::Audio("M4A".to_string()),
            "aac" => return DetectedKind::Audio("AAC".to_string()),
            "flac" => return DetectedKind::Audio("FLAC".to_string()),
            "ogg" | "opus" => return DetectedKind::Audio("OGG".to_string()),
            "wma" => return DetectedKind::Audio("WMA".to_string()),
            "amr" => return DetectedKind::Audio("AMR".to_string()),
            "aiff" | "aif" => return DetectedKind::Audio("AIFF".to_string()),
            "caf" => return DetectedKind::Audio("CAF".to_string()),
            "mp4" => return DetectedKind::Container("MP4".to_string()),
            "mov" => return DetectedKind::Container("MOV".to_string()),
            "mkv" => return DetectedKind::Container("MKV".to_string()),
            "webm" => return DetectedKind::Container("WebM".to_string()),
            _ => {}
        }
    }
    DetectedKind::Unknown
}

fn birkat_evidence_hash(path: &Path) -> std::io::Result<String> {
    use sha2::Digest;
    let mut f = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = std::io::Read::read(&mut f, &mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::tests::write_test_wav;

    #[test]
    fn detects_audio_signatures() {
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&[0, 0, 0, 0]);
        wav.extend_from_slice(b"WAVEfmt ");
        assert_eq!(
            detect_signature(&wav, Some("wav")),
            DetectedKind::Audio("WAV".to_string())
        );
        assert_eq!(
            detect_signature(b"fLaC\x00", None),
            DetectedKind::Audio("FLAC".to_string())
        );
    }

    #[test]
    fn builds_inventory_for_wav_files() {
        let dir = tempfile::tempdir().unwrap();
        write_test_wav(&dir.path().join("a.wav"), 44100, 1, 0.2);
        write_test_wav(&dir.path().join("b.wav"), 48000, 2, 0.2);
        let r = build_inventory(&[dir.path().to_path_buf()], true).unwrap();
        assert_eq!(r.scanned_files, 2);
        assert_eq!(r.audio_files, 2);
        assert_eq!(r.transcribable, 2);
        assert_eq!(r.unreadable, 0);
    }

    #[test]
    fn duplicate_files_are_flagged_but_kept() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        let data = {
            write_test_wav(&a, 44100, 1, 0.1);
            std::fs::read(&a).unwrap()
        };
        std::fs::write(&b, &data).unwrap();
        let _ = b;
        let r = build_inventory(&[dir.path().to_path_buf()], true).unwrap();
        assert_eq!(r.audio_files, 2, "iki kaynak ilişkisi de korunmalı");
        assert_eq!(r.duplicates, 1);
    }

    #[test]
    fn unreadable_audio_is_reported_with_reason() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bozuk.mp3");
        std::fs::write(&p, b"ID3\x04\x00\x00\x00\x00\x00\x00bozuk veri").unwrap();
        let r = build_inventory(&[dir.path().to_path_buf()], false).unwrap();
        assert_eq!(r.audio_files, 1);
        assert_eq!(r.unreadable, 1);
        let (entry, reason) = &r.unreadable_entries()[0];
        assert!(!reason.is_empty());
        assert!(matches!(
            entry.readability,
            Readability::UndecodableCodec(_)
        ));
    }

    #[test]
    fn non_audio_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("belge.pdf"), b"%PDF-1.7\n").unwrap();
        std::fs::write(dir.path().join("metin.txt"), b"merhaba").unwrap();
        let r = build_inventory(&[dir.path().to_path_buf()], false).unwrap();
        assert_eq!(r.audio_files, 0);
        assert_eq!(r.scanned_files, 2);
    }

    #[test]
    fn audio_asset_conversion_includes_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("k.wav");
        write_test_wav(&p, 44100, 1, 0.1);
        let r = build_inventory(&[p], false).unwrap();
        let asset = r.entries[0].to_audio_asset();
        assert_eq!(asset.file_name, "k.wav");
        assert_eq!(asset.metadata.codec, "PCM");
        assert_eq!(asset.metadata.sample_rate, Some(44100));
    }

    #[test]
    fn empty_directory_produces_empty_inventory() {
        let dir = tempfile::tempdir().unwrap();
        let r = build_inventory(&[dir.path().to_path_buf()], false).unwrap();
        assert_eq!(r.audio_files, 0);
        assert_eq!(r.scanned_files, 0);
    }
}
