//! Ana kontrol paneli verisi (2.1).
//!
//! Ana ekranda:
//!
//! * Yeni görevlendirme.
//! * Devam eden dosyalar.
//! * İnceleme bekleyen istisnalar.
//! * Rapor kontrolü bekleyen dosyalar.
//! * Teslim ve arşiv durumu.
//! * Yaklaşan görev süreleri.
//!
//! görülür.

use crate::store::CaseStore;
use birkat_core::case::Case;
use birkat_core::error::Result;
use birkat_core::time_util::{humanize_duration, Timestamp};
use birkat_core::workflow::WorkflowState;
use serde::{Deserialize, Serialize};

/// Paneldeki bir vaka satırı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardCaseRow {
    /// Vaka kimliği.
    pub case_id: String,
    /// Başlık.
    pub title: String,
    /// Dosya numarası.
    pub file_number: Option<String>,
    /// Durum etiketi.
    pub state: String,
    /// Son teslim tarihi.
    pub due_date: Option<Timestamp>,
    /// Son teslim tarihine kalan süre (Türkçe).
    pub due_in: Option<String>,
    /// Süre gecikmiş mi?
    pub overdue: bool,
    /// İnceleme bekleyen istisna sayısı.
    pub pending_exceptions: usize,
    /// Son güncelleme.
    pub updated_at: Timestamp,
}

/// Ana kontrol paneli verisi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Devam eden dosyalar.
    pub active_cases: Vec<DashboardCaseRow>,
    /// İnceleme bekleyen istisnalar (uyarı metinleri).
    pub pending_exceptions: Vec<(String, String)>,
    /// Rapor kontrolü bekleyen dosyalar.
    pub awaiting_report_review: Vec<DashboardCaseRow>,
    /// Teslim bekleyen dosyalar.
    pub awaiting_delivery: Vec<DashboardCaseRow>,
    /// Arşivlenmiş dosya sayısı.
    pub archived_count: u32,
    /// Yaklaşan görev süreleri.
    pub upcoming_deadlines: Vec<(String, String)>,
    /// Toplam dosya sayısı.
    pub total_cases: u32,
    /// Panel oluşturma zamanı.
    pub generated_at: Timestamp,
}

impl DashboardData {
    /// Türkçe özet satırı.
    pub fn summary(&self) -> String {
        format!(
            "{} dosya, {} devam ediyor, {} rapor kontrolü bekliyor, {} teslim bekliyor.",
            self.total_cases,
            self.active_cases.len(),
            self.awaiting_report_review.len(),
            self.awaiting_delivery.len()
        )
    }

    /// Yaklaşan süre var mı?
    pub fn has_upcoming_deadlines(&self) -> bool {
        !self.upcoming_deadlines.is_empty()
    }

    /// Gecikmiş dosya var mı?
    pub fn has_overdue(&self) -> bool {
        self.active_cases.iter().any(|c| c.overdue)
    }
}

/// Panel verisini depodan üretir.
pub fn build_dashboard(store: &CaseStore, deadline_horizon_days: i64) -> Result<DashboardData> {
    let cases = store.list_cases()?;
    let now = Timestamp::now();

    let mut active = Vec::new();
    let mut pending_exceptions = Vec::new();
    let mut awaiting_review = Vec::new();
    let mut awaiting_delivery = Vec::new();
    let mut upcoming = Vec::new();
    let mut archived_count = 0u32;

    for c in &cases {
        if c.archived || c.workflow.state == WorkflowState::Archived {
            archived_count += 1;
            continue;
        }

        let row = make_row(c, now);

        // İnceleme bekleyen istisnalar: yanıtlanmamış sorular, kapsam uyarıları.
        for w in c.preflight_warnings() {
            if w.contains("yanıtlanmadı") || w.contains("kapsam dışı") {
                pending_exceptions.push((c.title.clone(), w));
            }
        }
        for sw in &c.assignment.scope_warnings {
            if !sw.acknowledged {
                pending_exceptions.push((
                    c.title.clone(),
                    format!("Kapsam uyarısı: {}", sw.matched_text),
                ));
            }
        }

        match c.workflow.state {
            WorkflowState::Archived => {}
            WorkflowState::FinalReview
            | WorkflowState::UdfValidation
            | WorkflowState::UdfGeneration => awaiting_review.push(row.clone()),
            WorkflowState::DeliveryPending | WorkflowState::Signed => {
                awaiting_delivery.push(row.clone())
            }
            _ => active.push(row.clone()),
        }

        // Yaklaşan süreler
        if let Some(due) = c.assignment.due_date {
            let delta = due.inner() - now.inner();
            let days = delta.whole_days();
            if days <= deadline_horizon_days {
                let label = if delta.whole_seconds() < 0 {
                    format!("GECİKTİ ({})", humanize_duration(-delta))
                } else {
                    humanize_duration(delta)
                };
                upcoming.push((c.title.clone(), label));
            }
        }
    }

    // Yaklaşan süreleri en yakın önce sırala.
    upcoming.sort_by(|a, b| a.1.cmp(&b.1));

    Ok(DashboardData {
        active_cases: active,
        pending_exceptions,
        awaiting_report_review: awaiting_review,
        awaiting_delivery,
        archived_count,
        upcoming_deadlines: upcoming,
        total_cases: cases.len() as u32,
        generated_at: now,
    })
}

fn make_row(c: &Case, now: Timestamp) -> DashboardCaseRow {
    let due_date = c.assignment.due_date;
    let (due_in, overdue) = match due_date {
        Some(d) => {
            let delta = d.inner() - now.inner();
            if delta.whole_seconds() < 0 {
                (Some(format!("GECİKTİ: {}", humanize_duration(-delta))), true)
            } else {
                (Some(humanize_duration(delta)), false)
            }
        }
        None => (None, false),
    };

    DashboardCaseRow {
        case_id: c.id.to_string(),
        title: c.title.clone(),
        file_number: c.assignment.file_number.clone(),
        state: c.workflow.state.label().to_string(),
        due_date,
        due_in,
        overdue,
        pending_exceptions: c
            .preflight_warnings()
            .iter()
            .filter(|w| w.contains("yanıtlanmadı") || w.contains("kapsam"))
            .count(),
        updated_at: c.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::case::CourtOrAuthority;
    use birkat_core::ids::CourtId;

    fn court() -> CourtOrAuthority {
        CourtOrAuthority {
            id: CourtId::new(),
            name: "Mahkeme".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    fn store_with_cases() -> CaseStore {
        let mut s = CaseStore::open_in_memory().unwrap();
        let c1 = Case::new("Devam eden", court());
        let mut c2 = Case::new("Teslim bekleyen", court());
        c2.workflow.state = WorkflowState::DeliveryPending;
        let mut c3 = Case::new("Arşivlenmiş", court());
        c3.archived = true;
        c3.workflow.state = WorkflowState::Archived;
        s.save_case(&c1).unwrap();
        s.save_case(&c2).unwrap();
        s.save_case(&c3).unwrap();
        s
    }

    #[test]
    fn dashboard_counts_cases_by_stage() {
        let s = store_with_cases();
        let d = build_dashboard(&s, 30).unwrap();
        assert_eq!(d.total_cases, 3);
        assert_eq!(d.active_cases.len(), 1);
        assert_eq!(d.awaiting_delivery.len(), 1);
        assert_eq!(d.archived_count, 1);
    }

    #[test]
    fn summary_is_turkish() {
        let s = store_with_cases();
        let d = build_dashboard(&s, 30).unwrap();
        assert!(d.summary().contains("dosya"));
        assert!(d.summary().contains("teslim bekliyor"));
    }

    #[test]
    fn overdue_case_is_flagged() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut c = Case::new("Gecikmiş", court());
        c.assignment.due_date = Some(Timestamp::from(
            Timestamp::now().inner() - time::Duration::days(3),
        ));
        s.save_case(&c).unwrap();
        let d = build_dashboard(&s, 30).unwrap();
        assert!(d.has_overdue());
        assert!(d.active_cases[0].overdue);
        assert!(d.active_cases[0].due_in.as_ref().unwrap().contains("GECİKTİ"));
    }

    #[test]
    fn upcoming_deadlines_within_horizon() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut c = Case::new("Yakında", court());
        c.assignment.due_date = Some(Timestamp::from(
            Timestamp::now().inner() + time::Duration::days(2),
        ));
        s.save_case(&c).unwrap();
        let d = build_dashboard(&s, 30).unwrap();
        assert!(d.has_upcoming_deadlines());
        assert!(d.upcoming_deadlines[0].1.contains("gün"));
    }

    #[test]
    fn deadline_beyond_horizon_is_excluded() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut c = Case::new("Uzak", court());
        c.assignment.due_date = Some(Timestamp::from(
            Timestamp::now().inner() + time::Duration::days(90),
        ));
        s.save_case(&c).unwrap();
        let d = build_dashboard(&s, 30).unwrap();
        assert!(!d.has_upcoming_deadlines());
    }

    #[test]
    fn unanswered_questions_appear_as_exceptions() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut c = Case::new("Soru", court());
        c.assignment.questions.push(birkat_core::case::AssignmentQuestion {
            id: birkat_core::ids::QuestionId::new(),
            ordinal: 1,
            original_text: "Soru?".into(),
            related_evidence: vec![],
            related_segments: vec![],
            related_findings: vec![],
            requirement: None,
            expert_note: None,
            draft_answer: None,
            final_answer: None,
            answer_status: birkat_core::content_status::ContentStatus::MachineOutput,
            out_of_scope: false,
        });
        s.save_case(&c).unwrap();
        let d = build_dashboard(&s, 30).unwrap();
        assert!(!d.pending_exceptions.is_empty());
    }

    #[test]
    fn unacknowledged_scope_warning_appears() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut c = Case::new("Kapsam", court());
        c.assignment.scope_warnings = birkat_core::scope::scan_for_scope_warnings(
            "Kaydın montaj olup olmadığı belirlensin.",
        );
        s.save_case(&c).unwrap();
        let d = build_dashboard(&s, 30).unwrap();
        assert!(d
            .pending_exceptions
            .iter()
            .any(|(_, m)| m.contains("Kapsam uyarısı")));
    }

    #[test]
    fn report_review_stage_is_detected() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut c = Case::new("Kontrol", court());
        c.workflow.state = WorkflowState::FinalReview;
        s.save_case(&c).unwrap();
        let d = build_dashboard(&s, 30).unwrap();
        assert_eq!(d.awaiting_report_review.len(), 1);
    }

    #[test]
    fn empty_store_produces_empty_dashboard() {
        let s = CaseStore::open_in_memory().unwrap();
        let d = build_dashboard(&s, 30).unwrap();
        assert_eq!(d.total_cases, 0);
        assert!(d.active_cases.is_empty());
        assert!(!d.has_overdue());
    }
}
