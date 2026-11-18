//! # BİRKÂT işlem geçmişi (audit log) ve hash zinciri
//!
//! 6.2 maddesi uyarınca aşağıdaki işlemler kaydedilir: vaka oluşturma,
//! materyal ekleme/çıkarma, hash hesaplama, imaj alma, yazma koruması,
//! transkripsiyon, model değişikliği, transkript düzeltmesi, rapor
//! oluşturma/düzenleme, UDF üretme, onay, imzalama, dışa aktarma, teslim,
//! arşivleme ve geri yükleme.
//!
//! ## Bütünlük sınırı (dürüstlük notu)
//!
//! Yerel hash zinciri sonradan yapılabilecek değişiklikleri tespit etmeye
//! yardım eder. Ancak **harici güvenilir zaman damgası veya nitelikli
//! elektronik imza olmadan bağımsız ve değiştirilemez zaman damgası olarak
//! sunulamaz.** Bu sınır [`ChainVerification::is_independent_timestamp`]
//! ile açıkça taşınır.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use birkat_core::ids::{AuditEventId, CaseId};
use birkat_core::time_util::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Zincirin ilk kaydından önceki hash değeri (64 sıfır).
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Denetim olayı türü (6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    /// Vaka oluşturma.
    CaseCreated,
    /// Materyal ekleme.
    MaterialAdded,
    /// Materyal çıkarma.
    MaterialRemoved,
    /// Hash hesaplama.
    HashComputed,
    /// İmaj alma.
    ImageAcquired,
    /// Yazma koruması.
    WriteProtection,
    /// Transkripsiyon.
    Transcription,
    /// Model değişikliği.
    ModelChanged,
    /// Transkript düzeltmesi.
    TranscriptEdited,
    /// Rapor oluşturma.
    ReportGenerated,
    /// Rapor düzenleme.
    ReportEdited,
    /// UDF üretme.
    UdfGenerated,
    /// UDF doğrulama.
    UdfValidated,
    /// Onay.
    Approved,
    /// İmzalama.
    Signed,
    /// Dışa aktarma.
    Exported,
    /// Teslim.
    Delivered,
    /// Arşivleme.
    Archived,
    /// Geri yükleme.
    Restored,
    /// Yedekleme.
    BackedUp,
    /// Güncelleme.
    Updated,
    /// Güvenlik olayı (başarısız giriş, kilit).
    Security,
    /// Kapsam uyarısı.
    ScopeWarning,
}

impl AuditAction {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            AuditAction::CaseCreated => "Vaka oluşturma",
            AuditAction::MaterialAdded => "Materyal ekleme",
            AuditAction::MaterialRemoved => "Materyal çıkarma",
            AuditAction::HashComputed => "Hash hesaplama",
            AuditAction::ImageAcquired => "İmaj alma",
            AuditAction::WriteProtection => "Yazma koruması",
            AuditAction::Transcription => "Transkripsiyon",
            AuditAction::ModelChanged => "Model değişikliği",
            AuditAction::TranscriptEdited => "Transkript düzeltmesi",
            AuditAction::ReportGenerated => "Rapor oluşturma",
            AuditAction::ReportEdited => "Rapor düzenleme",
            AuditAction::UdfGenerated => "UDF üretme",
            AuditAction::UdfValidated => "UDF doğrulama",
            AuditAction::Approved => "Onay",
            AuditAction::Signed => "İmzalama",
            AuditAction::Exported => "Dışa aktarma",
            AuditAction::Delivered => "Teslim",
            AuditAction::Archived => "Arşivleme",
            AuditAction::Restored => "Geri yükleme",
            AuditAction::BackedUp => "Yedekleme",
            AuditAction::Updated => "Güncelleme",
            AuditAction::Security => "Güvenlik olayı",
            AuditAction::ScopeWarning => "Kapsam uyarısı",
        }
    }
}

/// Tek bir denetim olayı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Sıra numarası (1 tabanlı).
    pub sequence: u64,
    /// Kimlik.
    pub id: AuditEventId,
    /// Vaka.
    pub case: Option<CaseId>,
    /// İşlem türü.
    pub action: AuditAction,
    /// İşlemi yapan kullanıcı.
    pub actor: String,
    /// Açıklama.
    pub description: String,
    /// İlgili varlık (dosya, rapor vb.).
    pub subject: Option<String>,
    /// Ek veri (JSON).
    pub details: serde_json::Value,
    /// Zaman.
    pub at: Timestamp,
    /// Önceki kaydın hash'i.
    pub previous_hash: String,
    /// Bu kaydın hash'i.
    pub hash: String,
}

/// Hash zinciri.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditChain {
    /// Olaylar (sıralı).
    pub events: Vec<AuditEvent>,
}

/// Zincir doğrulama sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// Zincir geçerli mi?
    pub valid: bool,
    /// Doğrulanan olay sayısı.
    pub checked: usize,
    /// İlk bozuk kaydın sıra numarası.
    pub first_bad_sequence: Option<u64>,
    /// Açıklama.
    pub message: String,
    /// Bu doğrulama bağımsız zaman damgası yerine geçer mi?
    ///
    /// Yerel zincir tek başına değiştirilemez zaman damgası kanıtı değildir.
    pub is_independent_timestamp: bool,
}

/// Bir olayın hash'ini hesaplar.
///
/// Hash; sıra, kimlik, işlem, aktör, açıklama, konu, detay, zaman ve önceki
/// hash alanlarının kanonik birleşimi üzerinden alınır.
pub fn compute_event_hash(event: &AuditEvent) -> String {
    let mut hasher = Sha256::new();
    hasher.update(event.sequence.to_le_bytes());
    hasher.update(event.id.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(format!("{:?}", event.action).as_bytes());
    hasher.update(b"|");
    hasher.update(event.actor.as_bytes());
    hasher.update(b"|");
    hasher.update(event.description.as_bytes());
    hasher.update(b"|");
    hasher.update(event.subject.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(event.details.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(event.at.to_rfc3339().as_bytes());
    hasher.update(b"|");
    hasher.update(event.previous_hash.as_bytes());
    hex::encode(hasher.finalize())
}

impl AuditChain {
    /// Boş zincir oluşturur.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Zincirin son hash'i.
    pub fn head_hash(&self) -> &str {
        self.events
            .last()
            .map(|e| e.hash.as_str())
            .unwrap_or(GENESIS_HASH)
    }

    /// Zincire yeni olay ekler ve hash'ini hesaplar.
    pub fn append(
        &mut self,
        action: AuditAction,
        actor: impl Into<String>,
        description: impl Into<String>,
        case: Option<CaseId>,
        subject: Option<String>,
        details: serde_json::Value,
    ) -> &AuditEvent {
        let sequence = self.events.len() as u64 + 1;
        let mut event = AuditEvent {
            sequence,
            id: AuditEventId::new(),
            case,
            action,
            actor: actor.into(),
            description: description.into(),
            subject,
            details,
            at: Timestamp::now(),
            previous_hash: self.head_hash().to_string(),
            hash: String::new(),
        };
        event.hash = compute_event_hash(&event);
        self.events.push(event);
        self.events.last().expect("az önce eklendi")
    }

    /// Zinciri baştan sona doğrular.
    pub fn verify(&self) -> ChainVerification {
        let mut expected_previous = GENESIS_HASH.to_string();
        for (i, event) in self.events.iter().enumerate() {
            let expected_seq = i as u64 + 1;
            if event.sequence != expected_seq {
                return ChainVerification {
                    valid: false,
                    checked: i,
                    first_bad_sequence: Some(event.sequence),
                    message: format!(
                        "Sıra numarası kopuk: beklenen {expected_seq}, bulunan {}.",
                        event.sequence
                    ),
                    is_independent_timestamp: false,
                };
            }
            if event.previous_hash != expected_previous {
                return ChainVerification {
                    valid: false,
                    checked: i,
                    first_bad_sequence: Some(event.sequence),
                    message: format!(
                        "{} numaralı kaydın önceki hash bağı kopuk.",
                        event.sequence
                    ),
                    is_independent_timestamp: false,
                };
            }
            let recomputed = compute_event_hash(event);
            if recomputed != event.hash {
                return ChainVerification {
                    valid: false,
                    checked: i,
                    first_bad_sequence: Some(event.sequence),
                    message: format!(
                        "{} numaralı kaydın içeriği değiştirilmiş (hash uyuşmuyor).",
                        event.sequence
                    ),
                    is_independent_timestamp: false,
                };
            }
            expected_previous = event.hash.clone();
        }

        ChainVerification {
            valid: true,
            checked: self.events.len(),
            first_bad_sequence: None,
            message: "İşlem geçmişi hash zinciri bütünlüğü doğrulandı.".to_string(),
            is_independent_timestamp: false,
        }
    }

    /// Belirli bir işlem türüne göre filtreler.
    pub fn filter_action(&self, action: AuditAction) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| e.action == action).collect()
    }

    /// Bir vakanın olaylarını döndürür.
    pub fn for_case(&self, case: &CaseId) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.case.as_ref() == Some(case))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_with(n: usize) -> AuditChain {
        let mut c = AuditChain::new();
        for i in 0..n {
            c.append(
                AuditAction::CaseCreated,
                "bilirkişi",
                format!("olay {i}"),
                None,
                None,
                serde_json::json!({ "index": i }),
            );
        }
        c
    }

    #[test]
    fn empty_chain_is_valid() {
        let v = AuditChain::new().verify();
        assert!(v.valid);
        assert_eq!(v.checked, 0);
    }

    #[test]
    fn appended_chain_is_valid() {
        let c = chain_with(5);
        let v = c.verify();
        assert!(v.valid, "{}", v.message);
        assert_eq!(v.checked, 5);
    }

    #[test]
    fn first_event_links_to_genesis() {
        let c = chain_with(1);
        assert_eq!(c.events[0].previous_hash, GENESIS_HASH);
    }

    #[test]
    fn each_event_links_to_previous() {
        let c = chain_with(4);
        for w in c.events.windows(2) {
            assert_eq!(w[1].previous_hash, w[0].hash);
        }
    }

    #[test]
    fn tampering_with_description_is_detected() {
        let mut c = chain_with(3);
        c.events[1].description = "değiştirilmiş".to_string();
        let v = c.verify();
        assert!(!v.valid);
        assert_eq!(v.first_bad_sequence, Some(2));
    }

    #[test]
    fn removing_an_event_is_detected() {
        let mut c = chain_with(3);
        c.events.remove(1);
        let v = c.verify();
        assert!(!v.valid);
    }

    #[test]
    fn reordering_events_is_detected() {
        let mut c = chain_with(3);
        c.events.swap(0, 2);
        assert!(!c.verify().valid);
    }

    #[test]
    fn verification_is_not_an_independent_timestamp() {
        let v = chain_with(2).verify();
        assert!(v.valid);
        assert!(!v.is_independent_timestamp);
    }

    #[test]
    fn filter_by_action_and_case() {
        let mut c = AuditChain::new();
        let case = CaseId::new();
        c.append(
            AuditAction::CaseCreated,
            "a",
            "vaka",
            Some(case.clone()),
            None,
            serde_json::json!({}),
        );
        c.append(
            AuditAction::HashComputed,
            "a",
            "hash",
            Some(case.clone()),
            None,
            serde_json::json!({}),
        );
        assert_eq!(c.for_case(&case).len(), 2);
        assert_eq!(c.filter_action(AuditAction::HashComputed).len(), 1);
    }

    #[test]
    fn all_actions_have_turkish_labels() {
        for a in [
            AuditAction::CaseCreated,
            AuditAction::MaterialAdded,
            AuditAction::MaterialRemoved,
            AuditAction::HashComputed,
            AuditAction::ImageAcquired,
            AuditAction::WriteProtection,
            AuditAction::Transcription,
            AuditAction::ModelChanged,
            AuditAction::TranscriptEdited,
            AuditAction::ReportGenerated,
            AuditAction::ReportEdited,
            AuditAction::UdfGenerated,
            AuditAction::UdfValidated,
            AuditAction::Approved,
            AuditAction::Signed,
            AuditAction::Exported,
            AuditAction::Delivered,
            AuditAction::Archived,
            AuditAction::Restored,
            AuditAction::BackedUp,
            AuditAction::Updated,
            AuditAction::Security,
            AuditAction::ScopeWarning,
        ] {
            assert!(!a.label().is_empty());
        }
    }
}
