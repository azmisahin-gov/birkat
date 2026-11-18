//! Vaka çalışma alanı.
//!
//! Bir görevlendirme açıldığında o dosyanın bütün işlevleri aynı çalışma
//! alanında bulunur (2.1). Bu yapı, çekirdek alan modelini, kalıcı depoyu,
//! kasayı ve işlem geçmişini tek bir vaka etrafında birleştirir.

use crate::store::CaseStore;
use crate::vault::CaseVault;
use birkat_core::case::Case;
use birkat_core::error::{CoreError, Result};
use birkat_core::ids::CaseId;
use birkat_core::profile::ExpertProfile;
use birkat_core::time_util::Timestamp;
use std::path::PathBuf;

/// Vaka çalışma alanı.
pub struct CaseWorkspace {
    /// Vaka kaydı.
    pub case: Case,
    /// Kasa.
    pub vault: CaseVault,
    /// Vaka kök dizini.
    pub root: PathBuf,
    /// İşlem geçmişi (bellekte; kalıcı kopya depodadır).
    pub audit: birkat_audit::AuditChain,
}

impl CaseWorkspace {
    /// Yeni vaka için çalışma alanı oluşturur.
    pub fn create(
        data_root: impl AsRef<std::path::Path>,
        case: Case,
        store: &mut CaseStore,
    ) -> Result<Self> {
        let root = data_root.as_ref().join("kasalar").join(case.id.as_str());
        let mut vault = CaseVault::open(&root)?;
        vault.set_case(case.id.clone())?;

        let mut audit = birkat_audit::AuditChain::new();
        audit.append(
            birkat_audit::AuditAction::CaseCreated,
            "bilirkişi",
            format!("Vaka oluşturuldu: {}", case.title),
            Some(case.id.clone()),
            None,
            serde_json::json!({ "title": case.title }),
        );

        let mut case = case;
        case.updated_at = Timestamp::now();
        store.save_case(&case)?;
        persist_audit(store, &audit)?;

        Ok(Self {
            case,
            vault,
            root,
            audit,
        })
    }

    /// Var olan vakayı açar.
    pub fn open(
        data_root: impl AsRef<std::path::Path>,
        case_id: &CaseId,
        store: &mut CaseStore,
    ) -> Result<Self> {
        let case = store.load_case(case_id)?;
        let root = data_root.as_ref().join("kasalar").join(case.id.as_str());
        let vault = CaseVault::open(&root)?;
        let audit = store.load_audit_chain()?;
        Ok(Self {
            case,
            vault,
            root,
            audit,
        })
    }

    /// Kasayı parola ile açar.
    pub fn unlock_vault(&mut self, password: &str) -> Result<()> {
        self.vault.unlock(password)
    }

    /// Vaka değişikliğini kaydeder ve işlem geçmişine yazar.
    pub fn commit(
        &mut self,
        store: &mut CaseStore,
        action: birkat_audit::AuditAction,
        description: impl Into<String>,
        subject: Option<String>,
        details: serde_json::Value,
    ) -> Result<()> {
        self.case.updated_at = Timestamp::now();
        store.save_case(&self.case)?;
        self.audit.append(
            action,
            "bilirkişi",
            description,
            Some(self.case.id.clone()),
            subject,
            details,
        );
        // Son olayı kalıcılaştır.
        if let Some(last) = self.audit.events.last() {
            store.append_audit(last)?;
        }
        Ok(())
    }

    /// İş akışı durumunu ilerletir ve kaydeder.
    ///
    /// Hedef duruma giden tüm ara adımlar tek tek kaydedilir; böylece işlem
    /// geçmişi eksiksiz olur ve her adım denetlenebilir kalır.
    pub fn advance_workflow(
        &mut self,
        store: &mut CaseStore,
        to: birkat_core::workflow::WorkflowState,
    ) -> Result<()> {
        if self.case.workflow.state == to {
            return Ok(());
        }
        let path = birkat_core::workflow::path_to(self.case.workflow.state, to)
            .ok_or_else(|| {
                CoreError::InvalidTransition {
                    from: self.case.workflow.state.to_string(),
                    to: to.to_string(),
                }
            })?;
        for step in path {
            self.case.workflow.advance_to(step)?;
            self.commit(
                store,
                birkat_audit::AuditAction::Approved,
                format!("İş akışı durumu: {}", step.label()),
                None,
                serde_json::json!({ "state": format!("{step:?}") }),
            )?;
        }
        Ok(())
    }

    /// Sıradaki duruma ilerletir.
    pub fn advance_next(&mut self, store: &mut CaseStore) -> Result<birkat_core::workflow::WorkflowState> {
        let next = self.case.workflow.advance()?;
        self.commit(
            store,
            birkat_audit::AuditAction::Approved,
            format!("İş akışı ilerledi: {}", next.label()),
            None,
            serde_json::json!({ "state": format!("{next:?}") }),
        )?;
        Ok(next)
    }

    /// İşlem geçmişi bütünlüğünü doğrular.
    pub fn verify_audit(&self) -> birkat_audit::ChainVerification {
        self.audit.verify()
    }

    /// Vakayı arşivler.
    pub fn archive(&mut self, store: &mut CaseStore) -> Result<()> {
        self.case.archived = true;
        if self.case.workflow.state != birkat_core::workflow::WorkflowState::Archived {
            self.case.workflow.state = birkat_core::workflow::WorkflowState::Archived;
        }
        self.commit(
            store,
            birkat_audit::AuditAction::Archived,
            "Vaka arşivlendi",
            None,
            serde_json::json!({}),
        )
    }

    /// Vakayı kilitle (kasa oturumu kapanır).
    pub fn lock(&mut self) {
        self.vault.lock();
    }

    /// Vaka özeti.
    pub fn summary(&self) -> String {
        format!(
            "{} | {} | {} dosya | {} nesne",
            self.case.title,
            self.case.workflow.state.label(),
            self.case.assignment.file_number.as_deref().unwrap_or("-"),
            self.vault.object_count()
        )
    }
}

fn persist_audit(store: &mut CaseStore, chain: &birkat_audit::AuditChain) -> Result<()> {
    if let Some(last) = chain.events.last() {
        store.append_audit(last)?;
    }
    Ok(())
}

/// Vaka oluşturma yardımcısı: görevlendirme bilgilerini kurar.
pub fn new_case_from_assignment(
    title: impl Into<String>,
    court: birkat_core::case::CourtOrAuthority,
    file_number: Option<String>,
    subject: Option<String>,
) -> Case {
    let mut c = Case::new(title, court);
    c.assignment.file_number = file_number;
    c.assignment.subject = subject;
    c
}

/// Heyet üyesi ekler.
pub fn add_panel_member(case: &mut Case, member: birkat_core::ids::ExpertProfileId) {
    if !case.panel_members.contains(&member) {
        case.panel_members.push(member);
    }
}

/// Vakaya bilirkişi atar.
pub fn set_lead_expert(case: &mut Case, profile: &ExpertProfile) {
    case.lead_expert = Some(profile.id.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::case::CourtOrAuthority;
    use birkat_core::ids::CourtId;
    use birkat_core::workflow::WorkflowState;

    fn court() -> CourtOrAuthority {
        CourtOrAuthority {
            id: CourtId::new(),
            name: "Ankara 5. Asliye Ceza Mahkemesi".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    #[test]
    fn create_workspace_persists_case_and_audit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment(
            "2025/123",
            court(),
            Some("2025/123 Esas".into()),
            Some("Ses çözümlemesi".into()),
        );
        let ws = CaseWorkspace::create(dir.path(), case.clone(), &mut store).unwrap();
        assert_eq!(ws.case.id, case.id);
        assert!(ws.vault.root().exists());
        assert_eq!(store.case_count().unwrap(), 1);

        let chain = store.load_audit_chain().unwrap();
        assert_eq!(chain.events.len(), 1);
        assert!(chain.verify().valid);
    }

    #[test]
    fn open_existing_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment("A", court(), None, None);
        let id = case.id.clone();
        CaseWorkspace::create(dir.path(), case, &mut store).unwrap();

        let ws = CaseWorkspace::open(dir.path(), &id, &mut store).unwrap();
        assert_eq!(ws.case.id, id);
        assert_eq!(ws.case.title, "A");
    }

    #[test]
    fn advance_workflow_records_audit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();

        let next = ws.advance_next(&mut store).unwrap();
        assert_eq!(next, WorkflowState::AssignmentImported);
        assert!(ws.verify_audit().valid);

        // Kalıcı zincir de doğrulanmalı.
        let chain = store.load_audit_chain().unwrap();
        assert!(chain.verify().valid);
        assert!(chain.events.len() >= 2);
    }

    #[test]
    fn vault_stores_and_reads_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();

        let src = dir.path().join("kayit.wav");
        std::fs::write(&src, b"RIFF....WAVEfmt ").unwrap();
        ws.vault.store_file(&src, "ses/kayit.wav", "USB").unwrap();
        ws.commit(
            &mut store,
            birkat_audit::AuditAction::MaterialAdded,
            "Materyal eklendi",
            Some("ses/kayit.wav".into()),
            serde_json::json!({}),
        )
        .unwrap();

        let data = ws.vault.read_object("ses/kayit.wav").unwrap();
        assert_eq!(data, b"RIFF....WAVEfmt ");
        assert!(ws.verify_audit().valid);
    }

    #[test]
    fn archive_marks_case_and_audits() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        ws.archive(&mut store).unwrap();
        assert!(ws.case.archived);
        assert_eq!(ws.case.workflow.state, WorkflowState::Archived);

        let reloaded = store.load_case(&ws.case.id).unwrap();
        assert!(reloaded.archived);
    }

    #[test]
    fn commit_persists_updated_case() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        ws.case.assignment.file_number = Some("2025/1".into());
        ws.commit(
            &mut store,
            birkat_audit::AuditAction::ReportEdited,
            "Dosya numarası girildi",
            None,
            serde_json::json!({}),
        )
        .unwrap();
        let reloaded = store.load_case(&ws.case.id).unwrap();
        assert_eq!(reloaded.assignment.file_number.as_deref(), Some("2025/1"));
    }

    #[test]
    fn summary_mentions_state_and_file_number() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment("A", court(), Some("2025/9".into()), None);
        let ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        let s = ws.summary();
        assert!(s.contains("2025/9"));
        assert!(s.contains("Yeni"));
    }

    #[test]
    fn panel_member_added_once() {
        let mut c = Case::new("A", court());
        let id = birkat_core::ids::ExpertProfileId::new();
        add_panel_member(&mut c, id.clone());
        add_panel_member(&mut c, id.clone());
        assert_eq!(c.panel_members.len(), 1);
    }

    #[test]
    fn encrypted_workspace_vault_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        ws.unlock_vault("Guvenli-Kasa-2026").unwrap();

        let src = dir.path().join("gizli.wav");
        std::fs::write(&src, b"gizli icerik").unwrap();
        ws.vault.store_file(&src, "g.wav", "USB").unwrap();

        let raw = std::fs::read(ws.vault.root().join("data/g.wav")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("gizli icerik"));
        assert_eq!(ws.vault.read_object("g.wav").unwrap(), b"gizli icerik");
    }

    #[test]
    fn missing_case_reports_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        assert!(CaseWorkspace::open(dir.path(), &CaseId::new(), &mut store).is_err());
    }
}
