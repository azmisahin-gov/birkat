//! Rapor şablonları (14.2).
//!
//! Rapor şablonu düzenlenebilir olmalı; ancak zorunlu hukuki/mesleki alanlar
//! kaldırılamaz. Bu modül, kanonik bölüm listesini ve şablon uyarlamasını
//! yönetir.

use birkat_core::report::{ReportDocument, ReportKind, SectionId};
use serde::{Deserialize, Serialize};

/// Rapor şablonu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    /// Şablon adı.
    pub name: String,
    /// Açıklama.
    pub description: String,
    /// Bölüm başlığı uyarlamaları: (bölüm, yeni başlık).
    pub title_overrides: Vec<(SectionId, String)>,
    /// Gizlenecek bölümler (yalnızca zorunlu olmayanlar).
    pub hidden_sections: Vec<SectionId>,
    /// Sayfa yapısı uyarlaması.
    pub page_setup: Option<birkat_core::report::PageSetup>,
    /// Üstbilgi metni.
    pub header: Option<String>,
    /// Varsayılan yazı tipi.
    pub default_font: String,
    /// Varsayılan punto.
    pub default_font_size: f64,
}

impl Default for ReportTemplate {
    fn default() -> Self {
        Self::standard()
    }
}

impl ReportTemplate {
    /// Standart bilirkişi raporu şablonu.
    pub fn standard() -> Self {
        Self {
            name: "Standart bilirkişi raporu".to_string(),
            description: "Bilirkişilik Yönetmeliği'nin rapor içeriğine ilişkin \
                          hükümlerini karşılayan standart şablon."
                .to_string(),
            title_overrides: Vec::new(),
            hidden_sections: vec![
                SectionId::Cover,
                SectionId::SoftwareVersions,
                SectionId::TranscriptionConventions,
            ],
            page_setup: None,
            header: None,
            default_font: "Times New Roman".to_string(),
            default_font_size: 11.0,
        }
    }

    /// Ön rapor şablonu.
    pub fn preliminary() -> Self {
        let mut t = Self::standard();
        t.name = "Ön rapor".to_string();
        t.description = "Kısa sürede sunulan ön rapor şablonu.".to_string();
        t.hidden_sections.push(SectionId::Transcripts);
        t.hidden_sections.push(SectionId::AnswerReferences);
        t
    }

    /// Ek rapor şablonu.
    pub fn supplementary() -> Self {
        let mut t = Self::standard();
        t.name = "Ek rapor".to_string();
        t.description = "Önceki raporla ilişkili ek rapor şablonu.".to_string();
        t
    }

    /// Şablonu bir rapor belgesine uygular.
    ///
    /// Zorunlu bölümler gizlenemez; gizleme denemeleri sessizce yok sayılır ve
    /// [`Self::blocked_hides`] ile bildirilir.
    pub fn apply(&self, report: &mut ReportDocument) -> Vec<SectionId> {
        let mut blocked = Vec::new();

        for (id, title) in &self.title_overrides {
            if let Some(s) = report.section_mut(*id) {
                s.title = title.clone();
            }
        }

        for id in &self.hidden_sections {
            if id.is_mandatory() {
                blocked.push(*id);
                continue;
            }
            if let Some(s) = report.section_mut(*id) {
                s.hidden = true;
            }
        }

        if let Some(ps) = &self.page_setup {
            report.page_setup = ps.clone();
        }
        if let Some(h) = &self.header {
            report.page_setup.header = Some(h.clone());
        }
        blocked
    }

    /// Bu şablonun gizlemeye çalıştığı zorunlu bölümler.
    pub fn blocked_hides(&self) -> Vec<SectionId> {
        self.hidden_sections
            .iter()
            .copied()
            .filter(|id| id.is_mandatory())
            .collect()
    }

    /// Rapor türüne göre uygun şablonu seçer.
    pub fn for_kind(kind: ReportKind) -> Self {
        match kind {
            ReportKind::Preliminary => Self::preliminary(),
            ReportKind::Supplementary | ReportKind::Separate | ReportKind::Clarification => {
                Self::supplementary()
            }
            ReportKind::Main => Self::standard(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::ids::CaseId;

    fn report() -> ReportDocument {
        ReportDocument::new(CaseId::new(), ReportKind::Main, 1)
    }

    #[test]
    fn standard_template_hides_only_optional_sections() {
        let t = ReportTemplate::standard();
        assert!(t.blocked_hides().is_empty(), "zorunlu bölüm gizlenmemeli");
    }

    #[test]
    fn applying_template_hides_optional_sections() {
        let t = ReportTemplate::standard();
        let mut r = report();
        let blocked = t.apply(&mut r);
        assert!(blocked.is_empty());
        assert!(r.section(SectionId::Cover).unwrap().hidden);
        assert!(!r.section(SectionId::Conclusion).unwrap().hidden);
    }

    #[test]
    fn mandatory_sections_cannot_be_hidden_by_template() {
        let mut t = ReportTemplate::standard();
        t.hidden_sections.push(SectionId::Conclusion);
        assert_eq!(t.blocked_hides(), vec![SectionId::Conclusion]);
        let mut r = report();
        let blocked = t.apply(&mut r);
        assert_eq!(blocked, vec![SectionId::Conclusion]);
        assert!(!r.section(SectionId::Conclusion).unwrap().hidden);
    }

    #[test]
    fn title_override_is_applied() {
        let mut t = ReportTemplate::standard();
        t.title_overrides
            .push((SectionId::Conclusion, "SONUÇ VE KANAAT".to_string()));
        let mut r = report();
        t.apply(&mut r);
        assert_eq!(
            r.section(SectionId::Conclusion).unwrap().title,
            "SONUÇ VE KANAAT"
        );
    }

    #[test]
    fn template_selected_by_report_kind() {
        assert_eq!(
            ReportTemplate::for_kind(ReportKind::Preliminary).name,
            "Ön rapor"
        );
        assert_eq!(
            ReportTemplate::for_kind(ReportKind::Supplementary).name,
            "Ek rapor"
        );
        assert_eq!(
            ReportTemplate::for_kind(ReportKind::Main).name,
            "Standart bilirkişi raporu"
        );
    }

    #[test]
    fn template_serialises() {
        let t = ReportTemplate::standard();
        let json = serde_json::to_string(&t).unwrap();
        let back: ReportTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, t.name);
        assert_eq!(back.default_font, "Times New Roman");
    }
}
