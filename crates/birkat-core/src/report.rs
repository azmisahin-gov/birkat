//! Rapor modeli ve şablonları — 14. bölüm.
//!
//! Rapor üretimi tek bir kanonik veri modelinden yapılır. UDF, PDF ve DOCX
//! için ayrı rapor yazma sistemleri oluşturulmaz; tüm çıktılar aynı
//! onaylanmış [`ReportDocument`] verisinden üretilir.

use crate::content_status::ContentStatus;
use crate::ids::*;
use crate::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Raporun kanonik veri modeli.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDocument {
    /// Kimlik.
    pub id: ReportId,
    /// Vaka.
    pub case: CaseId,
    /// Rapor türü.
    pub kind: ReportKind,
    /// Sürüm numarası.
    pub version: u32,
    /// Bölümler (sıralı).
    pub sections: Vec<ReportSection>,
    /// Ekler.
    pub attachments: Vec<Attachment>,
    /// Rapor tarihi.
    pub report_date: Timestamp,
    /// Bilirkişi bilgileri.
    pub experts: Vec<ExpertSignatureBlock>,
    /// Heyet görüş farklılıkları.
    pub dissenting_opinions: Vec<DissentingOpinion>,
    /// İçerik durumu.
    pub status: ContentStatus,
    /// Önceki rapor (ek rapor için).
    pub supersedes: Option<ReportId>,
    /// İmza kaydı.
    pub signature: Option<SignatureRecord>,
    /// Teslim kaydı.
    pub delivery: Option<DeliveryRecord>,
    /// Üretim zamanı.
    pub created_at: Timestamp,
    /// Sayfa yapısı.
    pub page_setup: PageSetup,
    /// İçindekiler oluşturulsun mu?
    pub include_toc: bool,
}

/// Rapor türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportKind {
    /// Ana rapor.
    Main,
    /// Ön rapor.
    Preliminary,
    /// Ek rapor.
    Supplementary,
    /// Ayrı rapor (heyet üyesi).
    Separate,
    /// Açıklama.
    Clarification,
}

impl ReportKind {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            ReportKind::Main => "Bilirkişi raporu",
            ReportKind::Preliminary => "Ön rapor",
            ReportKind::Supplementary => "Ek rapor",
            ReportKind::Separate => "Ayrı rapor",
            ReportKind::Clarification => "Açıklama",
        }
    }
}

/// Rapor bölümü.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    /// Bölüm kimliği (kanonik ad).
    pub id: SectionId,
    /// Başlık (Türkçe, düzenlenebilir).
    pub title: String,
    /// İçerik blokları.
    pub blocks: Vec<Block>,
    /// Zorunlu bölüm mü? (Zorunlu alanlar kaldırılamaz.)
    pub mandatory: bool,
    /// Bölüm gizlensin mi?
    pub hidden: bool,
}

/// Kanonik bölüm kimlikleri (14.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SectionId {
    /// Kapak.
    Cover,
    /// Görevlendiren merci.
    AssigningAuthority,
    /// Dosya numarası.
    FileNumber,
    /// Taraf bilgileri.
    Parties,
    /// Bilirkişi bilgileri.
    ExpertInfo,
    /// Görevlendirme tarihi ve süresi.
    AssignmentDateAndDuration,
    /// İnceleme konusu.
    Subject,
    /// Bilirkişiye yöneltilen sorular.
    Questions,
    /// İncelenen materyaller.
    MaterialsExamined,
    /// Materyal teslim ve edinim bilgileri.
    MaterialHandoverAndAcquisition,
    /// Delil listesi.
    EvidenceList,
    /// Dosya hash ve bütünlük kayıtları.
    HashAndIntegrity,
    /// İnceleme yöntemi.
    Method,
    /// Kullanılan yazılımlar ve sürümleri.
    SoftwareVersions,
    /// Kullanılan transkripsiyon modeli ve parametreleri.
    TranscriptionModel,
    /// Transkripsiyon yazım yöntemi.
    TranscriptionConventions,
    /// Seslerin teknik envanteri.
    AudioInventory,
    /// Transkriptler.
    Transcripts,
    /// Zaman kodları.
    TimeCodes,
    /// Anlaşılamayan veya belirsiz bölümler.
    UncertainParts,
    /// Bilirkişi sorularının yanıtları.
    Answers,
    /// Yanıtların teknik dayanakları.
    AnswerReferences,
    /// Yöntem sınırlılıkları.
    Limitations,
    /// Gerekçeli sonuç.
    Conclusion,
    /// Rapor tarihi.
    ReportDate,
    /// İmza.
    Signature,
    /// Ekler listesi.
    AttachmentList,
}

impl SectionId {
    /// Bölümün varsayılan Türkçe başlığı.
    pub fn default_title(&self) -> &'static str {
        match self {
            SectionId::Cover => "Kapak",
            SectionId::AssigningAuthority => "Görevlendiren merci",
            SectionId::FileNumber => "Dosya numarası",
            SectionId::Parties => "Taraf bilgileri",
            SectionId::ExpertInfo => "Bilirkişi bilgileri",
            SectionId::AssignmentDateAndDuration => "Görevlendirme tarihi ve süresi",
            SectionId::Subject => "İnceleme konusu",
            SectionId::Questions => "Bilirkişiye yöneltilen sorular",
            SectionId::MaterialsExamined => "İncelenen materyaller",
            SectionId::MaterialHandoverAndAcquisition => "Materyal teslim ve edinim bilgileri",
            SectionId::EvidenceList => "Delil listesi",
            SectionId::HashAndIntegrity => "Dosya hash ve bütünlük kayıtları",
            SectionId::Method => "İnceleme yöntemi",
            SectionId::SoftwareVersions => "Kullanılan yazılımlar ve sürümleri",
            SectionId::TranscriptionModel => "Kullanılan transkripsiyon modeli ve parametreleri",
            SectionId::TranscriptionConventions => "Transkripsiyon yazım yöntemi",
            SectionId::AudioInventory => "Seslerin teknik envanteri",
            SectionId::Transcripts => "Transkriptler",
            SectionId::TimeCodes => "Zaman kodları",
            SectionId::UncertainParts => "Anlaşılamayan veya belirsiz bölümler",
            SectionId::Answers => "Bilirkişi sorularının yanıtları",
            SectionId::AnswerReferences => "Yanıtların teknik dayanakları",
            SectionId::Limitations => "Yöntem sınırlılıkları",
            SectionId::Conclusion => "Gerekçeli sonuç",
            SectionId::ReportDate => "Rapor tarihi",
            SectionId::Signature => "İmza",
            SectionId::AttachmentList => "Ekler listesi",
        }
    }

    /// Bu bölüm mevzuat gereği zorunlu mu? (14.1, 14.2)
    ///
    /// Zorunlu bölümler şablondan kaldırılamaz.
    pub fn is_mandatory(&self) -> bool {
        !matches!(
            self,
            SectionId::Cover
                | SectionId::TranscriptionConventions
                | SectionId::TimeCodes
                | SectionId::SoftwareVersions
        )
    }

    /// Raporun varsayılan bölüm sırası (14.2).
    pub fn standard_order() -> Vec<SectionId> {
        use SectionId::*;
        vec![
            Cover,
            AssigningAuthority,
            FileNumber,
            Parties,
            ExpertInfo,
            AssignmentDateAndDuration,
            Subject,
            Questions,
            MaterialsExamined,
            MaterialHandoverAndAcquisition,
            EvidenceList,
            HashAndIntegrity,
            Method,
            SoftwareVersions,
            TranscriptionModel,
            TranscriptionConventions,
            AudioInventory,
            Transcripts,
            TimeCodes,
            UncertainParts,
            Answers,
            AnswerReferences,
            Limitations,
            Conclusion,
            ReportDate,
            Signature,
            AttachmentList,
        ]
    }
}

/// Rapor içerik bloğu.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type")]
pub enum Block {
    /// Paragraf.
    Paragraph {
        /// Metin.
        text: String,
        /// Hizalama.
        align: Align,
    },
    /// Başlık (seviye 1-4).
    Heading {
        /// Metin.
        text: String,
        /// Seviye.
        level: u8,
    },
    /// Madde işaretli liste.
    BulletList {
        /// Maddeler.
        items: Vec<String>,
    },
    /// Numaralı liste.
    NumberedList {
        /// Maddeler.
        items: Vec<String>,
        /// Numara türü.
        number_type: NumberType,
    },
    /// Tablo.
    Table {
        /// Başlık satırı.
        header: Vec<String>,
        /// Satırlar.
        rows: Vec<Vec<String>>,
        /// Sütun genişlikleri (pt); boşsa otomatik.
        column_widths: Vec<f64>,
    },
    /// Görsel.
    Image {
        /// Base64 kodlu veri (JPEG/PNG).
        data_base64: String,
        /// Genişlik (pt).
        width: f64,
        /// Yükseklik (pt).
        height: f64,
        /// Alt yazı.
        caption: Option<String>,
    },
    /// Sayfa sonu.
    PageBreak,
    /// Zaman kodu referansı (vurgulu).
    TimeReference {
        /// Etiket (ör. "kayit1.wav").
        label: String,
        /// Başlangıç.
        start: f64,
        /// Bitiş.
        end: f64,
        /// Alıntı.
        quote: Option<String>,
    },
    /// Transkript bölümü (zaman kodu ve metin).
    TranscriptSegment {
        /// Zaman kodu metni.
        time_code: String,
        /// Konuşmacı etiketi.
        speaker: Option<String>,
        /// Metin.
        text: String,
    },
    /// Uyarı kutusu (kapsam, sınırlılık, dikkat).
    Notice {
        /// Başlık.
        title: String,
        /// Metin.
        text: String,
    },
}

/// Paragraf hizalaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Align {
    /// Sol.
    Left,
    /// Orta.
    Center,
    /// Sağ.
    Right,
    /// İki yana yasla.
    Justify,
}

impl Align {
    /// UDF `Alignment` öznitelik değeri.
    pub fn udf_value(&self) -> &'static str {
        match self {
            Align::Left => "0",
            Align::Center => "1",
            Align::Right => "2",
            Align::Justify => "3",
        }
    }
}

/// Numara türü (UDF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NumberType {
    /// 1. 2. 3.
    NumberDot,
    /// 1) 2) 3)
    NumberParenthesis,
    /// a. b. c.
    CharSmallDot,
    /// a) b) c)
    CharSmallParenthesis,
    /// A. B. C.
    CharBigDot,
    /// A) B) C)
    CharBigParenthesis,
    /// i. ii. iii.
    RomanSmallDot,
    /// i) ii) iii)
    RomanSmallParenthesis,
    /// I. II. III.
    RomanBigDot,
    /// I) II) III)
    RomanBigParenthesis,
}

impl NumberType {
    /// UDF `NumberType` öznitelik değeri.
    pub fn udf_value(&self) -> &'static str {
        match self {
            NumberType::NumberDot => "NUMBER_TYPE_NUMBER_DOT",
            NumberType::NumberParenthesis => "NUMBER_TYPE_NUMBER_PARENTHESIS",
            NumberType::CharSmallDot => "NUMBER_TYPE_CHAR_SMALL_DOT",
            NumberType::CharSmallParenthesis => "NUMBER_TYPE_CHAR_SMALL_PARENTHESIS",
            NumberType::CharBigDot => "NUMBER_TYPE_CHAR_BIG_DOT",
            NumberType::CharBigParenthesis => "NUMBER_TYPE_CHAR_BIG_PARENTHESIS",
            NumberType::RomanSmallDot => "NUMBER_TYPE_ROMAN_SMALL_DOT",
            NumberType::RomanSmallParenthesis => "NUMBER_TYPE_ROMAN_SMALL_PARENTHESIS",
            NumberType::RomanBigDot => "NUMBER_TYPE_ROMAN_BIG_DOT",
            NumberType::RomanBigParenthesis => "NUMBER_TYPE_ROMAN_BIG_PARENTHESIS",
        }
    }
}

/// Rapor eki.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Kimlik.
    pub id: AttachmentId,
    /// Ek numarası (rapordaki sıra).
    pub ordinal: u32,
    /// Başlık.
    pub title: String,
    /// Açıklama.
    pub description: Option<String>,
    /// Dosya kimliği (varsa).
    pub file_id: Option<FileId>,
    /// Ek türü.
    pub kind: AttachmentKind,
    /// Hash.
    pub sha256: Option<String>,
}

/// Ek türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttachmentKind {
    /// Transkript tablosu.
    TranscriptTable,
    /// Ses envanteri tablosu.
    AudioInventoryTable,
    /// Hash tablosu.
    HashTable,
    /// Zaman kodu listesi.
    TimeCodeList,
    /// Görsel.
    Image,
    /// Tutanak.
    CustodyForm,
    /// Diğer belge.
    Document,
}

/// Rapor imza bloğundaki bilirkişi bilgisi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertSignatureBlock {
    /// Bilirkişi profili.
    pub profile: ExpertProfileId,
    /// Ad soyad.
    pub full_name: String,
    /// Unvan.
    pub title: Option<String>,
    /// Sicil numarası.
    pub registry_number: Option<String>,
    /// Heyetteki rolü.
    pub panel_role: Option<String>,
    /// İmza alanı boş mu?
    pub signature_blank: bool,
}

/// Heyet görüş farklılığı (14.1, 17).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DissentingOpinion {
    /// Görüş sahibi.
    pub author: ExpertProfileId,
    /// Görüş metni.
    pub text: String,
    /// Karşı çıktığı bölüm.
    pub section: SectionId,
}

/// Sayfa yapısı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSetup {
    /// Kâğıt boyutu adı (UDF `mediaSizeName`).
    pub media_size_name: String,
    /// Sol kenar boşluğu (pt).
    pub left_margin: f64,
    /// Sağ kenar boşluğu (pt).
    pub right_margin: f64,
    /// Üst kenar boşluğu (pt).
    pub top_margin: f64,
    /// Alt kenar boşluğu (pt).
    pub bottom_margin: f64,
    /// Yön (0 = yatay, 1 = dikey).
    pub orientation: u8,
    /// Üstbilgi metni.
    pub header: Option<String>,
    /// Altbilgi sayfa numarası gösterilsin mi?
    pub footer_page_number: bool,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            media_size_name: "1".to_string(),
            left_margin: 42.52,
            right_margin: 28.35,
            top_margin: 14.17,
            bottom_margin: 14.17,
            orientation: 1,
            header: None,
            footer_page_number: true,
        }
    }
}

/// İmza kaydı (16.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRecord {
    /// Kimlik.
    pub id: SignatureId,
    /// İmzalanan dosya.
    pub file: String,
    /// İmza öncesi hash.
    pub pre_sign_hash: String,
    /// İmza sonrası hash.
    pub post_sign_hash: Option<String>,
    /// İmza dosyası değişti mi?
    pub changed_after_signing: Option<bool>,
    /// İmza yöntemi (resmî e-imza).
    pub method: String,
    /// İmza zamanı.
    pub signed_at: Option<Timestamp>,
    /// Sertifika sahibi (portal/araç tarafından bildirildiyse).
    pub signer: Option<String>,
    /// Doğrulama notu.
    pub note: Option<String>,
}

/// Teslim kaydı (16.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRecord {
    /// Kimlik.
    pub id: DeliveryId,
    /// Teslim edilen dosyalar.
    pub files: Vec<String>,
    /// Teslim paketi hash manifesti.
    pub manifest_sha256: Option<String>,
    /// Teslim zamanı.
    pub delivered_at: Option<Timestamp>,
    /// Portal tarafından verilen teslim bilgisi.
    pub portal_reference: Option<String>,
    /// Teslim durumu.
    pub status: DeliveryStatus,
    /// Kullanıcı notu.
    pub note: Option<String>,
}

/// Teslim durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliveryStatus {
    /// Hazırlanmadı.
    NotPrepared,
    /// Paket hazır.
    PackageReady,
    /// Kullanıcı tarafından teslim edildi olarak işaretlendi.
    SubmittedByUser,
    /// Teslim bilgisi kaydedildi.
    Recorded,
}

impl ReportDocument {
    /// Yeni rapor oluşturur; zorunlu bölümler dahil standart sırayı kurar.
    pub fn new(case: CaseId, kind: ReportKind, version: u32) -> Self {
        let sections = SectionId::standard_order()
            .into_iter()
            .map(|id| ReportSection {
                id,
                title: id.default_title().to_string(),
                blocks: Vec::new(),
                mandatory: id.is_mandatory(),
                hidden: false,
            })
            .collect();

        Self {
            id: ReportId::new(),
            case,
            kind,
            version,
            sections,
            attachments: Vec::new(),
            report_date: Timestamp::now(),
            experts: Vec::new(),
            dissenting_opinions: Vec::new(),
            status: ContentStatus::MachineOutput,
            supersedes: None,
            signature: None,
            delivery: None,
            created_at: Timestamp::now(),
            page_setup: PageSetup::default(),
            include_toc: true,
        }
    }

    /// Bir bölümü kimliğine göre bulur.
    pub fn section(&self, id: SectionId) -> Option<&ReportSection> {
        self.sections.iter().find(|s| s.id == id)
    }

    /// Bir bölümü değiştirilebilir olarak bulur.
    pub fn section_mut(&mut self, id: SectionId) -> Option<&mut ReportSection> {
        self.sections.iter_mut().find(|s| s.id == id)
    }

    /// Bölüme blok ekler; bölüm yoksa oluşturur.
    pub fn push_block(&mut self, id: SectionId, block: Block) {
        if let Some(s) = self.section_mut(id) {
            s.blocks.push(block);
            return;
        }
        self.sections.push(ReportSection {
            id,
            title: id.default_title().to_string(),
            blocks: vec![block],
            mandatory: id.is_mandatory(),
            hidden: false,
        });
    }

    /// Zorunlu bölümlerden gizlenmiş olanları döndürür.
    ///
    /// Zorunlu hukuki/mesleki alanlar kaldırılamaz (14.2).
    pub fn hidden_mandatory_sections(&self) -> Vec<SectionId> {
        self.sections
            .iter()
            .filter(|s| s.hidden && s.mandatory)
            .map(|s| s.id)
            .collect()
    }

    /// Zorunlu bölümlerin eksik olup olmadığını kontrol eder (14.3).
    pub fn missing_mandatory_sections(&self) -> Vec<SectionId> {
        let present: Vec<SectionId> = self.sections.iter().map(|s| s.id).collect();
        SectionId::standard_order()
            .into_iter()
            .filter(|id| id.is_mandatory() && !present.contains(id))
            .collect()
    }

    /// Raporun içerik durumunu ilerletir (A.2).
    pub fn transition_status(
        &mut self,
        t: crate::content_status::StatusTransition,
    ) -> crate::error::Result<()> {
        self.status = crate::content_status::apply_transition(self.status, t)?;
        Ok(())
    }

    /// İmza sonrası rapor değiştirilemez (kural 20).
    pub fn is_locked(&self) -> bool {
        self.signature
            .as_ref()
            .and_then(|s| s.post_sign_hash.as_ref())
            .is_some()
    }

    /// Raporu değiştirmeden önce kilit kontrolü yapar.
    pub fn ensure_editable(&self) -> crate::error::Result<()> {
        if self.is_locked() {
            Err(crate::error::CoreError::Validation(
                "İmzalanmış rapor değiştirilemez. Yeni sürüm oluşturun.".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_report_has_all_standard_sections() {
        let r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        assert_eq!(r.sections.len(), SectionId::standard_order().len());
        assert!(r.missing_mandatory_sections().is_empty());
    }

    #[test]
    fn mandatory_sections_cannot_be_hidden() {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.section_mut(SectionId::Conclusion).unwrap().hidden = true;
        assert!(r
            .hidden_mandatory_sections()
            .contains(&SectionId::Conclusion));
    }

    #[test]
    fn missing_sections_are_detected() {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.sections.retain(|s| s.id != SectionId::Answers);
        assert!(r.missing_mandatory_sections().contains(&SectionId::Answers));
    }

    #[test]
    fn push_block_creates_section_if_absent() {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        r.sections.clear();
        r.push_block(
            SectionId::Conclusion,
            Block::Paragraph {
                text: "Sonuç".into(),
                align: Align::Justify,
            },
        );
        assert_eq!(r.section(SectionId::Conclusion).unwrap().blocks.len(), 1);
    }

    #[test]
    fn report_status_follows_content_rules() {
        use crate::content_status::StatusTransition::*;
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        assert_eq!(r.status, ContentStatus::MachineOutput);
        assert!(r.transition_status(ExpertApproved).is_err());
        r.transition_status(QualityControlPassed).unwrap();
        r.transition_status(ExpertApproved).unwrap();
        assert_eq!(r.status, ContentStatus::ExpertApproved);
    }

    #[test]
    fn signed_report_is_locked() {
        let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
        assert!(r.ensure_editable().is_ok());
        r.signature = Some(SignatureRecord {
            id: SignatureId::new(),
            file: "BILIRKISI_RAPORU.udf".into(),
            pre_sign_hash: "a".into(),
            post_sign_hash: Some("b".into()),
            changed_after_signing: Some(false),
            method: "resmî e-imza".into(),
            signed_at: Some(Timestamp::now()),
            signer: None,
            note: None,
        });
        assert!(r.is_locked());
        assert!(r.ensure_editable().is_err());
    }

    #[test]
    fn udf_attribute_mappings_are_correct() {
        assert_eq!(Align::Justify.udf_value(), "3");
        assert_eq!(NumberType::NumberDot.udf_value(), "NUMBER_TYPE_NUMBER_DOT");
        assert_eq!(
            NumberType::RomanSmallParenthesis.udf_value(),
            "NUMBER_TYPE_ROMAN_SMALL_PARENTHESIS"
        );
    }
}
