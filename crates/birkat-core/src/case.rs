//! Vaka, görevlendirme, taraf, merci ve süre nesneleri — 4. bölüm ve 21.1.

use crate::ids::*;
use crate::time_util::Timestamp;
use crate::workflow::WorkflowRecord;
use serde::{Deserialize, Serialize};

/// Mahkeme veya görevlendiren merci.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtOrAuthority {
    /// Kimlik.
    pub id: CourtId,
    /// Merci adı (ör. "Ankara 5. Asliye Ceza Mahkemesi").
    pub name: String,
    /// Birim türü (mahkeme, savcılık, kolluk, kurum).
    pub unit_kind: String,
    /// İletişim bilgisi (varsa).
    pub contact: Option<String>,
    /// Kaynak belge referansı (çıkarımın dayandığı yer).
    pub source_ref: Option<SourceRef>,
}

/// Bir çıkarımın kaynak belge referansı (4.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    /// Belge kimliği.
    pub document_id: FileId,
    /// Belge adı.
    pub document_name: String,
    /// Sayfa numarası (varsa).
    pub page: Option<u32>,
    /// Paragraf veya konum bilgisi (varsa).
    pub anchor: Option<String>,
}

/// Yargılamanın tarafı veya ilgili kişi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    /// Kimlik.
    pub id: PartyId,
    /// Ad ve soyad veya unvan.
    pub name: String,
    /// Rol (müşteki, şüpheli, sanık, mağdur, tanık, ilgili kişi, vekil).
    pub role: String,
    /// Kaynak referansı.
    pub source_ref: Option<SourceRef>,
}

/// Süre kaydı (4.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deadline {
    /// Kimlik.
    pub id: DeadlineId,
    /// Süre türü.
    pub kind: DeadlineKind,
    /// Tarih (RFC 3339 gün başlangıcı).
    pub due_at: Timestamp,
    /// Açıklama.
    pub note: Option<String>,
    /// Tamamlandı mı?
    pub completed: bool,
}

/// Süre türleri (4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeadlineKind {
    /// Görevlendirme tarihi.
    AssignmentDate,
    /// Teslim süresi.
    DeliveryWindow,
    /// Son teslim tarihi.
    FinalDueDate,
    /// Süre uzatım talebi.
    ExtensionRequest,
    /// Ek rapor tarihi.
    SupplementaryReportDate,
    /// Duruşma/açıklama tarihi.
    HearingDate,
    /// Eksik materyal bekleme.
    AwaitingMaterial,
    /// Teslim edildi.
    Submitted,
}

impl DeadlineKind {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            DeadlineKind::AssignmentDate => "Görevlendirme tarihi",
            DeadlineKind::DeliveryWindow => "Teslim süresi",
            DeadlineKind::FinalDueDate => "Son teslim tarihi",
            DeadlineKind::ExtensionRequest => "Süre uzatım talebi",
            DeadlineKind::SupplementaryReportDate => "Ek rapor tarihi",
            DeadlineKind::HearingDate => "Duruşma/açıklama tarihi",
            DeadlineKind::AwaitingMaterial => "Eksik materyal bekleme",
            DeadlineKind::Submitted => "Teslim edildi",
        }
    }
}

/// Görevlendirme belgesinden çıkarılan ve bilirkişi tarafından doğrulanan veri.
///
/// Otomatik çıkarılan alanlar [`crate::content_status::ContentStatus`] ile
/// damgalanır; alanlar kesin doğru kabul edilmez.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    /// Kimlik.
    pub id: AssignmentId,
    /// Görevlendiren merci.
    pub court: CourtOrAuthority,
    /// Dosya esas numarası.
    pub file_number: Option<String>,
    /// Dosya karar numarası.
    pub decision_number: Option<String>,
    /// Taraflar.
    pub parties: Vec<Party>,
    /// Görevlendirme tarihi.
    pub assignment_date: Option<Timestamp>,
    /// Verilen süre (gün).
    pub granted_days: Option<u32>,
    /// Son teslim tarihi.
    pub due_date: Option<Timestamp>,
    /// İnceleme konusu.
    pub subject: Option<String>,
    /// İncelenecek maddi unsurlar.
    pub material_subjects: Vec<String>,
    /// Bilirkişiye yöneltilen sorular.
    pub questions: Vec<AssignmentQuestion>,
    /// Materyal listesi (belgede yazılı).
    pub declared_materials: Vec<String>,
    /// Belge ekleri.
    pub attachments: Vec<String>,
    /// Özel talimatlar.
    pub special_instructions: Vec<String>,
    /// Kapsam uyarıları (1.2).
    pub scope_warnings: Vec<crate::scope::ScopeWarningRecord>,
    /// Kaynak belge referansları.
    pub source_refs: Vec<SourceRef>,
    /// Çıkarım durumu.
    pub extraction_status: crate::content_status::ContentStatus,
    /// Çözülemeyen veya çelişen alanlar (yalnızca bunlar kullanıcıya gösterilir).
    pub unresolved_fields: Vec<UnresolvedField>,
}

/// Çözülemeyen veya çelişen alan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedField {
    /// Alan adı.
    pub field: String,
    /// Açıklama.
    pub reason: String,
    /// Aday değerler.
    pub candidates: Vec<String>,
}

/// Görevlendirme sorusu (4.3, 13).
///
/// Soru metni aynen korunur; anlamı değişecek biçimde yeniden yazılmaz.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentQuestion {
    /// Kimlik.
    pub id: QuestionId,
    /// Orijinal sıra numarası (1 tabanlı).
    pub ordinal: u32,
    /// Orijinal soru metni (aynen).
    pub original_text: String,
    /// İlgili materyal kimlikleri.
    pub related_evidence: Vec<EvidenceId>,
    /// İlgili transkript bölümü kimlikleri.
    pub related_segments: Vec<SegmentRef>,
    /// Teknik dayanak kimlikleri.
    pub related_findings: Vec<FindingId>,
    /// İnceleme gereksinimi.
    pub requirement: Option<String>,
    /// Bilirkişi notu.
    pub expert_note: Option<String>,
    /// Yanıt taslağı (makine önerisi).
    pub draft_answer: Option<DraftAnswer>,
    /// Nihai yanıt (bilirkişi).
    pub final_answer: Option<String>,
    /// Yanıtın içerik durumu.
    pub answer_status: crate::content_status::ContentStatus,
    /// Kapsam dışı mı?
    pub out_of_scope: bool,
}

impl AssignmentQuestion {
    /// Soru yanıtlanmış mı?
    pub fn is_answered(&self) -> bool {
        self.final_answer
            .as_ref()
            .map(|a| !a.trim().is_empty())
            .unwrap_or(false)
    }
}

/// Bir transkript bölümüne referans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentRef {
    /// Transkript kimliği.
    pub transcript_id: TranscriptId,
    /// Bölüm sıra numarası.
    pub segment_index: usize,
}

/// Makine tarafından önerilen yanıt taslağı.
///
/// A.2 gereği bu taslak bilirkişi kanaati değildir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftAnswer {
    /// Taslak metin.
    pub text: String,
    /// Dayanak referansları.
    pub references: Vec<ReferenceId>,
    /// Üretim zamanı.
    pub generated_at: Timestamp,
    /// Üretici (model/yöntem).
    pub producer: String,
}

/// Vaka — tüm çalışmanın merkezi kaydı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    /// Kimlik.
    pub id: CaseId,
    /// Vaka başlığı (ör. "2025/123 Esas — ses kaydı çözümü").
    pub title: String,
    /// Görevlendirme.
    pub assignment: Assignment,
    /// Süreler.
    pub deadlines: Vec<Deadline>,
    /// İş akışı kaydı.
    pub workflow: WorkflowRecord,
    /// Vakayı yürüten bilirkişi.
    pub lead_expert: Option<ExpertProfileId>,
    /// Heyet üyeleri (17).
    pub panel_members: Vec<ExpertProfileId>,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
    /// Son güncelleme zamanı.
    pub updated_at: Timestamp,
    /// Veri modeli sürümü.
    pub schema_version: u32,
    /// Arşivlendi mi?
    pub archived: bool,
}

impl Case {
    /// Yeni vaka oluşturur.
    pub fn new(title: impl Into<String>, court: CourtOrAuthority) -> Self {
        let now = Timestamp::now();
        Self {
            id: CaseId::new(),
            title: title.into(),
            assignment: Assignment {
                id: AssignmentId::new(),
                court,
                file_number: None,
                decision_number: None,
                parties: Vec::new(),
                assignment_date: None,
                granted_days: None,
                due_date: None,
                subject: None,
                material_subjects: Vec::new(),
                questions: Vec::new(),
                declared_materials: Vec::new(),
                attachments: Vec::new(),
                special_instructions: Vec::new(),
                scope_warnings: Vec::new(),
                source_refs: Vec::new(),
                extraction_status: crate::content_status::ContentStatus::MachineOutput,
                unresolved_fields: Vec::new(),
            },
            deadlines: Vec::new(),
            workflow: WorkflowRecord::new(),
            lead_expert: None,
            panel_members: Vec::new(),
            created_at: now,
            updated_at: now,
            schema_version: crate::MODEL_VERSION,
            archived: false,
        }
    }

    /// Vaka son kontrolü: yanıtlanmamış sorular ve eksik alanlar (4.3, 14.3).
    pub fn preflight_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        for q in &self.assignment.questions {
            if !q.is_answered() {
                warnings.push(format!("{} numaralı soru yanıtlanmadı.", q.ordinal));
            } else if q.related_segments.is_empty() && q.related_findings.is_empty() {
                warnings.push(format!(
                    "{} numaralı sorunun yanıtı için teknik dayanak bağlanmadı.",
                    q.ordinal
                ));
            }
            if q.out_of_scope {
                warnings.push(format!(
                    "{} numaralı soru kapsam dışı olarak işaretlendi; açıklama istenmesi gerekebilir.",
                    q.ordinal
                ));
            }
        }

        if self.assignment.file_number.is_none() {
            warnings.push("Dosya esas numarası belirlenmedi.".to_string());
        }
        if self.assignment.due_date.is_none() {
            warnings.push("Son teslim tarihi belirlenmedi.".to_string());
        }
        if self.assignment.subject.is_none() {
            warnings.push("İnceleme konusu belirlenmedi.".to_string());
        }
        if self.assignment.extraction_status
            != crate::content_status::ContentStatus::ExpertApproved
        {
            warnings.push(
                "Görevlendirme bilgileri bilirkişi tarafından doğrulanmadı.".to_string(),
            );
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn court() -> CourtOrAuthority {
        CourtOrAuthority {
            id: CourtId::new(),
            name: "Ankara 5. Asliye Ceza Mahkemesi".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    fn question(ordinal: u32) -> AssignmentQuestion {
        AssignmentQuestion {
            id: QuestionId::new(),
            ordinal,
            original_text: "Kayıttaki konuşmalar çözülerek metne dönüştürülsün.".into(),
            related_evidence: vec![],
            related_segments: vec![],
            related_findings: vec![],
            requirement: None,
            expert_note: None,
            draft_answer: None,
            final_answer: None,
            answer_status: crate::content_status::ContentStatus::MachineOutput,
            out_of_scope: false,
        }
    }

    #[test]
    fn new_case_starts_in_new_state() {
        let c = Case::new("Test", court());
        assert_eq!(c.workflow.state, crate::workflow::WorkflowState::New);
        assert_eq!(c.schema_version, crate::MODEL_VERSION);
    }

    #[test]
    fn unanswered_question_produces_warning() {
        let mut c = Case::new("Test", court());
        c.assignment.questions.push(question(1));
        let w = c.preflight_warnings();
        assert!(w.iter().any(|x| x.contains("1 numaralı soru yanıtlanmadı")));
    }

    #[test]
    fn answered_without_evidence_produces_warning() {
        let mut c = Case::new("Test", court());
        let mut q = question(1);
        q.final_answer = Some("Çözümleme yapıldı.".into());
        c.assignment.questions.push(q);
        let w = c.preflight_warnings();
        assert!(w.iter().any(|x| x.contains("teknik dayanak bağlanmadı")));
    }

    #[test]
    fn missing_file_number_produces_warning() {
        let c = Case::new("Test", court());
        assert!(c
            .preflight_warnings()
            .iter()
            .any(|x| x.contains("Dosya esas numarası")));
    }
}
