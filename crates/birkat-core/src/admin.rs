//! Takvim, ücret, gider ve heyet çalışması — 17 ve 18. bölümler.

use crate::ids::*;
use crate::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Ücret kaydı (18.2).
///
/// Portalda görülen tutarlar uygulama içinde resmî veriden alınmış gibi
/// gösterilmez; kullanıcı tarafından girilen tutar ile resmî portal bilgisi
/// ayrı kaynak türleri olarak saklanır.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecord {
    /// Kimlik.
    pub id: FeeId,
    /// Vaka.
    pub case: CaseId,
    /// Tutar (kuruş).
    pub amount_kurus: i64,
    /// Para birimi.
    pub currency: String,
    /// Kaynak türü.
    pub source: AmountSource,
    /// Ücretin belirlenme tarihi.
    pub determined_at: Option<Timestamp>,
    /// Tahakkuk bilgisi.
    pub accrued: bool,
    /// Ödeme bilgisi.
    pub paid: bool,
    /// Ödeme tarihi.
    pub paid_at: Option<Timestamp>,
    /// Uygulanan tarife sürümü.
    pub tariff_version: Option<String>,
    /// Tarifenin yürürlük tarihi.
    pub tariff_effective_from: Option<Timestamp>,
    /// Not.
    pub note: Option<String>,
}

/// Tutar kaynağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AmountSource {
    /// Bilirkişi tarafından girilen tutar.
    UserEntered,
    /// Resmî portalda görülen tutar (kullanıcı tarafından aktarıldı).
    PortalObserved,
    /// Tarife hesabı.
    TariffCalculated,
}

impl AmountSource {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            AmountSource::UserEntered => "Kullanıcı tarafından girilen",
            AmountSource::PortalObserved => "Portalda görülen (kullanıcı aktarımı)",
            AmountSource::TariffCalculated => "Tarifeye göre hesaplanan",
        }
    }
}

/// Gider kaydı (18.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expense {
    /// Kimlik.
    pub id: ExpenseId,
    /// Vaka.
    pub case: CaseId,
    /// Gider türü.
    pub kind: ExpenseKind,
    /// Tutar (kuruş).
    pub amount_kurus: i64,
    /// Para birimi.
    pub currency: String,
    /// Tarih.
    pub at: Timestamp,
    /// Açıklama.
    pub description: String,
    /// Belge/fatura ekleri.
    pub attachments: Vec<AttachmentId>,
}

/// Gider türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpenseKind {
    /// Yol.
    Travel,
    /// Konaklama.
    Accommodation,
    /// Diğer.
    Other,
}

impl ExpenseKind {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            ExpenseKind::Travel => "Yol gideri",
            ExpenseKind::Accommodation => "Konaklama gideri",
            ExpenseKind::Other => "Diğer gider",
        }
    }
}

/// Bilirkişi ücret tarifesi (18.2).
///
/// Tarifeler sürüm ve yürürlük tarihiyle tutulur; eski dosyaların hesapları
/// yeni tarife geldi diye geriye dönük değişmez.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTariff {
    /// Tarife sürümü.
    pub version: String,
    /// Yürürlük başlangıcı.
    pub effective_from: Timestamp,
    /// Yürürlük bitişi (yeni tarife geldiyse).
    pub effective_until: Option<Timestamp>,
    /// Açıklama.
    pub description: String,
    /// Sabit taban ücret (kuruş).
    pub base_amount_kurus: i64,
    /// Kademeler: (üst sınır dakika, dakika başına ücret kuruş).
    pub tiers: Vec<TariffTier>,
}

/// Tarife kademesi.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TariffTier {
    /// Bu kademenin kapsadığı üst sınır (dakika); `None` = sınırsız.
    pub up_to_minutes: Option<u32>,
    /// Dakika başına ücret (kuruş).
    pub per_minute_kurus: i64,
}

impl FeeTariff {
    /// Belirli bir süre için ücreti hesaplar (kuruş).
    ///
    /// Hesaplama, kademelerin artan sırada tanımlandığını varsayar.
    pub fn calculate_kurus(&self, minutes: u32) -> i64 {
        let mut total = self.base_amount_kurus;
        let mut remaining = minutes;
        let mut previous_limit = 0u32;

        for tier in &self.tiers {
            if remaining == 0 {
                break;
            }
            let span = match tier.up_to_minutes {
                Some(limit) => limit.saturating_sub(previous_limit),
                None => remaining,
            };
            let used = remaining.min(span);
            total += used as i64 * tier.per_minute_kurus;
            remaining -= used;
            if let Some(limit) = tier.up_to_minutes {
                previous_limit = limit;
            }
        }
        total
    }

    /// Tarife belirli bir tarihte yürürlükte mi?
    pub fn is_effective_at(&self, at: Timestamp) -> bool {
        at >= self.effective_from && self.effective_until.map(|u| at < u).unwrap_or(true)
    }
}

/// Takvim etkinliği (18.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Vaka.
    pub case: Option<CaseId>,
    /// Başlık.
    pub title: String,
    /// Zaman.
    pub at: Timestamp,
    /// Tür.
    pub kind: crate::case::DeadlineKind,
    /// Tamamlandı mı?
    pub completed: bool,
    /// Hatırlatma günleri.
    pub remind_days_before: Vec<u32>,
}

/// İş performans kayıtları (18.3).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Dosya sayısı.
    pub case_count: u32,
    /// İnceleme süresi (saniye).
    pub review_seconds: u64,
    /// Transkripsiyon süresi (saniye).
    pub transcription_seconds: u64,
    /// Rapor hazırlama süresi (saniye).
    pub reporting_seconds: u64,
    /// Bekleyen dosyalar.
    pub pending_cases: u32,
    /// Teslim edilen dosyalar.
    pub delivered_cases: u32,
    /// Ek raporlar.
    pub supplementary_reports: u32,
}

/// Heyet üyesi görevi (17).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelMemberAssignment {
    /// Üye.
    pub member: ExpertProfileId,
    /// Görev tanımı.
    pub role: PanelRole,
    /// Kişi bazında notlar.
    pub notes: Vec<String>,
    /// Kişi bazında onay verildi mi?
    pub approved: bool,
    /// Onay zamanı.
    pub approved_at: Option<Timestamp>,
}

/// Heyetteki rol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PanelRole {
    /// Heyet başkanı.
    Chair,
    /// Üye.
    Member,
    /// Raportör.
    Rapporteur,
    /// Bilirkişi (tek).
    SoleExpert,
}

impl PanelRole {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            PanelRole::Chair => "Heyet başkanı",
            PanelRole::Member => "Üye",
            PanelRole::Rapporteur => "Raportör",
            PanelRole::SoleExpert => "Tek bilirkişi",
        }
    }
}

/// Çakışan değişiklikleri birleştirme sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeOutcome {
    /// Çakışmasız birleştirilen alanlar.
    pub merged: Vec<String>,
    /// Çakışan alanlar (kullanıcı kararı gerekir).
    pub conflicts: Vec<MergeConflict>,
}

/// Birleştirme çakışması.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConflict {
    /// Alan yolu.
    pub field: String,
    /// Yerel değer.
    pub local: String,
    /// Gelen değer.
    pub incoming: String,
}

/// Basit üç yönlü metin birleştirme.
///
/// Amaç: iki bilirkişinin aynı alanda yaptığı değişikliklerin sessizce
/// kaybolmaması. Yalnızca bir taraf değiştiyse o değişiklik kabul edilir;
/// iki taraf da farklı değiştirdiyse çakışma bildirilir.
pub fn merge_text(base: &str, local: &str, incoming: &str) -> MergeOutcome {
    if local == incoming {
        return MergeOutcome {
            merged: vec!["değişiklik yok".to_string()],
            conflicts: Vec::new(),
        };
    }
    if base == local {
        return MergeOutcome {
            merged: vec![incoming.to_string()],
            conflicts: Vec::new(),
        };
    }
    if base == incoming {
        return MergeOutcome {
            merged: vec![local.to_string()],
            conflicts: Vec::new(),
        };
    }
    MergeOutcome {
        merged: Vec::new(),
        conflicts: vec![MergeConflict {
            field: "metin".to_string(),
            local: local.to_string(),
            incoming: incoming.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tariff() -> FeeTariff {
        FeeTariff {
            version: "2025/1".into(),
            effective_from: Timestamp::parse("2025-01-01T00:00:00Z").unwrap(),
            effective_until: None,
            description: "Test tarifesi".into(),
            base_amount_kurus: 10_000,
            tiers: vec![
                TariffTier {
                    up_to_minutes: Some(60),
                    per_minute_kurus: 100,
                },
                TariffTier {
                    up_to_minutes: None,
                    per_minute_kurus: 50,
                },
            ],
        }
    }

    #[test]
    fn tariff_calculates_tiered_amount() {
        let t = tariff();
        // 30 dakika: 10000 + 30*100 = 13000
        assert_eq!(t.calculate_kurus(30), 13_000);
        // 90 dakika: 10000 + 60*100 + 30*50 = 17500
        assert_eq!(t.calculate_kurus(90), 17_500);
    }

    #[test]
    fn tariff_zero_minutes_returns_base() {
        assert_eq!(tariff().calculate_kurus(0), 10_000);
    }

    #[test]
    fn tariff_effectivity_window() {
        let t = tariff();
        assert!(t.is_effective_at(Timestamp::parse("2025-06-01T00:00:00Z").unwrap()));
        assert!(!t.is_effective_at(Timestamp::parse("2024-06-01T00:00:00Z").unwrap()));
    }

    #[test]
    fn amount_sources_are_distinct() {
        assert_ne!(
            AmountSource::UserEntered.label(),
            AmountSource::PortalObserved.label()
        );
    }

    #[test]
    fn merge_accepts_one_sided_change() {
        let r = merge_text("a", "a", "b");
        assert!(r.conflicts.is_empty());
        assert_eq!(r.merged[0], "b");
    }

    #[test]
    fn merge_reports_conflict_when_both_changed() {
        let r = merge_text("a", "b", "c");
        assert_eq!(r.conflicts.len(), 1);
        assert_eq!(r.conflicts[0].local, "b");
        assert_eq!(r.conflicts[0].incoming, "c");
    }

    #[test]
    fn panel_roles_have_labels() {
        assert_eq!(PanelRole::Chair.label(), "Heyet başkanı");
        assert_eq!(PanelRole::SoleExpert.label(), "Tek bilirkişi");
    }
}
