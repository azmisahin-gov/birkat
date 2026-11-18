//! Gerçek dosya imzasına dayalı tür tespiti (4.1, 7).
//!
//! Dosya biçimi **yalnızca uzantıya göre değerlendirilmez**; gerçek dosya
//! imzası (magic bytes) da kontrol edilir.

use birkat_core::evidence::DetectedKind;
use std::path::Path;

/// Tür tespiti için okunacak azami başlık baytı.
pub const SNIFF_LEN: usize = 4096;

/// Dosyadan tür tespiti yapar.
pub fn detect_kind(path: impl AsRef<Path>) -> std::io::Result<DetectedKind> {
    let path = path.as_ref();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; SNIFF_LEN];
    let n = std::io::Read::read(&mut f, &mut buf)?;
    buf.truncate(n);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    Ok(detect_kind_from_bytes(&buf, ext.as_deref()))
}

/// Baytlardan tür tespiti yapar. `extension` ipucu olarak kullanılır.
pub fn detect_kind_from_bytes(data: &[u8], extension: Option<&str>) -> DetectedKind {
    if data.len() >= 4 {
        let head = &data[..4];
        // PDF
        if head == b"%PDF" {
            return DetectedKind::Pdf;
        }
        // JPEG
        if head[..2] == [0xFF, 0xD8] {
            return DetectedKind::Jpeg;
        }
        // PNG
        if head == [0x89, b'P', b'N', b'G'] {
            return DetectedKind::Png;
        }
        // RAR
        if head[..3] == *b"Rar" {
            return DetectedKind::Rar;
        }
        // 7z
        if head[..3] == [b'7', b'z', 0xBC] {
            return DetectedKind::SevenZip;
        }
        // TIFF (II*\0 veya MM\0*)
        if head == [b'I', b'I', 0x2A, 0x00] || head == [b'M', b'M', 0x00, 0x2A] {
            return DetectedKind::Tiff;
        }
        // OLE2 (DOC, XLS, PPT ...)
        if head == [0xD0, 0xCF, 0x11, 0xE0] {
            return DetectedKind::Doc;
        }
        // RIFF (WAV, AVI, WebP)
        if head == *b"RIFF" && data.len() >= 12 {
            let form = &data[8..12];
            if form == b"WAVE" {
                return DetectedKind::Audio("WAV".to_string());
            }
            if form == b"AVI " {
                return DetectedKind::Container("AVI".to_string());
            }
        }
        // OggS (OGG/Opus/Vorbis)
        if head == *b"OggS" {
            return DetectedKind::Audio("OGG".to_string());
        }
        // fLaC
        if head == *b"fLaC" {
            return DetectedKind::Audio("FLAC".to_string());
        }
        // FORM ... AIFF
        if head == *b"FORM" && data.len() >= 12 && &data[8..12] == b"AIFF" {
            return DetectedKind::Audio("AIFF".to_string());
        }
        // caff (CAF)
        if head == *b"caff" {
            return DetectedKind::Audio("CAF".to_string());
        }
        // EWF (EVF\x09\x0d\x0a\xff\x00)
        if data.len() >= 8 && &data[..8] == b"EVF\x09\x0d\x0a\xff\x00" {
            return DetectedKind::Ewf;
        }
    }

    // ISO Base Media (MP4/M4A/MOV/3GP): ftyp kutusu
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
        let brand = &data[8..12];
        return match brand {
            b"isom" | b"iso2" | b"mp41" | b"mp42" | b"M4V " | b"M4A " | b"M4B " | b"avc1"
            | b"dash" => {
                if brand.starts_with(b"M4") {
                    DetectedKind::Audio("M4A".to_string())
                } else {
                    DetectedKind::Container("MP4".to_string())
                }
            }
            b"qt  " => DetectedKind::Container("MOV".to_string()),
            b"3gp4" | b"3gp5" | b"3g2a" => DetectedKind::Audio("AMR/3GP".to_string()),
            _ => DetectedKind::Container("MP4".to_string()),
        };
    }

    // Matroska/WebM (EBML)
    if data.len() >= 4 && data[..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        // DocType okumaya çalış
        let doctype = find_ascii(data, b"webm").map(|_| "WebM").unwrap_or("MKV");
        return DetectedKind::Container(doctype.to_string());
    }

    // MP3: ID3 etiketi veya MPEG çerçeve eşitleme
    if data.len() >= 3 && &data[..3] == b"ID3" {
        return DetectedKind::Audio("MP3".to_string());
    }
    if data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
        return DetectedKind::Audio("MP3".to_string());
    }

    // AMR: "#!AMR"
    if data.len() >= 5 && &data[..5] == b"#!AMR" {
        return DetectedKind::Audio("AMR".to_string());
    }

    // ASF/WMA
    if data.len() >= 16
        && data[..16] == *b"\x30\x26\xB2\x75\x8E\x66\xCF\x11\xA6\xD9\x00\xAA\x00\x62\xCE\x6C"
    {
        return DetectedKind::Audio("WMA/ASF".to_string());
    }

    // ZIP tabanlı biçimler
    if data.len() >= 4 && data[..4] == [b'P', b'K', 0x03, 0x04] {
        // ZIP içeriğinden DOCX/ODT/UDF ayırt etmeye çalış
        if find_ascii(data, b"word/document.xml").is_some() {
            return DetectedKind::Docx;
        }
        if find_ascii(data, b"mimetypeapplication/vnd.oasis.opendocument").is_some()
            || find_ascii(data, b"content.xml").is_some()
        {
            // ODT ve UDF ikisi de content.xml içerir; uzantıya bak.
            if extension == Some("udf") {
                return DetectedKind::Udf;
            }
            return DetectedKind::Odt;
        }
        return DetectedKind::Zip;
    }

    // RTF
    if data.len() >= 5 && &data[..5] == b"{\\rtf" {
        return DetectedKind::Rtf;
    }

    // Uzantı ipuçları (imza tanınmadıysa)
    if let Some(ext) = extension {
        match ext {
            "udf" => return DetectedKind::Udf,
            "txt" | "md" | "csv" | "srt" | "vtt" | "json" | "xml" => {
                return DetectedKind::Text;
            }
            "docx" => return DetectedKind::Docx,
            "odt" => return DetectedKind::Odt,
            "rtf" => return DetectedKind::Rtf,
            "doc" => return DetectedKind::Doc,
            "pdf" => return DetectedKind::Pdf,
            "zip" => return DetectedKind::Zip,
            "7z" => return DetectedKind::SevenZip,
            "rar" => return DetectedKind::Rar,
            "jpg" | "jpeg" => return DetectedKind::Jpeg,
            "png" => return DetectedKind::Png,
            "tif" | "tiff" => return DetectedKind::Tiff,
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
            "e01" | "l01" | "ex01" | "lx01" => return DetectedKind::Ewf,
            "dd" | "raw" | "img" | "bin" => return DetectedKind::Raw,
            _ => {}
        }
    }

    // Metin mi?
    if !data.is_empty() && is_probably_text(data) {
        return DetectedKind::Text;
    }

    DetectedKind::Unknown
}

fn find_ascii(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Baytların büyük ölçüde yazdırılabilir metin olup olmadığını tahmin eder.
pub fn is_probably_text(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let sample = &data[..data.len().min(512)];
    let printable = sample
        .iter()
        .filter(|&&b| b == 0x09 || b == 0x0A || b == 0x0D || (0x20..0x7F).contains(&b) || b >= 0x80)
        .count();
    // NUL baytı varsa metin saymayız.
    if sample.contains(&0) {
        return false;
    }
    printable * 100 / sample.len() >= 90
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pdf() {
        assert_eq!(
            detect_kind_from_bytes(b"%PDF-1.7\n...", None),
            DetectedKind::Pdf
        );
    }

    #[test]
    fn detects_png_and_jpeg() {
        assert_eq!(
            detect_kind_from_bytes(&[0x89, b'P', b'N', b'G', 0x0D], None),
            DetectedKind::Png
        );
        assert_eq!(
            detect_kind_from_bytes(&[0xFF, 0xD8, 0xFF, 0xE0], None),
            DetectedKind::Jpeg
        );
    }

    #[test]
    fn detects_wav_from_riff_header() {
        let mut data = b"RIFF".to_vec();
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WAVEfmt ");
        assert_eq!(
            detect_kind_from_bytes(&data, Some("wav")),
            DetectedKind::Audio("WAV".to_string())
        );
    }

    #[test]
    fn detects_flac_ogg_mp3() {
        assert_eq!(
            detect_kind_from_bytes(b"fLaC\x00\x00", None),
            DetectedKind::Audio("FLAC".to_string())
        );
        assert_eq!(
            detect_kind_from_bytes(b"OggS\x00\x02", None),
            DetectedKind::Audio("OGG".to_string())
        );
        assert_eq!(
            detect_kind_from_bytes(b"ID3\x04\x00", None),
            DetectedKind::Audio("MP3".to_string())
        );
    }

    #[test]
    fn detects_mp4_ftyp_as_container() {
        let mut data = vec![0, 0, 0, 0x20];
        data.extend_from_slice(b"ftypisom");
        data.extend_from_slice(&[0u8; 8]);
        assert_eq!(
            detect_kind_from_bytes(&data, Some("mp4")),
            DetectedKind::Container("MP4".to_string())
        );
    }

    #[test]
    fn detects_m4a_from_brand() {
        let mut data = vec![0, 0, 0, 0x20];
        data.extend_from_slice(b"ftypM4A ");
        data.extend_from_slice(&[0u8; 8]);
        assert_eq!(
            detect_kind_from_bytes(&data, None),
            DetectedKind::Audio("M4A".to_string())
        );
    }

    #[test]
    fn detects_ewf_signature() {
        assert_eq!(
            detect_kind_from_bytes(b"EVF\x09\x0d\x0a\xff\x00", Some("e01")),
            DetectedKind::Ewf
        );
    }

    #[test]
    fn detects_zip_and_udf() {
        assert_eq!(
            detect_kind_from_bytes(b"PK\x03\x04xxxx", Some("zip")),
            DetectedKind::Zip
        );
        // UDF: ZIP imzası + content.xml adı
        let mut data = b"PK\x03\x04".to_vec();
        data.extend_from_slice(&[0u8; 20]);
        data.extend_from_slice(b"content.xml");
        assert_eq!(
            detect_kind_from_bytes(&data, Some("udf")),
            DetectedKind::Udf
        );
    }

    #[test]
    fn extension_is_only_a_hint() {
        // PDF içeriği ama .mp3 uzantısı: içerik kazanır.
        assert_eq!(
            detect_kind_from_bytes(b"%PDF-1.4", Some("mp3")),
            DetectedKind::Pdf
        );
    }

    #[test]
    fn detects_text_and_unknown() {
        assert_eq!(
            detect_kind_from_bytes("merhaba dünya".as_bytes(), None),
            DetectedKind::Text
        );
        assert_eq!(
            detect_kind_from_bytes(&[0x00, 0x01, 0x02, 0x03], None),
            DetectedKind::Unknown
        );
    }

    #[test]
    fn detects_docx_from_zip_content() {
        let mut data = b"PK\x03\x04".to_vec();
        data.extend_from_slice(&[0u8; 16]);
        data.extend_from_slice(b"word/document.xml");
        assert_eq!(
            detect_kind_from_bytes(&data, Some("docx")),
            DetectedKind::Docx
        );
    }

    #[test]
    fn real_file_detection_works() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("ornek.pdf");
        std::fs::write(&p, b"%PDF-1.7\nornek").unwrap();
        assert_eq!(detect_kind(&p).unwrap(), DetectedKind::Pdf);
    }
}
