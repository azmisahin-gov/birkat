//! Rapor otomatik doldurma (14.3).
//!
//! Uygulama görevlendirme bilgilerini aktarır, soruları orijinal sırayla
//! yerleştirir, materyal listesini oluşturur, ses teknik bilgilerini ekler,
//! bilirkişi tarafından onaylanmış transkriptleri aktarır, zaman kodu
//! referanslarını ekler, kullanılan yöntemi ve model bilgilerini açıklar,
//! dayanakları ilişkilendirir ve ekleri numaralandırır.
//!
//! Bilirkişi aynı bilgiyi farklı rapor alanlarında tekrar tekrar girmek
//! zorunda kalmaz.

use birkat_core::case::Case;
use birkat_core::content_status::ContentStatus;
use birkat_core::findings::Finding;
use birkat_core::profile::ExpertProfile;
use birkat_core::report::{
    Align, Block, ExpertSignatureBlock, ReportDocument, ReportKind, SectionId,
};
use birkat_core::time_util::Timestamp;

/// Rapor kurma girdisi.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ReportBuildInput {
    /// Ses envanteri satırları: (dosya adı, codec, süre, kanal, örnekleme, hash).
    pub audio_inventory: Vec<AudioInventoryRow>,
    /// Edinim ve hash kayıtları.
    pub acquisition_rows: Vec<Vec<String>>,
    /// Delil listesi satırları.
    pub evidence_rows: Vec<Vec<String>>,
    /// Transkript bölümleri: (ses adı, transkript sürümü, segmentler).
    pub transcripts: Vec<TranscriptRow>,
    /// Bulgular.
    pub findings: Vec<Finding>,
    /// Kullanılan model ve parametreler.
    pub model_info: Option<ModelInfo>,
    /// Yöntem açıklaması (bilirkişi tarafından yazılır veya düzenlenir).
    pub method_text: Option<String>,
    /// Yöntem sınırlılıkları.
    pub limitations: Vec<String>,
    /// Kullanılan yazılımlar ve sürümleri.
    pub software_versions: Vec<(String, String)>,
    /// Kapsam uyarıları (bilirkişi onaylıysa rapora girer).
    pub scope_notices: Vec<String>,
}

/// Ses envanteri satırı.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioInventoryRow {
    /// Dosya adı.
    pub file_name: String,
    /// Kapsayıcı.
    pub container: String,
    /// Codec.
    pub codec: String,
    /// Süre (saniye).
    pub duration_secs: Option<f64>,
    /// Kanal sayısı.
    pub channels: Option<u16>,
    /// Örnekleme frekansı.
    pub sample_rate: Option<u32>,
    /// SHA-256.
    pub sha256: Option<String>,
    /// Okunabilirlik etiketi.
    pub readability: String,
}

/// Transkript satırı.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptRow {
    /// Ses dosyası adı.
    pub audio_file: String,
    /// Transkript sürümü etiketi.
    pub version_label: String,
    /// İçerik durumu.
    pub status: ContentStatus,
    /// Segmentler: (zaman kodu, konuşmacı, metin).
    pub segments: Vec<(String, Option<String>, String)>,
    /// Belirsiz bölümler.
    pub uncertain: Vec<(String, String)>,
}

/// Kullanılan model bilgisi.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    /// Model adı.
    pub name: String,
    /// Sürüm.
    pub version: String,
    /// Model dosyası hash'i.
    pub file_hash: String,
    /// Backend.
    pub backend: String,
    /// Hassasiyet.
    pub precision: String,
    /// Parça uzunluğu.
    pub chunk_secs: f32,
    /// Örtüşme.
    pub overlap_secs: f32,
    /// Dil.
    pub language: Option<String>,
}

/// Vaka verisinden kanonik rapor belgesi kurar.
pub fn build_report(
    case: &Case,
    experts: &[ExpertProfile],
    input: &ReportBuildInput,
    kind: ReportKind,
    version: u32,
) -> ReportDocument {
    let mut report = ReportDocument::new(case.id.clone(), kind, version);

    // Görevlendiren merci
    report.push_block(
        SectionId::AssigningAuthority,
        Block::Paragraph {
            text: case.assignment.court.name.clone(),
            align: Align::Left,
        },
    );

    // Dosya numarası
    let file_no = case
        .assignment
        .file_number
        .clone()
        .unwrap_or_else(|| "(belirlenmedi)".to_string());
    report.push_block(
        SectionId::FileNumber,
        Block::Paragraph {
            text: file_no,
            align: Align::Left,
        },
    );

    // Taraflar
    if case.assignment.parties.is_empty() {
        report.push_block(
            SectionId::Parties,
            Block::Paragraph {
                text: "(taraf bilgisi belirlenmedi)".to_string(),
                align: Align::Left,
            },
        );
    } else {
        let rows: Vec<Vec<String>> = case
            .assignment
            .parties
            .iter()
            .map(|p| vec![p.role.clone(), p.name.clone()])
            .collect();
        report.push_block(
            SectionId::Parties,
            Block::Table {
                header: vec!["Rol".to_string(), "Ad Soyad / Unvan".to_string()],
                rows,
                column_widths: vec![150.0, 300.0],
            },
        );
    }

    // Bilirkişi bilgileri
    for e in experts {
        report.experts.push(ExpertSignatureBlock {
            profile: e.id.clone(),
            full_name: e.full_name.clone(),
            title: e.title.clone(),
            registry_number: e.registry_number.clone(),
            panel_role: None,
            signature_blank: true,
        });
    }
    if !experts.is_empty() {
        let rows: Vec<Vec<String>> = experts
            .iter()
            .map(|e| {
                vec![
                    e.full_name.clone(),
                    e.title.clone().unwrap_or_default(),
                    e.registry_number.clone().unwrap_or_default(),
                    e.specialty.clone(),
                ]
            })
            .collect();
        report.push_block(
            SectionId::ExpertInfo,
            Block::Table {
                header: vec![
                    "Ad Soyad".to_string(),
                    "Unvan".to_string(),
                    "Sicil No".to_string(),
                    "Uzmanlık".to_string(),
                ],
                rows,
                column_widths: vec![150.0, 100.0, 90.0, 100.0],
            },
        );
    }

    // Görevlendirme tarihi ve süresi
    let assignment_date = case
        .assignment
        .assignment_date
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "(belirlenmedi)".to_string());
    let due = case
        .assignment
        .due_date
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "(belirlenmedi)".to_string());
    let granted = case
        .assignment
        .granted_days
        .map(|d| format!("{d} gün"))
        .unwrap_or_else(|| "(belirlenmedi)".to_string());
    report.push_block(
        SectionId::AssignmentDateAndDuration,
        Block::Table {
            header: vec!["Alan".to_string(), "Değer".to_string()],
            rows: vec![
                vec!["Görevlendirme tarihi".to_string(), assignment_date],
                vec!["Verilen süre".to_string(), granted],
                vec!["Son teslim tarihi".to_string(), due],
            ],
            column_widths: vec![200.0, 250.0],
        },
    );

    // İnceleme konusu
    report.push_block(
        SectionId::Subject,
        Block::Paragraph {
            text: case
                .assignment
                .subject
                .clone()
                .unwrap_or_else(|| "(belirlenmedi)".to_string()),
            align: Align::Justify,
        },
    );

    // Sorular (orijinal sırayla)
    let mut ordered = case.assignment.questions.clone();
    ordered.sort_by_key(|q| q.ordinal);
    if ordered.is_empty() {
        report.push_block(
            SectionId::Questions,
            Block::Paragraph {
                text: "(görevlendirmede soru belirtilmedi)".to_string(),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::Questions,
            Block::NumberedList {
                items: ordered.iter().map(|q| q.original_text.clone()).collect(),
                number_type: birkat_core::report::NumberType::NumberDot,
            },
        );
    }

    // İncelenen materyaller
    if case.assignment.declared_materials.is_empty() {
        report.push_block(
            SectionId::MaterialsExamined,
            Block::Paragraph {
                text: "(materyal listesi belirlenmedi)".to_string(),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::MaterialsExamined,
            Block::BulletList {
                items: case.assignment.declared_materials.clone(),
            },
        );
    }

    // Materyal teslim ve edinim bilgileri
    if input.acquisition_rows.is_empty() {
        report.push_block(
            SectionId::MaterialHandoverAndAcquisition,
            Block::Paragraph {
                text: "Edinim kaydı bulunmuyor.".to_string(),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::MaterialHandoverAndAcquisition,
            Block::Table {
                header: vec![
                    "İşlem".to_string(),
                    "Kaynak".to_string(),
                    "Hedef".to_string(),
                    "Sonuç".to_string(),
                ],
                rows: input.acquisition_rows.clone(),
                column_widths: vec![120.0, 160.0, 160.0, 100.0],
            },
        );
    }

    // Delil listesi
    if input.evidence_rows.is_empty() {
        report.push_block(
            SectionId::EvidenceList,
            Block::Paragraph {
                text: "Delil kaydı bulunmuyor.".to_string(),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::EvidenceList,
            Block::Table {
                header: vec![
                    "Delil No".to_string(),
                    "Açıklama".to_string(),
                    "Kaynak".to_string(),
                    "Adet".to_string(),
                ],
                rows: input.evidence_rows.clone(),
                column_widths: vec![70.0, 220.0, 150.0, 60.0],
            },
        );
    }

    // Hash ve bütünlük kayıtları
    let mut hash_rows: Vec<Vec<String>> = Vec::new();
    for row in &input.audio_inventory {
        if let Some(h) = &row.sha256 {
            hash_rows.push(vec![
                row.file_name.clone(),
                "SHA-256".to_string(),
                h.clone(),
            ]);
        }
    }
    if hash_rows.is_empty() {
        report.push_block(
            SectionId::HashAndIntegrity,
            Block::Paragraph {
                text: "Hash kaydı bulunmuyor.".to_string(),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::HashAndIntegrity,
            Block::Table {
                header: vec![
                    "Dosya".to_string(),
                    "Algoritma".to_string(),
                    "Hash".to_string(),
                ],
                rows: hash_rows,
                column_widths: vec![140.0, 80.0, 260.0],
            },
        );
    }

    // İnceleme yöntemi
    report.push_block(
        SectionId::Method,
        Block::Paragraph {
            text: input
                .method_text
                .clone()
                .unwrap_or_else(default_method_text),
            align: Align::Justify,
        },
    );

    // Kullanılan yazılımlar
    if input.software_versions.is_empty() {
        report.push_block(
            SectionId::SoftwareVersions,
            Block::Paragraph {
                text: format!("BİRKÂT {}", birkat_core::APP_VERSION),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::SoftwareVersions,
            Block::Table {
                header: vec!["Yazılım".to_string(), "Sürüm".to_string()],
                rows: input
                    .software_versions
                    .iter()
                    .map(|(n, v)| vec![n.clone(), v.clone()])
                    .collect(),
                column_widths: vec![250.0, 150.0],
            },
        );
    }

    // Transkripsiyon modeli ve parametreleri
    if let Some(m) = &input.model_info {
        report.push_block(
            SectionId::TranscriptionModel,
            Block::Table {
                header: vec!["Alan".to_string(), "Değer".to_string()],
                rows: vec![
                    vec!["Model".to_string(), m.name.clone()],
                    vec!["Model sürümü".to_string(), m.version.clone()],
                    vec!["Model dosyası hash'i".to_string(), m.file_hash.clone()],
                    vec!["Çalıştırma cihazı".to_string(), m.backend.clone()],
                    vec!["Hassasiyet".to_string(), m.precision.clone()],
                    vec![
                        "Dil".to_string(),
                        m.language.clone().unwrap_or_else(|| "otomatik".to_string()),
                    ],
                    vec![
                        "Parça uzunluğu / örtüşme".to_string(),
                        format!("{:.1} sn / {:.1} sn", m.chunk_secs, m.overlap_secs),
                    ],
                ],
                column_widths: vec![200.0, 280.0],
            },
        );
    } else {
        report.push_block(
            SectionId::TranscriptionModel,
            Block::Paragraph {
                text: "Transkripsiyon yapılmadı.".to_string(),
                align: Align::Left,
            },
        );
    }

    // Yazım yöntemi
    report.push_block(
        SectionId::TranscriptionConventions,
        Block::Notice {
            title: "Transkripsiyon yazım yöntemi".to_string(),
            text: crate::conventions_note(),
        },
    );

    // Ses envanteri
    if input.audio_inventory.is_empty() {
        report.push_block(
            SectionId::AudioInventory,
            Block::Paragraph {
                text: "Ses dosyası bulunmuyor.".to_string(),
                align: Align::Left,
            },
        );
    } else {
        let rows: Vec<Vec<String>> = input
            .audio_inventory
            .iter()
            .map(|r| {
                vec![
                    r.file_name.clone(),
                    r.container.clone(),
                    r.codec.clone(),
                    r.duration_secs
                        .map(|d| format!("{d:.3}"))
                        .unwrap_or_else(|| "-".to_string()),
                    r.channels
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    r.sample_rate
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    r.readability.clone(),
                ]
            })
            .collect();
        report.push_block(
            SectionId::AudioInventory,
            Block::Table {
                header: vec![
                    "Dosya".to_string(),
                    "Kapsayıcı".to_string(),
                    "Codec".to_string(),
                    "Süre (sn)".to_string(),
                    "Kanal".to_string(),
                    "Örnekleme".to_string(),
                    "Durum".to_string(),
                ],
                rows,
                column_widths: vec![120.0, 70.0, 60.0, 60.0, 50.0, 70.0, 90.0],
            },
        );
    }

    // Transkriptler
    if input.transcripts.is_empty() {
        report.push_block(
            SectionId::Transcripts,
            Block::Paragraph {
                text: "Transkript bulunmuyor.".to_string(),
                align: Align::Left,
            },
        );
    } else {
        for t in &input.transcripts {
            report.push_block(
                SectionId::Transcripts,
                Block::Heading {
                    text: format!("{} — {}", t.audio_file, t.version_label),
                    level: 2,
                },
            );
            if t.status != ContentStatus::ExpertApproved {
                report.push_block(
                    SectionId::Transcripts,
                    Block::Notice {
                        title: "Durum uyarısı".to_string(),
                        text: format!(
                            "Bu transkript \"{}\" durumundadır. Makine çıktısı veya \
                             incelenmemiş içerik, bilirkişi tarafından doğrulanmış \
                             içerik olarak sunulmaz.",
                            t.status.label()
                        ),
                    },
                );
            }
            for (tc, spk, text) in &t.segments {
                report.push_block(
                    SectionId::Transcripts,
                    Block::TranscriptSegment {
                        time_code: tc.clone(),
                        speaker: spk.clone(),
                        text: text.clone(),
                    },
                );
            }
        }
    }

    // Zaman kodları ve belirsiz bölümler
    let mut uncertain_rows: Vec<Vec<String>> = Vec::new();
    for t in &input.transcripts {
        for (tc, desc) in &t.uncertain {
            uncertain_rows.push(vec![t.audio_file.clone(), tc.clone(), desc.clone()]);
        }
    }
    if uncertain_rows.is_empty() {
        report.push_block(
            SectionId::UncertainParts,
            Block::Paragraph {
                text: "Belirsiz veya anlaşılamayan bölüm bildirilmedi.".to_string(),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::UncertainParts,
            Block::Table {
                header: vec![
                    "Dosya".to_string(),
                    "Zaman kodu".to_string(),
                    "Açıklama".to_string(),
                ],
                rows: uncertain_rows,
                column_widths: vec![140.0, 100.0, 240.0],
            },
        );
    }

    // Yanıtlar (orijinal sırayla)
    for q in &ordered {
        report.push_block(
            SectionId::Answers,
            Block::Paragraph {
                text: format!("{}. {}", q.ordinal, q.original_text),
                align: Align::Justify,
            },
        );
        let answer = q
            .final_answer
            .clone()
            .unwrap_or_else(|| "(YANITLANMADI — mesleki değerlendirme bekliyor)".to_string());
        report.push_block(
            SectionId::Answers,
            Block::Paragraph {
                text: format!("Yanıt: {answer}"),
                align: Align::Justify,
            },
        );
    }

    // Dayanaklar
    let mut ref_rows: Vec<Vec<String>> = Vec::new();
    for q in &ordered {
        for seg in &q.related_segments {
            ref_rows.push(vec![
                format!("Soru {}", q.ordinal),
                seg.transcript_id.to_string(),
                format!("bölüm {}", seg.segment_index),
            ]);
        }
    }
    if ref_rows.is_empty() {
        report.push_block(
            SectionId::AnswerReferences,
            Block::Paragraph {
                text: "Yanıtlara bağlanmış teknik dayanak bulunmuyor.".to_string(),
                align: Align::Left,
            },
        );
    } else {
        report.push_block(
            SectionId::AnswerReferences,
            Block::Table {
                header: vec![
                    "İlgili soru".to_string(),
                    "Transkript".to_string(),
                    "Dayanak".to_string(),
                ],
                rows: ref_rows,
                column_widths: vec![100.0, 200.0, 150.0],
            },
        );
    }

    // Bulgular
    for f in &input.findings {
        if f.status == ContentStatus::MachineOutput {
            continue;
        }
        report.push_block(
            SectionId::Conclusion,
            Block::Heading {
                text: f.title.clone(),
                level: 3,
            },
        );
        report.push_block(
            SectionId::Conclusion,
            Block::Paragraph {
                text: f.body.clone(),
                align: Align::Justify,
            },
        );
    }

    // Kapsam uyarıları (bilirkişi onaylıysa)
    for n in &input.scope_notices {
        report.push_block(
            SectionId::Limitations,
            Block::Notice {
                title: "Kapsam uyarısı".to_string(),
                text: n.clone(),
            },
        );
    }

    // Yöntem sınırlılıkları
    for l in &input.limitations {
        report.push_block(
            SectionId::Limitations,
            Block::BulletList {
                items: vec![l.clone()],
            },
        );
    }
    if input.limitations.is_empty() {
        report.push_block(
            SectionId::Limitations,
            Block::BulletList {
                items: default_limitations(),
            },
        );
    }

    // Gerekçeli sonuç
    report.push_block(
        SectionId::Conclusion,
        Block::Paragraph {
            text: conclusion_placeholder(),
            align: Align::Justify,
        },
    );

    // Rapor tarihi
    report.push_block(
        SectionId::ReportDate,
        Block::Paragraph {
            text: Timestamp::now().to_rfc3339(),
            align: Align::Left,
        },
    );

    // İmza
    if !experts.is_empty() {
        report.push_block(
            SectionId::Signature,
            Block::Paragraph {
                text: experts
                    .iter()
                    .map(|e| e.signature_block())
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                align: Align::Left,
            },
        );
    }

    // Ekler listesi
    let mut table = birkat_core::report::Attachment {
        id: birkat_core::ids::AttachmentId::new(),
        ordinal: 1,
        title: "Ses envanteri tablosu".to_string(),
        description: Some("Ses dosyalarının teknik envanteri.".to_string()),
        file_id: None,
        kind: birkat_core::report::AttachmentKind::AudioInventoryTable,
        sha256: None,
    };
    report.attachments.push(table.clone());
    table.ordinal = 2;
    table.title = "Hash ve bütünlük tablosu".to_string();
    table.kind = birkat_core::report::AttachmentKind::HashTable;
    report.attachments.push(table);
    if !input.transcripts.is_empty() {
        let mut t3 = birkat_core::report::Attachment {
            id: birkat_core::ids::AttachmentId::new(),
            ordinal: 3,
            title: "Transkript tablosu".to_string(),
            description: Some("Zaman kodlu transkript bölümleri.".to_string()),
            file_id: None,
            kind: birkat_core::report::AttachmentKind::TranscriptTable,
            sha256: None,
        };
        t3.sha256 = None;
        report.attachments.push(t3);
    }

    report
}

/// Varsayılan yöntem açıklaması.
pub fn default_method_text() -> String {
    "İnceleme, ses kayıtlarının yerel (çevrimdışı) bir konuşma tanıma modeliyle \
     metne dönüştürülmesi, çıktının otomatik kalite kontrolünden geçirilmesi ve \
     belirsiz bölümlerin bilirkişi tarafından ses üzerinden doğrulanması \
     yöntemiyle yürütülmüştür. Transkriptler birebir (verbatim) yaklaşımla \
     üretilmiş; metin dilbilgisi açısından kendiliğinden yeniden yazılmamıştır. \
     Görüşme çözümlemesi kapsamında ses kayıtları metne dönüştürülmüş, \
     anlaşılamayan bölümler işaretlenmiş ve her yanıt ilgili zaman kodlarıyla \
     ilişkilendirilmiştir."
        .to_string()
}

/// Varsayılan yöntem sınırlılıkları.
pub fn default_limitations() -> Vec<String> {
    vec![
        "Otomatik konuşma tanıma çıktısı hata içerebilir; transkriptler bilirkişi \
         tarafından ses üzerinden doğrulanmıştır ancak özel isim, sayı, tarih ve \
         olumsuzluk ifadeleri ayrıca kontrol edilmelidir."
            .to_string(),
        "Konuşmacı etiketleri geçici ayrımdır; gerçek kişi kimliği veya biyometrik \
         tanımlama değildir."
            .to_string(),
        "Model güven göstergeleri gerçek doğruluk yüzdesi değildir; yalnızca \
         inceleme önceliği belirlemek için kullanılmıştır."
            .to_string(),
        "Bu inceleme 56.12 (Ses Kayıtlarının Metin Hâline Dönüştürülmesi) \
         kapsamındadır; kaydın orijinalliği, montaj tespiti ve konuşmacı kimliği \
         tespiti bu raporun kapsamı dışındadır."
            .to_string(),
    ]
}

/// Gerekçeli sonuç yer tutucusu.
pub fn conclusion_placeholder() -> String {
    "(Gerekçeli sonuç ve mesleki kanaat bilirkişi tarafından yazılacaktır.)".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::case::CourtOrAuthority;

    fn court() -> CourtOrAuthority {
        CourtOrAuthority {
            id: birkat_core::ids::CourtId::new(),
            name: "Ankara 5. Asliye Ceza Mahkemesi".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    fn case_with_question() -> Case {
        let mut c = Case::new("2025/123", court());
        c.assignment.file_number = Some("2025/123 Esas".into());
        c.assignment.subject = Some("Ses kaydının metne dönüştürülmesi".into());
        c.assignment.declared_materials = vec!["1 adet USB bellek".into()];
        c.assignment
            .questions
            .push(birkat_core::case::AssignmentQuestion {
                id: birkat_core::ids::QuestionId::new(),
                ordinal: 1,
                original_text: "Kayıttaki konuşmalar çözülsün.".into(),
                related_evidence: vec![],
                related_segments: vec![],
                related_findings: vec![],
                requirement: None,
                expert_note: None,
                draft_answer: None,
                final_answer: Some("Çözümleme yapıldı.".into()),
                answer_status: ContentStatus::ExpertApproved,
                out_of_scope: false,
            });
        c
    }

    fn expert() -> ExpertProfile {
        let mut e = ExpertProfile::new("Ayşe Yılmaz");
        e.title = Some("Dr.".into());
        e.registry_number = Some("12345".into());
        e
    }

    fn input() -> ReportBuildInput {
        ReportBuildInput {
            audio_inventory: vec![AudioInventoryRow {
                file_name: "kayit1.wav".into(),
                container: "WAV".into(),
                codec: "PCM".into(),
                duration_secs: Some(12.5),
                channels: Some(1),
                sample_rate: Some(44100),
                sha256: Some("aa".repeat(32)),
                readability: "Okunabilir".into(),
            }],
            model_info: Some(ModelInfo {
                name: "whisper-large-v3-turbo".into(),
                version: "1.0".into(),
                file_hash: "bb".repeat(32),
                backend: "cpu".into(),
                precision: "f16".into(),
                chunk_secs: 30.0,
                overlap_secs: 2.0,
                language: Some("tr".into()),
            }),
            transcripts: vec![TranscriptRow {
                audio_file: "kayit1.wav".into(),
                version_label: "Bilirkişi tarafından onaylanan nihai transkript".into(),
                status: ContentStatus::ExpertApproved,
                segments: vec![(
                    "00:00.000".into(),
                    Some("Konuşmacı 1".into()),
                    "Merhaba, bu bir denemedir.".into(),
                )],
                uncertain: vec![],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn builds_report_with_all_sections() {
        let c = case_with_question();
        let r = build_report(&c, &[expert()], &input(), ReportKind::Main, 1);
        assert!(r.missing_mandatory_sections().is_empty());
        assert!(r.section(SectionId::Conclusion).is_some());
    }

    #[test]
    fn questions_appear_in_original_order() {
        let mut c = case_with_question();
        c.assignment
            .questions
            .push(birkat_core::case::AssignmentQuestion {
                id: birkat_core::ids::QuestionId::new(),
                ordinal: 2,
                original_text: "İkinci soru.".into(),
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
        let r = build_report(&c, &[expert()], &input(), ReportKind::Main, 1);
        let questions = r.section(SectionId::Questions).unwrap();
        match &questions.blocks[0] {
            Block::NumberedList { items, .. } => {
                assert_eq!(items[0], "Kayıttaki konuşmalar çözülsün.");
                assert_eq!(items[1], "İkinci soru.");
            }
            _ => panic!("numaralı liste bekleniyordu"),
        }
    }

    #[test]
    fn unanswered_question_is_marked_in_report() {
        let mut c = case_with_question();
        c.assignment.questions[0].final_answer = None;
        let r = build_report(&c, &[expert()], &input(), ReportKind::Main, 1);
        let text = section_text(&r, SectionId::Answers);
        assert!(text.contains("YANITLANMADI"));
    }

    #[test]
    fn unapproved_transcript_carries_warning() {
        let mut i = input();
        i.transcripts[0].status = ContentStatus::MachineOutput;
        let c = case_with_question();
        let r = build_report(&c, &[expert()], &i, ReportKind::Main, 1);
        let text = section_text(&r, SectionId::Transcripts);
        assert!(text.contains("doğrulanmış içerik olarak sunulmaz"));
    }

    #[test]
    fn expert_signature_block_is_included() {
        let c = case_with_question();
        let r = build_report(&c, &[expert()], &input(), ReportKind::Main, 1);
        assert_eq!(r.experts.len(), 1);
        let text = section_text(&r, SectionId::Signature);
        assert!(text.contains("Ayşe Yılmaz"));
        assert!(text.contains("12345"));
    }

    #[test]
    fn attachments_are_numbered() {
        let c = case_with_question();
        let r = build_report(&c, &[expert()], &input(), ReportKind::Main, 1);
        assert!(r.attachments.len() >= 2);
        assert_eq!(r.attachments[0].ordinal, 1);
        assert_eq!(r.attachments[1].ordinal, 2);
    }

    #[test]
    fn model_info_is_recorded() {
        let c = case_with_question();
        let r = build_report(&c, &[expert()], &input(), ReportKind::Main, 1);
        let text = section_text(&r, SectionId::TranscriptionModel);
        assert!(text.contains("whisper-large-v3-turbo"));
        assert!(text.contains("cpu"));
    }

    #[test]
    fn hash_table_lists_audio_hashes() {
        let c = case_with_question();
        let r = build_report(&c, &[expert()], &input(), ReportKind::Main, 1);
        let text = section_text(&r, SectionId::HashAndIntegrity);
        assert!(text.contains("SHA-256"));
    }

    #[test]
    fn defaults_mention_scope_limitation() {
        let l = default_limitations();
        assert!(l.iter().any(|s| s.contains("56.12")));
        assert!(l
            .iter()
            .any(|s| s.contains("biyometrik tanımlama değildir")));
    }

    fn section_text(r: &ReportDocument, id: SectionId) -> String {
        let mut out = String::new();
        if let Some(s) = r.section(id) {
            for b in &s.blocks {
                match b {
                    Block::Paragraph { text, .. } => out.push_str(text),
                    Block::Heading { text, .. } => out.push_str(text),
                    Block::Notice { text, title } => {
                        out.push_str(title);
                        out.push_str(text);
                    }
                    Block::BulletList { items } => {
                        for i in items {
                            out.push_str(i);
                        }
                    }
                    Block::NumberedList { items, .. } => {
                        for i in items {
                            out.push_str(i);
                        }
                    }
                    Block::Table { header, rows, .. } => {
                        out.push_str(&header.join(" "));
                        for r in rows {
                            out.push_str(&r.join(" "));
                        }
                    }
                    Block::TranscriptSegment { text, speaker, .. } => {
                        if let Some(s) = speaker {
                            out.push_str(s);
                        }
                        out.push_str(text);
                    }
                    _ => {}
                }
                out.push(' ');
            }
        }
        out
    }
}
