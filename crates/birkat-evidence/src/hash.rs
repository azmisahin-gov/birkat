//! Hash hesaplama (6.1).
//!
//! SHA-256 zorunludur. MD5 ve SHA-1 eski imaj formatlarıyla uyumluluk ve
//! karşılaştırma amacıyla tutulur; **güvenlik için SHA-256 esas alınır**.

use birkat_core::error::{CoreError, Result};
use birkat_core::evidence::HashAlgorithm;
use sha2::Digest;
use std::io::Read;
use std::path::Path;

/// Hesaplanan hash değeri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashDigest {
    /// Algoritma.
    pub algorithm: HashAlgorithm,
    /// Küçük harf onaltılık değer.
    pub value: String,
}

impl HashDigest {
    /// Onaltılık gösterim.
    pub fn hex(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Display for HashDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.algorithm.name(), self.value)
    }
}

/// Baytlardan tek bir algoritma ile hash hesaplar.
pub fn hash_bytes(data: &[u8], algorithm: HashAlgorithm) -> HashDigest {
    let value = match algorithm {
        HashAlgorithm::Sha256 => {
            let mut h = sha2::Sha256::new();
            h.update(data);
            hex::encode(h.finalize())
        }
        HashAlgorithm::Sha1 => {
            let mut h = sha1::Sha1::new();
            h.update(data);
            hex::encode(h.finalize())
        }
        HashAlgorithm::Md5 => {
            let mut h = md5::Md5::new();
            h.update(data);
            hex::encode(h.finalize())
        }
    };
    HashDigest { algorithm, value }
}

/// Baytlardan SHA-256, SHA-1 ve MD5 değerlerini hesaplar.
pub fn hash_bytes_all(data: &[u8]) -> Vec<HashDigest> {
    vec![
        hash_bytes(data, HashAlgorithm::Sha256),
        hash_bytes(data, HashAlgorithm::Sha1),
        hash_bytes(data, HashAlgorithm::Md5),
    ]
}

/// Dosyadan tek bir algoritma ile hash hesaplar (akış hâlinde, belleğe
/// tamamını yüklemeden).
pub fn hash_file(path: impl AsRef<Path>, algorithm: HashAlgorithm) -> Result<HashDigest> {
    let file = std::fs::File::open(path.as_ref())?;
    hash_reader(file, algorithm)
}

/// Bir `Read` kaynağından hash hesaplar.
pub fn hash_reader<R: Read>(mut reader: R, algorithm: HashAlgorithm) -> Result<HashDigest> {
    let mut buf = vec![0u8; 1024 * 1024];
    let value = match algorithm {
        HashAlgorithm::Sha256 => {
            let mut h = sha2::Sha256::new();
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            hex::encode(h.finalize())
        }
        HashAlgorithm::Sha1 => {
            let mut h = sha1::Sha1::new();
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            hex::encode(h.finalize())
        }
        HashAlgorithm::Md5 => {
            let mut h = md5::Md5::new();
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            hex::encode(h.finalize())
        }
    };
    Ok(HashDigest { algorithm, value })
}

/// Dosyadan SHA-256, SHA-1 ve MD5 değerlerini tek geçişte hesaplar.
pub fn hash_file_all(path: impl AsRef<Path>) -> Result<Vec<HashDigest>> {
    let mut file = std::fs::File::open(path.as_ref())?;
    let mut buf = vec![0u8; 1024 * 1024];
    let mut s256 = sha2::Sha256::new();
    let mut s1 = sha1::Sha1::new();
    let mut m5 = md5::Md5::new();
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        s256.update(&buf[..n]);
        s1.update(&buf[..n]);
        m5.update(&buf[..n]);
    }
    Ok(vec![
        HashDigest {
            algorithm: HashAlgorithm::Sha256,
            value: hex::encode(s256.finalize()),
        },
        HashDigest {
            algorithm: HashAlgorithm::Sha1,
            value: hex::encode(s1.finalize()),
        },
        HashDigest {
            algorithm: HashAlgorithm::Md5,
            value: hex::encode(m5.finalize()),
        },
    ])
}

/// İki dosyanın SHA-256 değerlerini karşılaştırır.
///
/// Kaynak ve hedef kopya karşılaştırması için kullanılır (5.4).
pub fn verify_identical(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<bool> {
    let a = hash_file(&source, HashAlgorithm::Sha256)?;
    let b = hash_file(&target, HashAlgorithm::Sha256)?;
    Ok(a.value.eq_ignore_ascii_case(&b.value))
}

/// Onaltılık bir hash metnini doğrular.
pub fn is_valid_hex_digest(value: &str, algorithm: HashAlgorithm) -> bool {
    let expected_len = match algorithm {
        HashAlgorithm::Sha256 => 64,
        HashAlgorithm::Sha1 => 40,
        HashAlgorithm::Md5 => 32,
    };
    value.len() == expected_len && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Hash değerini karşılaştırır (büyük/küçük harf duyarsız).
pub fn digest_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Doğrulama hatası üretir.
pub fn mismatch_error(expected: &str, actual: &str) -> CoreError {
    CoreError::Data(format!(
        "Hash uyuşmazlığı: beklenen {expected}, hesaplanan {actual}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn sha256_of_empty_is_known_value() {
        assert_eq!(hash_bytes(b"", HashAlgorithm::Sha256).value, EMPTY_SHA256);
    }

    #[test]
    fn sha256_of_hello_is_known_value() {
        assert_eq!(
            hash_bytes(b"hello", HashAlgorithm::Sha256).value,
            HELLO_SHA256
        );
    }

    #[test]
    fn md5_and_sha1_are_available_for_legacy_comparison() {
        let all = hash_bytes_all(b"hello");
        assert_eq!(all.len(), 3);
        assert_eq!(
            all.iter()
                .find(|d| d.algorithm == HashAlgorithm::Md5)
                .unwrap()
                .value,
            "5d41402abc4b2a76b9719d911017c592"
        );
        assert_eq!(
            all.iter()
                .find(|d| d.algorithm == HashAlgorithm::Sha1)
                .unwrap()
                .value,
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        );
    }

    #[test]
    fn file_hashing_matches_in_memory_hashing() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("veri.bin");
        let data = b"Bilirkisilik icin ornek veri".to_vec();
        std::fs::write(&p, &data).unwrap();
        assert_eq!(
            hash_file(&p, HashAlgorithm::Sha256).unwrap().value,
            hash_bytes(&data, HashAlgorithm::Sha256).value
        );
    }

    #[test]
    fn hash_file_all_matches_individual() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("v.bin");
        std::fs::write(&p, b"abc").unwrap();
        let all = hash_file_all(&p).unwrap();
        assert_eq!(
            all[0].value,
            hash_bytes(b"abc", HashAlgorithm::Sha256).value
        );
        assert_eq!(all[1].value, hash_bytes(b"abc", HashAlgorithm::Sha1).value);
        assert_eq!(all[2].value, hash_bytes(b"abc", HashAlgorithm::Md5).value);
    }

    #[test]
    fn identical_files_verify() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        std::fs::write(&a, "aynı içerik").unwrap();
        std::fs::write(&b, "aynı içerik").unwrap();
        assert!(verify_identical(&a, &b).unwrap());
    }

    #[test]
    fn different_files_do_not_verify() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        std::fs::write(&a, b"bir").unwrap();
        std::fs::write(&b, b"iki").unwrap();
        assert!(!verify_identical(&a, &b).unwrap());
    }

    #[test]
    fn hex_validation_checks_length_and_charset() {
        assert!(is_valid_hex_digest(EMPTY_SHA256, HashAlgorithm::Sha256));
        assert!(!is_valid_hex_digest("zz", HashAlgorithm::Sha256));
        assert!(!is_valid_hex_digest(EMPTY_SHA256, HashAlgorithm::Md5));
    }

    #[test]
    fn digest_comparison_is_case_insensitive() {
        assert!(digest_eq("ABC", "abc"));
    }
}
