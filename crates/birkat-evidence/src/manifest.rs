//! Edinim manifestosu (5.4, 6).
//!
//! Manifesto, edinilen dosyaların listesini, boyutlarını, hash değerlerini ve
//! doğrulama durumunu taşır. Manifestonun kendisi de bir hash ile
//! bütünlüğe bağlanır.

use birkat_core::evidence::AcquisitionMethod;
use birkat_core::time_util::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Manifestodaki tek bir dosya kaydı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Kaynağa göreli yol (eğik çizgi ile).
    pub relative_path: String,
    /// Boyut (bayt).
    pub size_bytes: u64,
    /// Kaynak SHA-256.
    pub sha256: String,
    /// Hedef kopya doğrulandı mı?
    pub verified: bool,
    /// Hata (varsa).
    pub error: Option<String>,
}

/// Edinim manifestosu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionManifest {
    /// Manifesto biçim sürümü.
    pub version: u32,
    /// Kaynak yolu.
    pub source: String,
    /// Hedef yolu.
    pub target: String,
    /// Yöntem.
    pub method: AcquisitionMethod,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
    /// Dosya kayıtları.
    pub entries: Vec<ManifestEntry>,
    /// Toplam boyut.
    pub total_bytes: u64,
    /// Başarılı sayısı.
    pub ok_count: usize,
    /// Hatalı sayısı.
    pub error_count: usize,
    /// Manifestonun kendi hash'i.
    pub manifest_sha256: Option<String>,
    /// Dürüstlük notu.
    pub note: String,
}

impl AcquisitionManifest {
    /// Yeni manifesto.
    pub fn new(source: String, target: String, method: AcquisitionMethod) -> Self {
        Self {
            version: 1,
            source,
            target,
            method,
            created_at: Timestamp::now(),
            entries: Vec::new(),
            total_bytes: 0,
            ok_count: 0,
            error_count: 0,
            manifest_sha256: None,
            note: String::new(),
        }
    }

    /// Özet alanlarını ve manifesto hash'ini hesaplar.
    pub fn finalize(&mut self) {
        self.total_bytes = self.entries.iter().map(|e| e.size_bytes).sum();
        self.ok_count = self
            .entries
            .iter()
            .filter(|e| e.error.is_none() && e.verified)
            .count();
        self.error_count = self.entries.iter().filter(|e| e.error.is_some()).count();
        self.note = crate::acquisition::metadata_limits_note().to_string();
        self.manifest_sha256 = Some(self.compute_hash());
    }

    /// Manifesto içeriğinin hash'ini hesaplar.
    ///
    /// Hash, manifesto hash alanı hariç tutularak hesaplanır.
    pub fn compute_hash(&self) -> String {
        let mut clone = self.clone();
        clone.manifest_sha256 = None;
        let json = serde_json::to_string(&clone).unwrap_or_default();
        let mut h = Sha256::new();
        h.update(json.as_bytes());
        hex::encode(h.finalize())
    }

    /// Manifestonun bütünlüğünü doğrular.
    pub fn verify(&self) -> bool {
        match &self.manifest_sha256 {
            Some(stored) => stored.eq_ignore_ascii_case(&self.compute_hash()),
            None => false,
        }
    }

    /// Hatalı girdiler.
    pub fn errors(&self) -> Vec<&ManifestEntry> {
        self.entries.iter().filter(|e| e.error.is_some()).collect()
    }

    /// Manifestoyu JSON olarak serileştirir.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// JSON'dan çözer.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AcquisitionManifest {
        let mut m = AcquisitionManifest::new(
            "/kaynak".into(),
            "/hedef".into(),
            AcquisitionMethod::FileBased,
        );
        m.entries.push(ManifestEntry {
            relative_path: "a.wav".into(),
            size_bytes: 100,
            sha256: "aa".repeat(32),
            verified: true,
            error: None,
        });
        m.entries.push(ManifestEntry {
            relative_path: "b.wav".into(),
            size_bytes: 200,
            sha256: String::new(),
            verified: false,
            error: Some("okuma hatası".into()),
        });
        m.finalize();
        m
    }

    #[test]
    fn finalize_computes_totals_and_hash() {
        let m = sample();
        assert_eq!(m.total_bytes, 300);
        assert_eq!(m.ok_count, 1);
        assert_eq!(m.error_count, 1);
        assert!(m.manifest_sha256.is_some());
        assert!(m.verify());
    }

    #[test]
    fn tampering_is_detected() {
        let mut m = sample();
        assert!(m.verify());
        m.entries[0].sha256 = "bb".repeat(32);
        assert!(!m.verify());
    }

    #[test]
    fn json_round_trip_preserves_hash() {
        let m = sample();
        let json = m.to_json();
        let back = AcquisitionManifest::from_json(&json).unwrap();
        assert!(back.verify());
        assert_eq!(back.entries.len(), 2);
    }

    #[test]
    fn errors_are_listed() {
        let m = sample();
        assert_eq!(m.errors().len(), 1);
        assert_eq!(m.errors()[0].relative_path, "b.wav");
    }
}
