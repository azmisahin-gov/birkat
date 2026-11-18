//! İçerik durumu — A.2 maddesinin teknik karşılığı.
//!
//! Belgede üç durum kesin olarak ayrılır:
//!
//! 1. **Makine çıktısı** ([`ContentStatus::MachineOutput`]): modelin veya
//!    otomatik işlemin ürettiği içerik.
//! 2. **İncelemeye hazır içerik** ([`ContentStatus::ReadyForReview`]):
//!    teknik kontrolleri yapılmış, fakat bilirkişi tarafından onaylanmamış.
//! 3. **Bilirkişi onaylı içerik** ([`ContentStatus::ExpertApproved`]):
//!    mesleki inceleme ve onay kaydedilmiş içerik.
//!
//! Sistem bu durumlar arasında yalnızca açık, denetlenebilir geçişlere izin
//! verir. Makine çıktısı hiçbir zaman kendiliğinden onaylı sayılamaz.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

/// İçeriğin mesleki inceleme durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContentStatus {
    /// Modelin veya otomatik işlemin ürettiği içerik.
    MachineOutput,
    /// Teknik kontrolden geçmiş, bilirkişi onayı bekleyen içerik.
    ReadyForReview,
    /// Bilirkişi tarafından incelenmiş ve onaylanmış içerik.
    ExpertApproved,
}

impl ContentStatus {
    /// Durumun Türkçe etiketi (arayüzde ve raporda kullanılır).
    pub fn label(&self) -> &'static str {
        match self {
            ContentStatus::MachineOutput => "Makine çıktısı",
            ContentStatus::ReadyForReview => "İncelemeye hazır içerik",
            ContentStatus::ExpertApproved => "Bilirkişi tarafından onaylanmış içerik",
        }
    }

    /// Bu durumdaki içerik rapora mesleki kanaat olarak girebilir mi?
    pub fn is_reportable_as_expert_opinion(&self) -> bool {
        matches!(self, ContentStatus::ExpertApproved)
    }

    /// Bu durumdaki içerik rapora taslak olarak girebilir mi?
    pub fn is_reportable_as_draft(&self) -> bool {
        !matches!(self, ContentStatus::MachineOutput)
    }

    /// Bu durumdaki içerik bilirkişi tarafından düzenlenebilir mi?
    pub fn is_editable(&self) -> bool {
        true
    }
}

/// İçerik durumu geçiş kuralları.
///
/// Amaç: makine çıktısının insan onayı gibi sunulmasını engellemek.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTransition {
    /// Teknik kalite kontrolü tamamlandı.
    QualityControlPassed,
    /// Bilirkişi incelemeyi tamamladı ve onayladı.
    ExpertApproved,
    /// Bilirkişi onayı geri çekti; içerik yeniden incelemeye döner.
    ApprovalWithdrawn,
    /// İçerik yeniden üretildi (model yeniden çalıştı).
    Regenerated,
}

/// Durum geçişini uygular ve kural ihlallerinde hata üretir.
///
/// Kurallar:
/// * Makine çıktısı doğrudan onaylanamaz; önce kalite kontrolünden geçmelidir.
/// * Onaylı içerik ancak açık geri çekme veya yeniden üretim ile durum değiştirir.
/// * Yeniden üretim her durumda içeriği makine çıktısına döndürür.
pub fn apply_transition(from: ContentStatus, t: StatusTransition) -> Result<ContentStatus> {
    use ContentStatus::*;
    use StatusTransition as T;

    let to = match (from, t) {
        // Kalite kontrolü: makine çıktısı -> incelemeye hazır
        (MachineOutput, T::QualityControlPassed) => ReadyForReview,
        // İncelemeye hazır içerik kalite kontrolünden geçmiş sayılır
        (ReadyForReview, T::QualityControlPassed) => ReadyForReview,
        // Onay: yalnızca incelemeye hazır içerik onaylanabilir
        (ReadyForReview, T::ExpertApproved) => ExpertApproved,
        // Onay geri çekme
        (ExpertApproved, T::ApprovalWithdrawn) => ReadyForReview,
        // Yeniden üretim: her durumda makine çıktısına döner
        (_, T::Regenerated) => MachineOutput,

        (MachineOutput, T::ExpertApproved) => {
            return Err(CoreError::ContentStatusRule(
                "Makine çıktısı, teknik kalite kontrolü tamamlanmadan bilirkişi onaylı sayılamaz."
                    .to_string(),
            ))
        }
        (from, t) => {
            return Err(CoreError::ContentStatusRule(format!(
                "{from:?} durumundan {t:?} geçişine izin verilmez."
            )))
        }
    };

    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::ContentStatus::*;
    use super::StatusTransition as T;
    use super::*;

    #[test]
    fn machine_output_cannot_be_approved_directly() {
        let err = apply_transition(MachineOutput, T::ExpertApproved).unwrap_err();
        assert!(matches!(err, CoreError::ContentStatusRule(_)));
    }

    #[test]
    fn quality_control_moves_to_ready_for_review() {
        assert_eq!(
            apply_transition(MachineOutput, T::QualityControlPassed).unwrap(),
            ReadyForReview
        );
    }

    #[test]
    fn ready_for_review_can_be_approved() {
        assert_eq!(
            apply_transition(ReadyForReview, T::ExpertApproved).unwrap(),
            ExpertApproved
        );
    }

    #[test]
    fn regeneration_always_returns_to_machine_output() {
        assert_eq!(
            apply_transition(ExpertApproved, T::Regenerated).unwrap(),
            MachineOutput
        );
    }

    #[test]
    fn withdrawal_returns_to_review() {
        assert_eq!(
            apply_transition(ExpertApproved, T::ApprovalWithdrawn).unwrap(),
            ReadyForReview
        );
    }

    #[test]
    fn only_expert_approved_is_reportable_as_opinion() {
        assert!(!MachineOutput.is_reportable_as_expert_opinion());
        assert!(!ReadyForReview.is_reportable_as_expert_opinion());
        assert!(ExpertApproved.is_reportable_as_expert_opinion());
    }

    #[test]
    fn draft_reporting_excludes_machine_output() {
        assert!(!MachineOutput.is_reportable_as_draft());
        assert!(ReadyForReview.is_reportable_as_draft());
        assert!(ExpertApproved.is_reportable_as_draft());
    }
}
