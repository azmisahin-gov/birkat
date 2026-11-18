//! Ek rapor ve yardımcı mesleki belgeler (14.4, 14.5).
//!
//! Uygulama şu belgeleri hazırlayabilmelidir: ön rapor, ek rapor, süre uzatım
//! talebi, eksik materyal talebi, ek kayıt/belge talebi, görev kapsamının
//! açıklığa kavuşturulması talebi, görevden çekilme/mazeret bildirimi, materyal
//! teslim tutanağı, materyal iade tutanağı, dizi pusulası, rapor ek listesi,
//! teslim kontrol listesi.
//!
//! Bu belgeler de doğrudan UDF olarak üretilebilmelidir.

use birkat_core::case::Case;
use birkat_core::content_status::ContentStatus;
use birkat_core::evidence::CustodyRecord;
use birkat_core::profile::ExpertProfile;
use birkat_core::report::{
    Align, Attachment, Block, ExpertSignatureBlock, ReportDocument, ReportKind, SectionId,
};
use birkat_core::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Yardımcı mesleki belge türü (14.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuxiliaryDocumentKind {
    /// Süre uzatım talebi.
    ExtensionRequest,
    /// Eksik materyal talebi.
    MissingMaterialRequest,
    /// Ek kayıt/belge talebi.
    AdditionalRecordsRequest,
    /// Görev kapsamının açıklığa kavuşturulması talebi.
    ScopeClarificationRequest,
    /// Görevden çekilme/mazeret bildirimi.
    WithdrawalNotice,
    /// Materyal teslim tutanağı.
    MaterialHandoverForm,
    /// Materyal iade tutanağı.
    MaterialReturnForm,
    /// Dizi pusulası.
    EvidenceSchedule,
    /// Rapor ek listesi.
    ReportAttachmentList,
    /// Teslim kontrol listesi.
    DeliveryChecklist,
}

impl AuxiliaryDocumentKind {
    /// Belgenin Türkçe başlığı.
    pub fn title(&self) -> &'static str {
        match self {
            AuxiliaryDocumentKind::ExtensionRequest => "Süre Uzatım Talebi",
            AuxiliaryDocumentKind::MissingMaterialRequest => "Eksik Materyal Talebi",
            AuxiliaryDocumentKind::AdditionalRecordsRequest => "Ek Kayıt/Belge Talebi",
            AuxiliaryDocumentKind::ScopeClarificationRequest => {
                "Görev Kapsamının Açıklığa Kavuşturulması Talebi"
            }
            AuxiliaryDocumentKind::WithdrawalNotice => "Görevden Çekilme/Mazeret Bildirimi",
            AuxiliaryDocumentKind::MaterialHandoverForm => "Materyal Teslim Tutanağı",
            AuxiliaryDocumentKind::MaterialReturnForm => "Materyal İade Tutanağı",
            AuxiliaryDocumentKind::EvidenceSchedule => "Dizi Pusulası",
            AuxiliaryDocumentKind::ReportAttachmentList => "Rapor Ek Listesi",
            AuxiliaryDocumentKind::DeliveryChecklist => "Teslim Kontrol Listesi",
        }
    }

    /// Belgenin UDF dosya adı önerisi.
    pub fn suggested_file_name(&self) -> &'static str {
        match self {
            AuxiliaryDocumentKind::ExtensionRequest => "SURE_UZATIM_TALEBI.udf",
            AuxiliaryDocumentKind::MissingMaterialRequest => "EKSIK_MATERYAL_TALEBI.udf",
            AuxiliaryDocumentKind::AdditionalRecordsRequest => "EK_KAYIT_TALEBI.udf",
            AuxiliaryDocumentKind::ScopeClarificationRequest => "KAPSAM_ACIKLAMA_TALEBI.udf",
            AuxiliaryDocumentKind::WithdrawalNotice => "GOREVDEN_CEKILME_BILDIRIMI.udf",
            AuxiliaryDocumentKind::MaterialHandoverForm => "MATERYAL_TESLIM_TUTANAGI.udf",
            AuxiliaryDocumentKind::MaterialReturnForm => "MATERYAL_IADE_TUTANAGI.udf",
            AuxiliaryDocumentKind::EvidenceSchedule => "DIZI_PUSULASI.udf",
            AuxiliaryDocumentKind::ReportAttachmentList => "RAPOR_EK_LISTESI.udf",
            AuxiliaryDocumentKind::DeliveryChecklist => "TESLIM_KONTROL_LISTESI.udf",
        }
    }
}

/// Ek rapor bilgisi (14.4).
#[derive(Debug, Clone, Default)]
pub struct SupplementaryInput {
    /// Önceki rapor tarihi.
    pub previous_report_date: Option<Timestamp>,
    /// Önceki rapor sürümü.
    pub previous_report_version: Option<u32>,
    /// Yeni soru veya açıklama talebi.
    pub new_requests: Vec<String>,
    /// İlgili bulgular.
    pub related_findings: Vec<String>,
    /// Yeni materyaller.
    pub new_materials: Vec<String>,
    /// Yeni yanıtlar.
    pub new_answers: Vec<(String, String)>,
    /// Yeni sonuç.
    pub new_conclusion: Option<String>,
}

/// Ek rapor belgesi kurar.
///
/// Önceki rapor değiştirilmez; yeni rapor sürümü oluşturulur.
pub fn build_supplementary_report(
    case: &Case,
    experts: &[ExpertProfile],
    input: &SupplementaryInput,
    version: u32,
) -> ReportDocument {
    let mut r = ReportDocument::new(case.id.clone(), ReportKind::Supplementary, version);
    r.supersedes = None;

    let mut rows: Vec<Vec<String>> = Vec::new();
    rows.push(vec![
        "Önceki rapor tarihi".to_string(),
        input
            .previous_report_date
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| "(bilinmiyor)".to_string()),
    ]);
    rows.push(vec![
        "Önceki rapor sürümü".to_string(),
        input
            .previous_report_version
            .map(|v| v.to_string())
            .unwrap_or_else(|| "(bilinmiyor)".to_string()),
    ]);
    r.push_block(
        SectionId::Subject,
        Block::Table {
            header: vec!["Alan".to_string(), "Değer".to_string()],
            rows,
            column_widths: vec![180.0, 280.0],
        },
    );

    r.push_block(
        SectionId::Questions,
        Block::Notice {
            title: "Bu rapor önceki raporu değiştirmez".to_string(),
            text: "Bu ek rapor, önceki raporla ilişkili olarak düzenlenmiştir. \
                   Önceki rapor değiştirilmemiş; yeni rapor sürümü \
                   oluşturulmuştur."
                .to_string(),
        },
    );

    if !input.new_requests.is_empty() {
        r.push_block(
            SectionId::Questions,
            Block::Heading {
                text: "Yeni soru veya açıklama talepleri".to_string(),
                level: 2,
            },
        );
        r.push_block(
            SectionId::Questions,
            Block::NumberedList {
                items: input.new_requests.clone(),
                number_type: birkat_core::report::NumberType::NumberDot,
            },
        );
    }

    if !input.new_materials.is_empty() {
        r.push_block(
            SectionId::MaterialsExamined,
            Block::Heading {
                text: "Yeni materyaller".to_string(),
                level: 2,
            },
        );
        r.push_block(
            SectionId::MaterialsExamined,
            Block::BulletList {
                items: input.new_materials.clone(),
            },
        );
    }

    if !input.related_findings.is_empty() {
        r.push_block(
            SectionId::Conclusion,
            Block::Heading {
                text: "İlgili bulgular".to_string(),
                level: 2,
            },
        );
        for f in &input.related_findings {
            r.push_block(
                SectionId::Conclusion,
                Block::Paragraph {
                    text: f.clone(),
                    align: Align::Justify,
                },
            );
        }
    }

    for (q, a) in &input.new_answers {
        r.push_block(
            SectionId::Answers,
            Block::Paragraph {
                text: q.clone(),
                align: Align::Justify,
            },
        );
        r.push_block(
            SectionId::Answers,
            Block::Paragraph {
                text: format!("Yanıt: {a}"),
                align: Align::Justify,
            },
        );
    }

    r.push_block(
        SectionId::Conclusion,
        Block::Paragraph {
            text: input
                .new_conclusion
                .clone()
                .unwrap_or_else(crate::builder::conclusion_placeholder),
            align: Align::Justify,
        },
    );

    r.push_block(
        SectionId::ReportDate,
        Block::Paragraph {
            text: Timestamp::now().to_rfc3339(),
            align: Align::Left,
        },
    );

    for e in experts {
        r.experts.push(ExpertSignatureBlock {
            profile: e.id.clone(),
            full_name: e.full_name.clone(),
            title: e.title.clone(),
            registry_number: e.registry_number.clone(),
            panel_role: None,
            signature_blank: true,
        });
        r.push_block(
            SectionId::Signature,
            Block::Paragraph {
                text: e.signature_block(),
                align: Align::Left,
            },
        );
    }

    r
}

/// Yardımcı belge kurma girdisi.
#[derive(Debug, Clone, Default)]
pub struct AuxiliaryInput {
    /// Talep/bildirim gerekçesi (serbest metin).
    pub body: Option<String>,
    /// Talep edilen ek süre (gün).
    pub requested_days: Option<u32>,
    /// İstenen materyaller/belgeler.
    pub requested_items: Vec<String>,
    /// Teslim/iade kaydı.
    pub custody: Option<CustodyRecord>,
    /// Ek listesi satırları.
    pub attachment_rows: Vec<Vec<String>>,
    /// Teslim kontrol listesi maddeleri.
    pub checklist_items: Vec<String>,
    /// Kapsam uyarısı açıklaması.
    pub scope_note: Option<String>,
}

/// Yardımcı mesleki belge kurar.
pub fn build_auxiliary_document(
    kind: AuxiliaryDocumentKind,
    case: &Case,
    experts: &[ExpertProfile],
    input: &AuxiliaryInput,
) -> ReportDocument {
    let mut r = ReportDocument::new(case.id.clone(), ReportKind::Clarification, 1);

    // Kapak
    r.push_block(
        SectionId::Cover,
        Block::Heading {
            text: kind.title().to_string(),
            level: 1,
        },
    );

    r.push_block(
        SectionId::AssigningAuthority,
        Block::Paragraph {
            text: case.assignment.court.name.clone(),
            align: Align::Left,
        },
    );
    r.push_block(
        SectionId::FileNumber,
        Block::Paragraph {
            text: case
                .assignment
                .file_number
                .clone()
                .unwrap_or_else(|| "(belirlenmedi)".to_string()),
            align: Align::Left,
        },
    );

    match kind {
        AuxiliaryDocumentKind::ExtensionRequest => {
            r.push_block(
                SectionId::Subject,
                Block::Paragraph {
                    text: format!(
                        "{} sayılı dosyada tarafıma verilen bilirkişilik görevi için \
                         ek süre verilmesi talep edilmektedir.",
                        case.assignment
                            .file_number
                            .clone()
                            .unwrap_or_else(|| "<dosya no>".to_string())
                    ),
                    align: Align::Justify,
                },
            );
            if let Some(d) = input.requested_days {
                r.push_block(
                    SectionId::Subject,
                    Block::Paragraph {
                        text: format!("Talep edilen ek süre: {d} gün."),
                        align: Align::Left,
                    },
                );
            }
        }
        AuxiliaryDocumentKind::MissingMaterialRequest
        | AuxiliaryDocumentKind::AdditionalRecordsRequest => {
            r.push_block(
                SectionId::MaterialsExamined,
                Block::Paragraph {
                    text: "Görevin tamamlanabilmesi için aşağıdaki materyal/belgelerin \
                           gönderilmesi gerekmektedir."
                        .to_string(),
                    align: Align::Justify,
                },
            );
            r.push_block(
                SectionId::MaterialsExamined,
                Block::BulletList {
                    items: input.requested_items.clone(),
                },
            );
        }
        AuxiliaryDocumentKind::ScopeClarificationRequest => {
            r.push_block(
                SectionId::Limitations,
                Block::Notice {
                    title: "Görev kapsamı uyarısı".to_string(),
                    text: input
                        .scope_note
                        .clone()
                        .unwrap_or_else(|| {
                            "Görevlendirme metnindeki bazı hususlar 56.12 uzmanlık \
                             alanı dışında görünmektedir. Bilirkişiliğe esas olacak \
                             açıklamanın bildirilmesi istenmektedir."
                                .to_string()
                        }),
                },
            );
        }
        AuxiliaryDocumentKind::WithdrawalNotice => {
            r.push_block(
                SectionId::Limitations,
                Block::Paragraph {
                    text: input
                        .body
                        .clone()
                        .unwrap_or_else(|| {
                            "Aşağıda belirtilen mazeretim nedeniyle görevden \
                             çekilme talebim bulunmaktadır."
                                .to_string()
                        }),
                    align: Align::Justify,
                },
            );
        }
        AuxiliaryDocumentKind::MaterialHandoverForm
        | AuxiliaryDocumentKind::MaterialReturnForm => {
            if let Some(c) = &input.custody {
                r.push_block(
                    SectionId::MaterialHandoverAndAcquisition,
                    Block::Table {
                        header: vec!["Alan".to_string(), "Değer".to_string()],
                        rows: vec![
                            vec!["Teslim eden".to_string(), c.delivered_by.clone()],
                            vec!["Teslim alan".to_string(), c.received_by.clone()],
                            vec!["Tarih ve saat".to_string(), c.at.to_rfc3339()],
                            vec!["Materyal türü".to_string(), c.material_kind.clone()],
                            vec![
                                "Üretici/model".to_string(),
                                c.make_model.clone().unwrap_or_default(),
                            ],
                            vec!["Seri numarası".to_string(), c.serial.clone().unwrap_or_default()],
                            vec![
                                "Mühür numarası".to_string(),
                                c.seal_number.clone().unwrap_or_default(),
                            ],
                            vec![
                                "Ambalaj".to_string(),
                                c.packaging.clone().unwrap_or_default(),
                            ],
                            vec![
                                "Fiziksel durum".to_string(),
                                c.physical_condition.clone().unwrap_or_default(),
                            ],
                            vec!["Not".to_string(), c.note.clone().unwrap_or_default()],
                        ],
                        column_widths: vec![150.0, 300.0],
                    },
                );
            }
        }
        AuxiliaryDocumentKind::EvidenceSchedule
        | AuxiliaryDocumentKind::ReportAttachmentList => {
            if !input.attachment_rows.is_empty() {
                r.push_block(
                    SectionId::AttachmentList,
                    Block::Table {
                        header: vec![
                            "Sıra".to_string(),
                            "Açıklama".to_string(),
                            "Adet".to_string(),
                        ],
                        rows: input.attachment_rows.clone(),
                        column_widths: vec![50.0, 320.0, 60.0],
                    },
                );
            }
        }
        AuxiliaryDocumentKind::DeliveryChecklist => {
            let items = if input.checklist_items.is_empty() {
                default_delivery_checklist()
            } else {
                input.checklist_items.clone()
            };
            r.push_block(
                SectionId::AttachmentList,
                Block::BulletList { items },
            );
        }
    }

    r.push_block(
        SectionId::ReportDate,
        Block::Paragraph {
            text: Timestamp::now().to_rfc3339(),
            align: Align::Left,
        },
    );

    for e in experts {
        r.experts.push(ExpertSignatureBlock {
            profile: e.id.clone(),
            full_name: e.full_name.clone(),
            title: e.title.clone(),
            registry_number: e.registry_number.clone(),
            panel_role: None,
            signature_blank: true,
        });
        r.push_block(
            SectionId::Signature,
            Block::Paragraph {
                text: e.signature_block(),
                align: Align::Left,
            },
        );
    }

    r.status = ContentStatus::ReadyForReview;
    r
}

/// Varsayılan teslim kontrol listesi (16.2).
pub fn default_delivery_checklist() -> Vec<String> {
    vec![
        "Ana UDF raporu (BILIRKISI_RAPORU.udf) hazır ve doğrulandı.".to_string(),
        "Rapor bilirkişi tarafından onaylandı.".to_string(),
        "Rapor imza sürecinden geçti; imzalı dosya arşivlendi.".to_string(),
        "İmza sonrası dosya değişmedi (hash karşılaştırması yapıldı).".to_string(),
        "Ekler numaralandırıldı ve raporla ilişkilendirildi.".to_string(),
        "Teslim paketi hash manifesti oluşturuldu.".to_string(),
        "Dosya adları ve klasör yapısı teslim kurallarına uygun.".to_string(),
        "Vaka arşivi ve şifreli yedek güncel.".to_string(),
    ]
}

/// Yardımcı belgenin UDF dosya adını döndürür.
pub fn file_name_for(kind: AuxiliaryDocumentKind) -> &'static str {
    kind.suggested_file_name()
}

/// Ek listesi satırı üretir.
pub fn attachment_row(ordinal: u32, title: &str, count: u32) -> Vec<String> {
    vec![ordinal.to_string(), title.to_string(), count.to_string()]
}

/// Rapor eki oluşturur.
pub fn make_attachment(ordinal: u32, title: &str) -> Attachment {
    Attachment {
        id: birkat_core::ids::AttachmentId::new(),
        ordinal,
        title: title.to_string(),
        description: None,
        file_id: None,
        kind: birkat_core::report::AttachmentKind::Document,
        sha256: None,
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
            name: "Ankara 5. Asliye Ceza Mahkemesi".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    fn case() -> Case {
        let mut c = Case::new("2025/123", court());
        c.assignment.file_number = Some("2025/123 Esas".into());
        c
    }

    fn expert() -> ExpertProfile {
        let mut e = ExpertProfile::new("Ayşe Yılmaz");
        e.registry_number = Some("12345".into());
        e
    }

    #[test]
    fn all_auxiliary_kinds_have_titles_and_file_names() {
        for k in [
            AuxiliaryDocumentKind::ExtensionRequest,
            AuxiliaryDocumentKind::MissingMaterialRequest,
            AuxiliaryDocumentKind::AdditionalRecordsRequest,
            AuxiliaryDocumentKind::ScopeClarificationRequest,
            AuxiliaryDocumentKind::WithdrawalNotice,
            AuxiliaryDocumentKind::MaterialHandoverForm,
            AuxiliaryDocumentKind::MaterialReturnForm,
            AuxiliaryDocumentKind::EvidenceSchedule,
            AuxiliaryDocumentKind::ReportAttachmentList,
            AuxiliaryDocumentKind::DeliveryChecklist,
        ] {
            assert!(!k.title().is_empty());
            assert!(k.suggested_file_name().ends_with(".udf"));
        }
    }

    #[test]
    fn extension_request_mentions_file_number_and_days() {
        let input = AuxiliaryInput {
            requested_days: Some(15),
            ..Default::default()
        };
        let r = build_auxiliary_document(
            AuxiliaryDocumentKind::ExtensionRequest,
            &case(),
            &[expert()],
            &input,
        );
        let text = plain(&r);
        assert!(text.contains("2025/123 Esas"));
        assert!(text.contains("15 gün"));
    }

    #[test]
    fn missing_material_request_lists_items() {
        let input = AuxiliaryInput {
            requested_items: vec!["Orijinal ses kaydı".into(), "Teslim tutanağı".into()],
            ..Default::default()
        };
        let r = build_auxiliary_document(
            AuxiliaryDocumentKind::MissingMaterialRequest,
            &case(),
            &[expert()],
            &input,
        );
        let text = plain(&r);
        assert!(text.contains("Orijinal ses kaydı"));
        assert!(text.contains("Teslim tutanağı"));
    }

    #[test]
    fn handover_form_includes_custody_fields() {
        let mut c = CustodyRecord::new("Ahmet Kolluk", "Ayşe Yılmaz");
        c.material_kind = "USB bellek".into();
        c.serial = Some("SN123".into());
        c.seal_number = Some("M001".into());
        let input = AuxiliaryInput {
            custody: Some(c),
            ..Default::default()
        };
        let r = build_auxiliary_document(
            AuxiliaryDocumentKind::MaterialHandoverForm,
            &case(),
            &[expert()],
            &input,
        );
        let text = plain(&r);
        assert!(text.contains("Ahmet Kolluk"));
        assert!(text.contains("USB bellek"));
        assert!(text.contains("SN123"));
        assert!(text.contains("M001"));
    }

    #[test]
    fn scope_clarification_uses_note() {
        let input = AuxiliaryInput {
            scope_note: Some("Montaj tespiti 56.11 kapsamındadır.".into()),
            ..Default::default()
        };
        let r = build_auxiliary_document(
            AuxiliaryDocumentKind::ScopeClarificationRequest,
            &case(),
            &[expert()],
            &input,
        );
        assert!(plain(&r).contains("56.11"));
    }

    #[test]
    fn delivery_checklist_has_default_items() {
        let r = build_auxiliary_document(
            AuxiliaryDocumentKind::DeliveryChecklist,
            &case(),
            &[expert()],
            &AuxiliaryInput::default(),
        );
        let text = plain(&r);
        assert!(text.contains("BILIRKISI_RAPORU.udf"));
        assert!(text.contains("imza"));
    }

    #[test]
    fn auxiliary_document_converts_to_udf() {
        let r = build_auxiliary_document(
            AuxiliaryDocumentKind::ExtensionRequest,
            &case(),
            &[expert()],
            &AuxiliaryInput::default(),
        );
        let udf = crate::report_to_udf(&r, &crate::ReportToUdfOptions::default());
        let bytes = birkat_udf::write_udf_to_bytes(udf).unwrap();
        let v = birkat_udf::validate_udf(&bytes);
        assert!(v.ok, "hatalar: {:?}", v.errors());
    }

    #[test]
    fn supplementary_report_does_not_replace_previous() {
        let input = SupplementaryInput {
            previous_report_date: Some(Timestamp::now()),
            previous_report_version: Some(1),
            new_requests: vec!["Ek kayıt incelensin.".into()],
            new_conclusion: Some("Ek inceleme sonucu.".into()),
            ..Default::default()
        };
        let r = build_supplementary_report(&case(), &[expert()], &input, 2);
        assert_eq!(r.kind, ReportKind::Supplementary);
        assert_eq!(r.version, 2);
        let text = plain(&r);
        assert!(text.contains("önceki raporu değiştirmez") || text.contains("değiştirilmemiş"));
    }

    #[test]
    fn supplementary_report_includes_new_answers() {
        let input = SupplementaryInput {
            new_answers: vec![(
                "Yeni soru?".to_string(),
                "Yeni yanıt.".to_string(),
            )],
            ..Default::default()
        };
        let r = build_supplementary_report(&case(), &[expert()], &input, 2);
        let text = plain(&r);
        assert!(text.contains("Yeni soru?"));
        assert!(text.contains("Yeni yanıt."));
    }

    fn plain(r: &ReportDocument) -> String {
        let mut out = String::new();
        for s in &r.sections {
            out.push_str(&s.title);
            out.push(' ');
            for b in &s.blocks {
                match b {
                    Block::Paragraph { text, .. } => out.push_str(text),
                    Block::Heading { text, .. } => out.push_str(text),
                    Block::BulletList { items } => out.push_str(&items.join(" ")),
                    Block::NumberedList { items, .. } => out.push_str(&items.join(" ")),
                    Block::Notice { title, text } => {
                        out.push_str(title);
                        out.push_str(text);
                    }
                    Block::Table { header, rows, .. } => {
                        out.push_str(&header.join(" "));
                        for row in rows {
                            out.push_str(&row.join(" "));
                        }
                    }
                    _ => {}
                }
                out.push(' ');
            }
        }
        out
    }
}
