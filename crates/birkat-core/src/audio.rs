//! Ses envanteri ve transkripsiyon veri nesneleri — 7, 9, 10, 11, 12 ve 21.1.

use crate::content_status::ContentStatus;
use crate::ids::*;
use crate::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Ses varlığı — envanterdeki bir ses akışı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAsset {
    /// Kimlik.
    pub id: AudioId,
    /// Kaynak dosya.
    pub file_id: FileId,
    /// Dosya adı.
    pub file_name: String,
    /// Akış indeksi (kapsayıcı içinde birden fazla ses akışı olabilir).
    pub stream_index: usize,
    /// Teknik metadata.
    pub metadata: AudioMetadata,
    /// Tespit edilen dil (otomatik veya bilirkişi tarafından).
    pub language: Option<LanguageAssignment>,
    /// Transkript kimliği (üretildiyse).
    pub transcript: Option<TranscriptId>,
    /// Yinelenen içerik mi?
    pub duplicate_of: Option<AudioId>,
    /// Okunabilirlik durumu.
    pub readability: Readability,
    /// İnceleme işaretleri.
    pub marks: Vec<ReviewMark>,
    /// Bilirkişi notları.
    pub notes: Vec<ExpertNote>,
}

/// Teknik metadata (7.2).
///
/// Kaynak dosyada olmayan metadata üretilmez.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    /// Kapsayıcı biçimi.
    pub container: String,
    /// Codec.
    pub codec: String,
    /// Süre (saniye).
    pub duration_secs: Option<f64>,
    /// Örnekleme frekansı (Hz).
    pub sample_rate: Option<u32>,
    /// Bit derinliği.
    pub bit_depth: Option<u16>,
    /// Bit hızı (bit/s).
    pub bit_rate: Option<u64>,
    /// Kanal sayısı.
    pub channels: Option<u16>,
    /// Kanal düzeni.
    pub channel_layout: Option<String>,
    /// Metadata etiketleri (kaynakta varsa).
    pub tags: Vec<(String, String)>,
    /// Kodlayıcı bilgisi (varsa).
    pub encoder: Option<String>,
    /// Ses akışı başlangıç zamanı (saniye).
    pub stream_start: Option<f64>,
    /// Ses akışı bitiş zamanı (saniye).
    pub stream_end: Option<f64>,
    /// Uyarılar (ör. kaynakta bulunmayan alanlar).
    pub warnings: Vec<String>,
}

impl AudioMetadata {
    /// Bilinmeyen alanların listesi (raporda "kaynakta yok" olarak gösterilir).
    pub fn unknown_fields(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.duration_secs.is_none() {
            v.push("süre");
        }
        if self.sample_rate.is_none() {
            v.push("örnekleme frekansı");
        }
        if self.bit_depth.is_none() {
            v.push("bit derinliği");
        }
        if self.bit_rate.is_none() {
            v.push("bit hızı");
        }
        if self.channels.is_none() {
            v.push("kanal sayısı");
        }
        v
    }
}

/// Ses okunabilirlik durumu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "state", content = "detail")]
pub enum Readability {
    /// Okunabilir.
    Ok,
    /// Codec çözülemedi.
    UndecodableCodec(String),
    /// Dosya bozuk.
    Corrupt(String),
    /// Şifreli.
    Encrypted,
    /// Kısmen okunabilir.
    Partial(String),
}

impl Readability {
    /// Türkçe etiket.
    pub fn label(&self) -> String {
        match self {
            Readability::Ok => "Okunabilir".to_string(),
            Readability::UndecodableCodec(c) => format!("Codec çözülemedi: {c}"),
            Readability::Corrupt(r) => format!("Dosya bozuk: {r}"),
            Readability::Encrypted => "Şifreli".to_string(),
            Readability::Partial(r) => format!("Kısmen okunabilir: {r}"),
        }
    }

    /// Transkripsiyon kuyruğuna alınabilir mi?
    pub fn is_transcribable(&self) -> bool {
        matches!(self, Readability::Ok | Readability::Partial(_))
    }
}

/// Dil ataması.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageAssignment {
    /// Dil kodu (ISO 639-1, ör. "tr").
    pub code: String,
    /// Otomatik mi tespit edildi?
    pub automatic: bool,
    /// Tespit güven göstergesi (yalnızca model göstergesi; doğruluk yüzdesi değildir).
    pub model_indicator: Option<f32>,
    /// Bilirkişi tarafından onaylandı mı?
    pub expert_confirmed: bool,
}

/// İnceleme işareti (12.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewMark {
    /// Başlangıç (saniye).
    pub start: f64,
    /// Bitiş (saniye).
    pub end: f64,
    /// İşaret türü.
    pub kind: ReviewMarkKind,
    /// Etiket.
    pub label: String,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
}

/// İnceleme işareti türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewMarkKind {
    /// İncelenecek bölüm.
    NeedsReview,
    /// Kritik kelime/ifade.
    CriticalPhrase,
    /// Anlaşılamayan bölüm.
    Unintelligible,
    /// Örtüşen konuşma.
    OverlappingSpeech,
    /// Özel isim.
    ProperNoun,
    /// Sayı/tarih/tutar.
    NumericExpression,
    /// Olumsuzluk.
    Negation,
    /// Bilirkişi notu.
    Note,
}

/// Bilirkişi notu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertNote {
    /// Zaman damgası (saniye) veya boş.
    pub at_secs: Option<f64>,
    /// Not metni.
    pub text: String,
    /// Yazan bilirkişi.
    pub author: Option<ExpertProfileId>,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
}

/// Transkripsiyon işi (9.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionJob {
    /// Kimlik.
    pub id: JobId,
    /// Ses varlığı.
    pub audio: AudioId,
    /// Öncelik (küçük değer önce çalışır).
    pub priority: u32,
    /// Durum.
    pub status: JobStatus,
    /// İlerleme yüzdesi (0.0 - 1.0).
    pub progress: f64,
    /// Tamamlanan parça sayısı.
    pub chunks_done: u32,
    /// Toplam parça sayısı.
    pub chunks_total: Option<u32>,
    /// Tahmini kalan saniye (yalnızca ölçümlere dayalıysa).
    pub eta_secs: Option<u64>,
    /// Deneme sayısı.
    pub attempts: u32,
    /// Maksimum deneme.
    pub max_attempts: u32,
    /// Başlangıç zamanı.
    pub started_at: Option<Timestamp>,
    /// Bitiş zamanı.
    pub finished_at: Option<Timestamp>,
    /// Kullanılan model sürümü.
    pub model: Option<ModelVersion>,
    /// Kullanılan çıkarım parametreleri.
    pub params: InferenceParams,
    /// Hata mesajı.
    pub error: Option<String>,
    /// Son kontrol noktası (kesinti sonrası devam için).
    pub checkpoint: Option<String>,
}

/// İş durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    /// Kuyrukta.
    Queued,
    /// Çalışıyor.
    Running,
    /// Duraklatıldı.
    Paused,
    /// Tamamlandı.
    Completed,
    /// Hata.
    Failed,
    /// İptal edildi.
    Cancelled,
}

impl JobStatus {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            JobStatus::Queued => "Kuyrukta",
            JobStatus::Running => "Çalışıyor",
            JobStatus::Paused => "Duraklatıldı",
            JobStatus::Completed => "Tamamlandı",
            JobStatus::Failed => "Hata",
            JobStatus::Cancelled => "İptal edildi",
        }
    }
}

/// Model sürüm bilgisi (9.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// Model adı.
    pub name: String,
    /// Model sürümü.
    pub version: String,
    /// Model dosyası hash'i.
    pub file_hash: String,
    /// Lisans.
    pub license: String,
    /// Model dosyası yolu.
    pub path: String,
}

/// Çıkarım parametreleri (9.2).
///
/// Kullanıcıya bu parametreler sorulmaz; sistem güvenli varsayılanları
/// kendisi seçer ve kaydeder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceParams {
    /// Donanım.
    pub hardware: String,
    /// İşletim sistemi.
    pub os: String,
    /// Çıkarım motoru sürümü.
    pub engine_version: String,
    /// Kullanılan backend (CPU/CUDA/Metal).
    pub backend: String,
    /// Hassasiyet/quantization.
    pub precision: String,
    /// Dil (None = otomatik).
    pub language: Option<String>,
    /// Görev.
    pub task: String,
    /// Sıcaklık.
    pub temperature: f32,
    /// Beam size.
    pub beam_size: usize,
    /// Parça uzunluğu (saniye).
    pub chunk_secs: f32,
    /// Parçalar arası örtüşme (saniye).
    pub overlap_secs: f32,
}

impl Default for InferenceParams {
    fn default() -> Self {
        Self {
            hardware: String::new(),
            os: String::new(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            backend: "cpu".to_string(),
            precision: "f16".to_string(),
            language: None,
            task: "transcribe".to_string(),
            temperature: 0.0,
            beam_size: 5,
            chunk_secs: 30.0,
            overlap_secs: 2.0,
        }
    }
}

/// Transkript (12.3 sürümleme).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// Kimlik.
    pub id: TranscriptId,
    /// Ses varlığı.
    pub audio: AudioId,
    /// Sürüm.
    pub version: TranscriptVersion,
    /// İçerik durumu.
    pub status: ContentStatus,
    /// Bölümler.
    pub segments: Vec<TranscriptSegment>,
    /// Kullanılan model.
    pub model: Option<ModelVersion>,
    /// Üretim zamanı.
    pub created_at: Timestamp,
    /// Dil.
    pub language: Option<String>,
    /// Konuşmacı etiketleri.
    pub speakers: Vec<SpeakerLabel>,
    /// Kalite kontrolü sonucu.
    pub quality: Option<QualityReport>,
    /// Değişiklik geçmişi.
    pub changes: Vec<TranscriptChange>,
}

/// Transkript sürümü (12.3).
///
/// İlk makine çıktısı asla insan düzeltmesiyle üzerine yazılarak kaybolmaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TranscriptVersion {
    /// Makine tarafından oluşturulan ilk transkript.
    MachineRaw,
    /// Otomatik kalite kontrolü tamamlanmış taslak.
    QualityCheckedDraft,
    /// Bilirkişi tarafından incelenen transkript.
    ExpertReviewed,
    /// Bilirkişi tarafından düzeltilen transkript.
    ExpertCorrected,
    /// Bilirkişi tarafından onaylanan nihai transkript.
    ExpertApprovedFinal,
}

impl TranscriptVersion {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            TranscriptVersion::MachineRaw => "Makine tarafından oluşturulan ilk transkript",
            TranscriptVersion::QualityCheckedDraft => "Kalite kontrolü tamamlanmış taslak",
            TranscriptVersion::ExpertReviewed => "Bilirkişi tarafından incelenen transkript",
            TranscriptVersion::ExpertCorrected => "Bilirkişi tarafından düzeltilen transkript",
            TranscriptVersion::ExpertApprovedFinal => {
                "Bilirkişi tarafından onaylanan nihai transkript"
            }
        }
    }

    /// Bu sürüm değiştirilemez mi?
    ///
    /// İlk makine çıktısı ve onaylı nihai sürüm korunur.
    pub fn is_immutable(&self) -> bool {
        matches!(
            self,
            TranscriptVersion::MachineRaw | TranscriptVersion::ExpertApprovedFinal
        )
    }

    /// Bir sonraki insan inceleme sürümü.
    pub fn next_human_version(&self) -> TranscriptVersion {
        match self {
            TranscriptVersion::MachineRaw | TranscriptVersion::QualityCheckedDraft => {
                TranscriptVersion::ExpertReviewed
            }
            TranscriptVersion::ExpertReviewed => TranscriptVersion::ExpertCorrected,
            TranscriptVersion::ExpertCorrected => TranscriptVersion::ExpertApprovedFinal,
            TranscriptVersion::ExpertApprovedFinal => TranscriptVersion::ExpertApprovedFinal,
        }
    }
}

/// Transkript bölümü.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    /// Sıra numarası.
    pub index: usize,
    /// Başlangıç (saniye).
    pub start: f64,
    /// Bitiş (saniye).
    pub end: f64,
    /// Metin (birebir).
    pub text: String,
    /// Kelime düzeyi zaman kodları (yalnızca doğrulanmış yöntemle elde edildiyse).
    pub words: Vec<WordTimestamp>,
    /// Konuşmacı etiketi.
    pub speaker: Option<SpeakerId>,
    /// Belirsizlik işaretleri.
    pub annotations: Vec<Annotation>,
    /// Model göstergesi (olasılık); doğruluk yüzdesi değildir.
    pub model_confidence: Option<f32>,
    /// Ses düzeyi göstergesi (dBFS).
    pub level_dbfs: Option<f32>,
}

impl TranscriptSegment {
    /// Bölüm süresi.
    pub fn duration(&self) -> f64 {
        (self.end - self.start).max(0.0)
    }

    /// Zaman kodu ses süresiyle tutarlı mı?
    pub fn is_time_consistent(&self, audio_duration: f64) -> bool {
        self.start >= 0.0 && self.end >= self.start && self.end <= audio_duration + 0.5
    }
}

/// Kelime düzeyi zaman kodu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTimestamp {
    /// Kelime.
    pub word: String,
    /// Başlangıç (saniye).
    pub start: f64,
    /// Bitiş (saniye).
    pub end: f64,
    /// Kaynak: model çıktısı mı, hizalama mı?
    pub source: WordTimestampSource,
}

/// Kelime zamanı kaynağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WordTimestampSource {
    /// Model doğrudan üretti.
    ModelOutput,
    /// Doğrulanmış hizalama yöntemiyle elde edildi.
    VerifiedAlignment,
}

/// Belirsizlik işareti (10.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Annotation {
    /// `[anlaşılamayan bölüm]`
    Unintelligible,
    /// `[belirsiz: ...]`
    Uncertain,
    /// `[üst üste konuşma]`
    OverlappingSpeech,
    /// `[konuşma kesildi]`
    SpeechCutOff,
    /// `[ses anlaşılmıyor]`
    Inaudible,
    /// `[konuşmacı etiketi belirsiz]`
    SpeakerUncertain,
}

impl Annotation {
    /// Varsayılan Türkçe işaret metni.
    ///
    /// İşaretlerin kesin standardı kullanıcı tarafından düzenlenebilir;
    /// ancak her dosyada aynı kuralla uygulanır.
    pub fn default_marker(&self) -> &'static str {
        match self {
            Annotation::Unintelligible => "[anlaşılamayan bölüm]",
            Annotation::Uncertain => "[belirsiz: ...]",
            Annotation::OverlappingSpeech => "[üst üste konuşma]",
            Annotation::SpeechCutOff => "[konuşma kesildi]",
            Annotation::Inaudible => "[ses anlaşılmıyor]",
            Annotation::SpeakerUncertain => "[konuşmacı etiketi belirsiz]",
        }
    }
}

/// Konuşmacı kimliği (geçici etiket).
///
/// Bu etiket gerçek kişi kimliği veya biyometrik tanımlama değildir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpeakerId(pub u32);

impl SpeakerId {
    /// Görüntülenecek geçici etiket: "Konuşmacı 1", "Konuşmacı 2" ...
    pub fn display_label(&self) -> String {
        format!("Konuşmacı {}", self.0)
    }
}

/// Konuşmacı etiketi kaydı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerLabel {
    /// Geçici konuşmacı kimliği.
    pub id: SpeakerId,
    /// Otomatik öneri mi?
    pub automatic_suggestion: bool,
    /// Bilirkişi tarafından onaylandı mı?
    pub expert_confirmed: bool,
    /// Bilirkişi notu (ör. "müşteki olabilir" — kesin kimlik değildir).
    pub expert_note: Option<String>,
}

/// Transkript değişikliği (12.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptChange {
    /// Bölüm indeksi.
    pub segment_index: usize,
    /// Alan adı.
    pub field: String,
    /// Önceki değer.
    pub previous: String,
    /// Yeni değer.
    pub new: String,
    /// Değişiklik zamanı.
    pub at: Timestamp,
    /// İşlemi yapan kullanıcı.
    pub user: String,
    /// Gerekçe/not.
    pub reason: Option<String>,
}

/// Transkripsiyon kalite kontrol raporu (11).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    /// Bulgular.
    pub findings: Vec<QualityFinding>,
    /// Tamamlanma zamanı.
    pub completed_at: Timestamp,
    /// Kontrol edilen bölüm sayısı.
    pub segments_checked: usize,
}

impl QualityReport {
    /// İncelenmesi gereken bulgular (öncelik sırasına göre).
    pub fn review_priorities(&self) -> Vec<&QualityFinding> {
        let mut v: Vec<&QualityFinding> = self
            .findings
            .iter()
            .filter(|f| f.severity >= QualitySeverity::Medium)
            .collect();
        v.sort_by(|a, b| b.severity.cmp(&a.severity));
        v
    }

    /// Bilirkişi onayı gerektiren bulgu var mı?
    pub fn needs_expert_review(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity >= QualitySeverity::Medium)
    }
}

/// Kalite bulgusu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFinding {
    /// Bulgu türü.
    pub kind: QualityFindingKind,
    /// Önem derecesi.
    pub severity: QualitySeverity,
    /// Bölüm indeksi (varsa).
    pub segment_index: Option<usize>,
    /// Başlangıç (saniye).
    pub start: Option<f64>,
    /// Bitiş (saniye).
    pub end: Option<f64>,
    /// Açıklama.
    pub description: String,
}

/// Kalite bulgu türleri (11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualityFindingKind {
    /// Uzun sessizlik.
    LongSilence,
    /// Konuşma varken boş transkript.
    SpeechButEmptyTranscript,
    /// Sessizlikte yapay metin.
    TextInSilence,
    /// Tekrarlanan kelime ve cümle.
    RepeatedText,
    /// Şüpheli metin sıçraması.
    SuspiciousTextJump,
    /// Zaman kodu çakışması.
    TimeCodeOverlap,
    /// Zaman kodu boşluğu.
    TimeCodeGap,
    /// Örtüşen konuşma.
    OverlappingSpeech,
    /// Çok düşük ses.
    VeryLowAudio,
    /// Bozuk ses bölümü.
    CorruptAudio,
    /// Beklenmeyen dil değişimi.
    UnexpectedLanguageChange,
    /// Parça sınırında tekrar veya kayıp.
    ChunkBoundaryArtifact,
    /// Anlaşılması güç özel isim.
    DifficultProperNoun,
    /// Sayı, tarih ve tutar.
    NumberDateAmount,
    /// Olumsuzluk sözcükleri.
    Negation,
    /// Model kaynaklı olası halüsinasyon.
    PossibleHallucination,
}

impl QualityFindingKind {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            QualityFindingKind::LongSilence => "Uzun sessizlik",
            QualityFindingKind::SpeechButEmptyTranscript => "Konuşma var, transkript boş",
            QualityFindingKind::TextInSilence => "Sessizlikte metin üretilmiş",
            QualityFindingKind::RepeatedText => "Tekrarlanan kelime veya cümle",
            QualityFindingKind::SuspiciousTextJump => "Şüpheli metin sıçraması",
            QualityFindingKind::TimeCodeOverlap => "Zaman kodu çakışması",
            QualityFindingKind::TimeCodeGap => "Zaman kodu boşluğu",
            QualityFindingKind::OverlappingSpeech => "Örtüşen konuşma",
            QualityFindingKind::VeryLowAudio => "Çok düşük ses",
            QualityFindingKind::CorruptAudio => "Bozuk ses bölümü",
            QualityFindingKind::UnexpectedLanguageChange => "Beklenmeyen dil değişimi",
            QualityFindingKind::ChunkBoundaryArtifact => "Parça sınırında tekrar veya kayıp",
            QualityFindingKind::DifficultProperNoun => "Anlaşılması güç özel isim",
            QualityFindingKind::NumberDateAmount => "Sayı, tarih veya tutar",
            QualityFindingKind::Negation => "Olumsuzluk sözcüğü",
            QualityFindingKind::PossibleHallucination => "Olası halüsinasyon",
        }
    }
}

/// Bulgu önem derecesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualitySeverity {
    /// Bilgi.
    Info,
    /// Düşük.
    Low,
    /// Orta; incelenmeli.
    Medium,
    /// Yüksek; mutlaka incelenmeli.
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_raw_transcript_is_immutable() {
        assert!(TranscriptVersion::MachineRaw.is_immutable());
        assert!(TranscriptVersion::ExpertApprovedFinal.is_immutable());
        assert!(!TranscriptVersion::ExpertCorrected.is_immutable());
    }

    #[test]
    fn version_progression_is_monotonic() {
        let mut v = TranscriptVersion::MachineRaw;
        v = v.next_human_version();
        assert_eq!(v, TranscriptVersion::ExpertReviewed);
        v = v.next_human_version();
        assert_eq!(v, TranscriptVersion::ExpertCorrected);
        v = v.next_human_version();
        assert_eq!(v, TranscriptVersion::ExpertApprovedFinal);
        v = v.next_human_version();
        assert_eq!(v, TranscriptVersion::ExpertApprovedFinal);
    }

    #[test]
    fn speaker_label_is_not_biometric() {
        assert_eq!(SpeakerId(1).display_label(), "Konuşmacı 1");
    }

    #[test]
    fn annotations_have_default_markers() {
        assert_eq!(
            Annotation::Unintelligible.default_marker(),
            "[anlaşılamayan bölüm]"
        );
        assert_eq!(Annotation::Uncertain.default_marker(), "[belirsiz: ...]");
        assert_eq!(
            Annotation::OverlappingSpeech.default_marker(),
            "[üst üste konuşma]"
        );
    }

    #[test]
    fn segment_time_consistency_detects_overflow() {
        let s = TranscriptSegment {
            index: 0,
            start: 1.0,
            end: 5.0,
            text: "merhaba".into(),
            words: vec![],
            speaker: None,
            annotations: vec![],
            model_confidence: None,
            level_dbfs: None,
        };
        assert!(s.is_time_consistent(10.0));
        assert!(!s.is_time_consistent(3.0));
    }

    #[test]
    fn quality_report_prioritises_high_severity() {
        let r = QualityReport {
            findings: vec![
                QualityFinding {
                    kind: QualityFindingKind::LongSilence,
                    severity: QualitySeverity::Low,
                    segment_index: None,
                    start: None,
                    end: None,
                    description: "d".into(),
                },
                QualityFinding {
                    kind: QualityFindingKind::PossibleHallucination,
                    severity: QualitySeverity::High,
                    segment_index: Some(2),
                    start: None,
                    end: None,
                    description: "d".into(),
                },
            ],
            completed_at: Timestamp::now(),
            segments_checked: 3,
        };
        let p = r.review_priorities();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].kind, QualityFindingKind::PossibleHallucination);
        assert!(r.needs_expert_review());
    }

    #[test]
    fn undecodable_audio_is_not_transcribable() {
        assert!(!Readability::UndecodableCodec("amr".into()).is_transcribable());
        assert!(Readability::Ok.is_transcribable());
        assert!(Readability::Partial("x".into()).is_transcribable());
    }

    #[test]
    fn unknown_metadata_fields_are_reported() {
        let m = AudioMetadata {
            container: "WAV".into(),
            codec: "PCM".into(),
            duration_secs: Some(1.0),
            sample_rate: None,
            bit_depth: None,
            bit_rate: None,
            channels: Some(2),
            channel_layout: None,
            tags: vec![],
            encoder: None,
            stream_start: None,
            stream_end: None,
            warnings: vec![],
        };
        let u = m.unknown_fields();
        assert!(u.contains(&"örnekleme frekansı"));
        assert!(!u.contains(&"süre"));
    }
}
