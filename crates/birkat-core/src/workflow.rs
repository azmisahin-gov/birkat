//! Kalıcı iş akışı motoru — 21.3 maddesi.
//!
//! Vaka yaşam döngüsü durumları burada tanımlanır. Her durum:
//! * Kalıcı kaydedilir.
//! * Tekrarlanabilir.
//! * İptal edilebilir / duraklatılabilir / devam edebilir.
//! * Hata sonrası güvenli şekilde yeniden denenebilir.
//! * Çift işlem oluşturmaz.
//! * Uygulama kapanınca kaybolmaz.
//!
//! Durum makinesi saf (pure) bir fonksiyon olarak yazılmıştır; böylece
//! birim testleri veritabanı gerektirmez.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Vaka iş akışı durumu (21.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkflowState {
    /// Yeni vaka.
    New,
    /// Görevlendirme içeri alındı.
    AssignmentImported,
    /// Görevlendirme inceleniyor.
    AssignmentReview,
    /// Materyal bekleniyor.
    MaterialsPending,
    /// Materyal teslim alındı.
    MaterialsReceived,
    /// Edinim işlemleri sürüyor.
    Acquisition,
    /// Teknik envanter çıkarılıyor.
    Inventory,
    /// Transkripsiyon kuyruğa alındı.
    TranscriptionQueued,
    /// Transkripsiyon çalışıyor.
    TranscriptionRunning,
    /// Otomatik kalite kontrolü.
    QualityControl,
    /// Bilirkişi incelemesi.
    ExpertReview,
    /// Sorular ve bulgular.
    QuestionsAndFindings,
    /// Rapor taslağı.
    ReportDraft,
    /// UDF üretimi.
    UdfGeneration,
    /// UDF doğrulaması.
    UdfValidation,
    /// Son kontrol.
    FinalReview,
    /// Onaylandı.
    Approved,
    /// İmza bekleniyor.
    SignaturePending,
    /// İmzalandı.
    Signed,
    /// Teslim bekleniyor.
    DeliveryPending,
    /// Teslim edildi.
    Submitted,
    /// Arşivlendi.
    Archived,
}

impl WorkflowState {
    /// Durumun Türkçe etiketi.
    pub fn label(&self) -> &'static str {
        match self {
            WorkflowState::New => "Yeni",
            WorkflowState::AssignmentImported => "Görevlendirme içeri alındı",
            WorkflowState::AssignmentReview => "Görevlendirme inceleniyor",
            WorkflowState::MaterialsPending => "Materyal bekleniyor",
            WorkflowState::MaterialsReceived => "Materyal teslim alındı",
            WorkflowState::Acquisition => "Edinim",
            WorkflowState::Inventory => "Teknik envanter",
            WorkflowState::TranscriptionQueued => "Transkripsiyon kuyruğa alındı",
            WorkflowState::TranscriptionRunning => "Transkripsiyon çalışıyor",
            WorkflowState::QualityControl => "Kalite kontrolü",
            WorkflowState::ExpertReview => "Bilirkişi incelemesi",
            WorkflowState::QuestionsAndFindings => "Sorular ve bulgular",
            WorkflowState::ReportDraft => "Rapor taslağı",
            WorkflowState::UdfGeneration => "UDF üretimi",
            WorkflowState::UdfValidation => "UDF doğrulaması",
            WorkflowState::FinalReview => "Son kontrol",
            WorkflowState::Approved => "Onaylandı",
            WorkflowState::SignaturePending => "İmza bekleniyor",
            WorkflowState::Signed => "İmzalandı",
            WorkflowState::DeliveryPending => "Teslim bekleniyor",
            WorkflowState::Submitted => "Teslim edildi",
            WorkflowState::Archived => "Arşivlendi",
        }
    }

    /// Normal ilerleme sırasındaki bir sonraki durum.
    ///
    /// Bazı durumlar (kalite kontrolü, bilirkişi incelemesi) tekrarlanabilir
    /// döngüler oluşturur; bu nedenle `next()` her zaman geri dönebilir.
    pub fn next(&self) -> Option<WorkflowState> {
        use WorkflowState::*;
        Some(match self {
            New => AssignmentImported,
            AssignmentImported => AssignmentReview,
            AssignmentReview => MaterialsPending,
            MaterialsPending => MaterialsReceived,
            MaterialsReceived => Acquisition,
            Acquisition => Inventory,
            Inventory => TranscriptionQueued,
            TranscriptionQueued => TranscriptionRunning,
            TranscriptionRunning => QualityControl,
            QualityControl => ExpertReview,
            ExpertReview => QuestionsAndFindings,
            QuestionsAndFindings => ReportDraft,
            ReportDraft => UdfGeneration,
            UdfGeneration => UdfValidation,
            UdfValidation => FinalReview,
            FinalReview => Approved,
            Approved => SignaturePending,
            SignaturePending => Signed,
            Signed => DeliveryPending,
            DeliveryPending => Submitted,
            Submitted => Archived,
            Archived => return None,
        })
    }

    /// Bu durum son (uç) durum mu?
    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Archived)
    }

    /// İleri/geri geçişler dahil izin verilen hedef durumlar.
    ///
    /// İş kuralı: süreç ileri gider; geri dönüş yalnızca teknik yeniden
    /// üretim gerektiren belirli durumlarda (kalite kontrolü başarısız,
    /// bilirkişi düzeltme istedi) mümkündür.
    pub fn allowed_transitions(&self) -> Vec<WorkflowState> {
        use WorkflowState::*;
        let mut v = Vec::new();
        if let Some(n) = self.next() {
            v.push(n);
        }
        // Geri dönüş yolları
        match self {
            QualityControl | ExpertReview | QuestionsAndFindings => {
                v.push(TranscriptionRunning);
                v.push(Inventory);
            }
            ReportDraft | UdfGeneration | UdfValidation | FinalReview => {
                v.push(QuestionsAndFindings);
            }
            SignaturePending => {
                v.push(ReportDraft);
            }
            DeliveryPending => {
                v.push(Signed);
            }
            _ => {}
        }
        v
    }
}

impl fmt::Display for WorkflowState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Durum geçişini doğrular.
pub fn transition(from: WorkflowState, to: WorkflowState) -> Result<WorkflowState> {
    if from == to {
        return Ok(to);
    }
    if from.allowed_transitions().contains(&to) {
        Ok(to)
    } else {
        Err(CoreError::InvalidTransition {
            from: from.to_string(),
            to: to.to_string(),
        })
    }
}

/// Vaka iş akışı kaydı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRecord {
    /// Mevcut durum.
    pub state: WorkflowState,
    /// Durum geçiş geçmişi.
    pub history: Vec<WorkflowHistoryEntry>,
    /// Duraklatıldı mı?
    pub paused: bool,
    /// İptal edildi mi?
    pub cancelled: bool,
}

/// Tek bir durum geçiş kaydı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowHistoryEntry {
    /// Önceki durum.
    pub from: WorkflowState,
    /// Yeni durum.
    pub to: WorkflowState,
    /// Geçiş zamanı.
    pub at: crate::time_util::Timestamp,
    /// Açıklama.
    pub note: Option<String>,
}

impl Default for WorkflowRecord {
    fn default() -> Self {
        Self {
            state: WorkflowState::New,
            history: Vec::new(),
            paused: false,
            cancelled: false,
        }
    }
}

impl WorkflowRecord {
    /// Yeni iş akışı kaydı.
    pub fn new() -> Self {
        Self::default()
    }

    /// Durumu ilerletir ve geçmişe kaydeder.
    ///
    /// Duraklatılmış veya iptal edilmiş iş akışları ilerletilemez.
    pub fn advance_to(&mut self, to: WorkflowState) -> Result<()> {
        if self.cancelled {
            return Err(CoreError::Validation(
                "İptal edilmiş iş akışı ilerletilemez.".to_string(),
            ));
        }
        if self.paused {
            return Err(CoreError::Validation(
                "Duraklatılmış iş akışı ilerletilemez; önce devam ettirilmelidir.".to_string(),
            ));
        }
        let from = self.state;
        let next = transition(from, to)?;
        self.state = next;
        self.history.push(WorkflowHistoryEntry {
            from,
            to: next,
            at: crate::time_util::Timestamp::now(),
            note: None,
        });
        Ok(())
    }

    /// Sıradaki duruma ilerletir.
    pub fn advance(&mut self) -> Result<WorkflowState> {
        let next = self.state.next().ok_or_else(|| {
            CoreError::Validation("Bu durumdan sonraki durum bulunmuyor.".to_string())
        })?;
        self.advance_to(next)?;
        Ok(self.state)
    }

    /// İş akışını duraklatır.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// İş akışını devam ettirir.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// İş akışını iptal eder.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::WorkflowState::*;
    use super::*;

    #[test]
    fn full_happy_path_reaches_archived() {
        let mut wf = WorkflowRecord::new();
        let mut steps = 0;
        while !wf.state.is_terminal() {
            wf.advance().unwrap();
            steps += 1;
            assert!(steps < 40, "döngüye girdi");
        }
        assert_eq!(wf.state, Archived);
        assert_eq!(wf.history.len(), steps);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let err = transition(New, Signed).unwrap_err();
        assert!(matches!(err, CoreError::InvalidTransition { .. }));
    }

    #[test]
    fn quality_failure_can_return_to_transcription() {
        assert!(QualityControl.allowed_transitions().contains(&TranscriptionRunning));
    }

    #[test]
    fn paused_workflow_cannot_advance() {
        let mut wf = WorkflowRecord::new();
        wf.pause();
        assert!(wf.advance().is_err());
        wf.resume();
        assert!(wf.advance().is_ok());
    }

    #[test]
    fn cancelled_workflow_cannot_advance() {
        let mut wf = WorkflowRecord::new();
        wf.cancel();
        assert!(wf.advance().is_err());
    }

    #[test]
    fn every_state_has_turkish_label() {
        for s in [
            New,
            AssignmentImported,
            AssignmentReview,
            MaterialsPending,
            MaterialsReceived,
            Acquisition,
            Inventory,
            TranscriptionQueued,
            TranscriptionRunning,
            QualityControl,
            ExpertReview,
            QuestionsAndFindings,
            ReportDraft,
            UdfGeneration,
            UdfValidation,
            FinalReview,
            Approved,
            SignaturePending,
            Signed,
            DeliveryPending,
            Submitted,
            Archived,
        ] {
            assert!(!s.label().is_empty());
        }
    }
}
