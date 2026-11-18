//! Rapor son kontrolü (14.3, 27).
//!
//! Uygulama, mevzuatın zorunlu kıldığı alanları rapor son kontrolünde
//! denetler. Eksik alanlar, yanıtlanmamış sorular ve dayanaksız yanıtlar
//! açık uyarı üretir.

use birkat_core::case::Case;
use birkat_core::content_status::ContentStatus;
use birkat_core::report::{ReportDocument, SectionId};
use serde::{Deserialize, Serialize};

/// Son kontrol bulgusu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// İlgili bölüm (varsa).
    pub section: Option<SectionId>,
    /// Önem düzeyi.
    pub severity: ReviewSeverity,
    /// Açıklama.
    pub message: String,
}

/// Bulgu önem düzeyi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewSeverity {
    /// Bilgi.
    Info,
    /// Uyarı; teslim edilebilir ama gözden geçirilmeli.
    Warning,
    /// Hata; teslim edilemez.
    Error,
}

/// Son kontrol sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalReview {
    /// Teslim edilebilir mi?
    pub ready: bool,
    /// Bulgular.
    pub issues: Vec<ReviewIssue>,
    /// Hata sayısı.
    pub error_count: usize,
    /// Uyarı sayısı.
    pub warning_count: usize,
}

impl FinalReview {
    /// Türkçe özet.
    pub fn summary(&self) -> String {
        if self.ready {
            format!(
                "Son kontrol tamamlandı. {} uyarı var; teslim engeli yok.",
                self.warning_count
            )
        } else {
            format!(
                "Son kontrol başarısız: {} hata, {} uyarı.",
                self.error_count, self.warning_count
            )
        }
    }

    /// Hataları listeler.
    pub fn errors(&self) -> Vec<&ReviewIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ReviewSeverity::Error)
            .collect()
    }

    /// Uyarıları listeler.
    pub fn warnings(&self) -> Vec<&ReviewIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ReviewSeverity::Warning)
            .collect()
    }
}

/// Zorunlu rapor alanlarını denetler (14.1).
pub fn required_fields() -> Vec<(&'static str, SectionId)> {
    vec![
        ("Görevlendirmeyi yapan merci", SectionId::AssigningAuthority),
        ("Dosya numarası", SectionId::FileNumber),
        ("Yargılamanın tarafları", SectionId::Parties),
        (
            "Görevlendirme tarihi ve süresi",
            SectionId::AssignmentDateAndDuration,
        ),
        ("İncelemenin konusu", SectionId::Subject),
        (
            "İncelenmesi istenen maddi unsurlar",
            SectionId::MaterialsExamined,
        ),
        ("İnceleme yöntemi", SectionId::Method),
        (
            "Kullanılan transkripsiyon modeli ve parametreleri",
            SectionId::TranscriptionModel,
        ),
        ("Bilimsel ve teknik dayanaklar", SectionId::AnswerReferences),
        ("Gerekçeli sonuç", SectionId::Conclusion),
        ("Rapor tarihi", SectionId::ReportDate),
        ("Bilirkişi bilgileri", SectionId::ExpertInfo),
        ("İmza alanı", SectionId::Signature),
    ]
}

/// Rapor ve vaka üzerinde son kontrol yapar.
pub fn final_review(report: &ReportDocument, case: Option<&Case>) -> FinalReview {
    let mut issues = Vec::new();

    // 1. Zorunlu bölümler mevcut mu?
    for id in report.missing_mandatory_sections() {
        issues.push(ReviewIssue {
            section: Some(id),
            severity: ReviewSeverity::Error,
            message: format!("Zorunlu bölüm eksik: {}", id.default_title()),
        });
    }

    // 2. Zorunlu bölümler gizlenmiş mi?
    for id in report.hidden_mandatory_sections() {
        issues.push(ReviewIssue {
            section: Some(id),
            severity: ReviewSeverity::Error,
            message: format!(
                "Zorunlu bölüm gizlenmiş: {}. Zorunlu alanlar kaldırılamaz.",
                id.default_title()
            ),
        });
    }

    // 3. Zorunlu alanların içeriği var mı?
    for (label, id) in required_fields() {
        let filled = report
            .section(id)
            .map(|s| !s.blocks.is_empty())
            .unwrap_or(false);
        if !filled && report.section(id).is_some() {
            issues.push(ReviewIssue {
                section: Some(id),
                severity: ReviewSeverity::Warning,
                message: format!("Zorunlu alan boş: {label}."),
            });
        }
    }

    // 4. Rapor durumu
    if report.status != ContentStatus::ExpertApproved {
        issues.push(ReviewIssue {
            section: None,
            severity: ReviewSeverity::Warning,
            message: "Rapor bilirkişi tarafından onaylanmadı.".to_string(),
        });
    }

    // 5. İmza alanı bilirkişi bilgisi içeriyor mu?
    if report.experts.is_empty() {
        issues.push(ReviewIssue {
            section: Some(SectionId::Signature),
            severity: ReviewSeverity::Error,
            message: "Raporda bilirkişi imza bilgisi yok.".to_string(),
        });
    }

    // 6. Vaka düzeyi denetimler
    if let Some(c) = case {
        for w in c.preflight_warnings() {
            let severity = if w.contains("yanıtlanmadı") {
                ReviewSeverity::Error
            } else {
                ReviewSeverity::Warning
            };
            issues.push(ReviewIssue {
                section: Some(SectionId::Answers),
                severity,
                message: w,
            });
        }

        // Heyet görüş farklılıkları korunmuş mu?
        if !c.panel_members.is_empty() && report.dissenting_opinions.is_empty() {
            issues.push(ReviewIssue {
                section: Some(SectionId::Conclusion),
                severity: ReviewSeverity::Warning,
                message: "Heyet çalışmasında görüş farklılığı kaydı yok. \
                          Farklı görüşler ayrı gösterilmeli veya ayrı rapor hazırlanmalıdır."
                    .to_string(),
            });
        }
    }

    // 7. Ekler numaralandırılmış mı?
    for a in &report.attachments {
        if a.ordinal == 0 {
            issues.push(ReviewIssue {
                section: Some(SectionId::AttachmentList),
                severity: ReviewSeverity::Warning,
                message: format!("Ek numarası verilmemiş: {}", a.title),
            });
        }
    }

    let error_count = issues
        .iter()
        .filter(|i| i.severity == ReviewSeverity::Error)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.severity == ReviewSeverity::Warning)
        .count();

    FinalReview {
        ready: error_count == 0,
        issues,
        error_count,
        warning_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::case::{AssignmentQuestion, CourtOrAuthority};
    use birkat_core::ids::{CaseId, QuestionId};
    use birkat_core::report::{ExpertSignatureBlock, ReportKind};

    fn court() -> CourtOrAuthority {
        CourtOrAuthority {
            id: birkat_core::ids::CourtId::new(),
            name: "Ankara 5. Asliye Ceza Mahkemesi".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    fn signed_off_report() -> ReportDocument {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.experts.push(ExpertSignatureBlock {
            profile: birkat_core::ids::ExpertProfileId::new(),
            full_name: "Ayşe Yılmaz".into(),
            title: Some("Dr.".into()),
            registry_number: Some("12345".into()),
            panel_role: None,
            signature_blank: true,
        });
        r
    }

    #[test]
    fn complete_report_is_ready() {
        let r = signed_off_report();
        let rev = final_review(&r, None);
        assert!(rev.ready, "{:?}", rev.errors());
    }

    #[test]
    fn missing_expert_is_an_error() {
        let r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        let rev = final_review(&r, None);
        assert!(!rev.ready);
        assert!(rev
            .errors()
            .iter()
            .any(|e| e.message.contains("bilirkişi imza bilgisi")));
    }

    #[test]
    fn hidden_mandatory_section_is_an_error() {
        let mut r = signed_off_report();
        r.section_mut(SectionId::Conclusion).unwrap().hidden = true;
        let rev = final_review(&r, None);
        assert!(!rev.ready);
        assert!(rev.errors().iter().any(|e| e.message.contains("gizlenmiş")));
    }

    #[test]
    fn unanswered_question_is_an_error() {
        let r = signed_off_report();
        let mut c = Case::new("Test", court());
        c.assignment.questions.push(AssignmentQuestion {
            id: QuestionId::new(),
            ordinal: 1,
            original_text: "Kayıt çözülsün.".into(),
            related_evidence: vec![],
            related_segments: vec![],
            related_findings: vec![],
            requirement: None,
            expert_note: None,
            draft_answer: None,
            final_answer: None,
            answer_status: ContentStatus::MachineOutput,
            out_of_scope: false,
        });
        let rev = final_review(&r, Some(&c));
        assert!(!rev.ready);
        assert!(rev
            .errors()
            .iter()
            .any(|e| e.message.contains("1 numaralı soru")));
    }

    #[test]
    fn unapproved_report_is_a_warning_not_error() {
        let r = signed_off_report();
        let rev = final_review(&r, None);
        assert!(rev.ready);
        assert!(rev
            .warnings()
            .iter()
            .any(|w| w.message.contains("onaylanmadı")));
    }

    #[test]
    fn required_fields_list_covers_mandatory_sections() {
        let fields = required_fields();
        assert!(fields.iter().any(|(l, _)| l.contains("Gerekçeli sonuç")));
        assert!(fields.iter().any(|(l, _)| l.contains("Dosya numarası")));
    }

    #[test]
    fn summary_text_is_turkish() {
        let r = signed_off_report();
        let rev = final_review(&r, None);
        assert!(rev.summary().contains("Son kontrol"));
    }

    #[test]
    fn panel_without_dissenting_opinion_warns() {
        let r = signed_off_report();
        let mut c = Case::new("Heyet", court());
        c.panel_members
            .push(birkat_core::ids::ExpertProfileId::new());
        let rev = final_review(&r, Some(&c));
        assert!(rev
            .warnings()
            .iter()
            .any(|w| w.message.contains("görüş farklılığı")));
    }
}
