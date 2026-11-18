//! Bulgular, dayanaklar ve yanıtlar — 13. bölüm.

use crate::ids::*;
use crate::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Teknik bulgu veya mesleki not (13).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Kimlik.
    pub id: FindingId,
    /// Başlık.
    pub title: String,
    /// Bulgu metni.
    pub body: String,
    /// Bulgu türü.
    pub kind: FindingKind,
    /// İçerik durumu (makine çıktısı / incelemeye hazır / onaylı).
    pub status: crate::content_status::ContentStatus,
    /// Dayanaklar.
    pub references: Vec<ReferenceId>,
    /// İlgili sorular.
    pub related_questions: Vec<QuestionId>,
    /// Yazan bilirkişi.
    pub author: Option<ExpertProfileId>,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
    /// Son güncelleme.
    pub updated_at: Timestamp,
}

/// Bulgu türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingKind {
    /// Teknik bulgu.
    Technical,
    /// Mesleki değerlendirme.
    ProfessionalAssessment,
    /// Yöntem sınırlılığı.
    MethodLimitation,
    /// Uyarı.
    Caution,
    /// İstatistik.
    Statistic,
}

impl FindingKind {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            FindingKind::Technical => "Teknik bulgu",
            FindingKind::ProfessionalAssessment => "Mesleki değerlendirme",
            FindingKind::MethodLimitation => "Yöntem sınırlılığı",
            FindingKind::Caution => "Uyarı",
            FindingKind::Statistic => "İstatistik",
        }
    }
}

/// Delil referansı — bir yanıtın veya bulgunun dayanağı (13).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceReference {
    /// Kimlik.
    pub id: ReferenceId,
    /// Dayanak türü.
    pub kind: ReferenceKind,
    /// İnsan okuyabilir etiket (raporda görünür).
    pub label: String,
    /// Kaynak dosya.
    pub file_id: Option<FileId>,
    /// Ses varlığı.
    pub audio: Option<AudioId>,
    /// Transkript.
    pub transcript: Option<TranscriptId>,
    /// Bölüm indeksi.
    pub segment_index: Option<usize>,
    /// Zaman kodu başlangıcı (saniye).
    pub start_secs: Option<f64>,
    /// Zaman kodu bitişi (saniye).
    pub end_secs: Option<f64>,
    /// Alıntı metni.
    pub quote: Option<String>,
    /// Dosya hash'i (dayanağın bütünlük bağı).
    pub sha256: Option<String>,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
}

/// Dayanak türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReferenceKind {
    /// Ses kaydı bölümü.
    AudioSegment,
    /// Transkript bölümü.
    TranscriptSegment,
    /// Teknik metadata.
    TechnicalMetadata,
    /// Delil envanteri kaydı.
    EvidenceInventory,
    /// Hash kaydı.
    HashRecord,
    /// Edinim işlemi.
    AcquisitionOperation,
    /// Dosya.
    File,
    /// Bilirkişi notu.
    ExpertNote,
}

impl EvidenceReference {
    /// Zaman aralığını `[başlangıç - bitiş]` biçiminde döndürür.
    pub fn time_range_label(&self) -> Option<String> {
        match (self.start_secs, self.end_secs) {
            (Some(s), Some(e)) => Some(format!("[{:.3} - {:.3}]", s, e)),
            (Some(s), None) => Some(format!("[{:.3}]", s)),
            _ => None,
        }
    }
}

/// Bir sorunun yanıtı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    /// İlgili soru.
    pub question: QuestionId,
    /// Yanıt metni.
    pub text: String,
    /// İçerik durumu.
    pub status: crate::content_status::ContentStatus,
    /// Dayanaklar.
    pub references: Vec<ReferenceId>,
    /// Yanıtı yazan bilirkişi.
    pub author: Option<ExpertProfileId>,
    /// Onay zamanı.
    pub approved_at: Option<Timestamp>,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
}

impl Answer {
    /// Yanıt rapora mesleki kanaat olarak girebilir mi?
    pub fn is_reportable_as_opinion(&self) -> bool {
        self.status.is_reportable_as_expert_opinion()
            && !self.text.trim().is_empty()
            && !self.references.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_range_label_formats_seconds() {
        let r = EvidenceReference {
            id: ReferenceId::new(),
            kind: ReferenceKind::AudioSegment,
            label: "kayit1.wav 00:12".into(),
            file_id: None,
            audio: None,
            transcript: None,
            segment_index: Some(3),
            start_secs: Some(12.5),
            end_secs: Some(18.25),
            quote: None,
            sha256: None,
            created_at: Timestamp::now(),
        };
        assert_eq!(r.time_range_label().unwrap(), "[12.500 - 18.250]");
    }

    #[test]
    fn answer_needs_references_and_approval_to_be_opinion() {
        let mut a = Answer {
            question: QuestionId::new(),
            text: "Yanıt".into(),
            status: crate::content_status::ContentStatus::ExpertApproved,
            references: vec![],
            author: None,
            approved_at: None,
            created_at: Timestamp::now(),
        };
        assert!(!a.is_reportable_as_opinion(), "dayanak yok");
        a.references.push(ReferenceId::new());
        assert!(a.is_reportable_as_opinion());

        a.status = crate::content_status::ContentStatus::ReadyForReview;
        assert!(!a.is_reportable_as_opinion(), "onay yok");
    }
}
