//! Delil yönetimi veri nesneleri — 5, 6 ve 21.1.

use crate::ids::*;
use crate::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Hash algoritması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HashAlgorithm {
    /// SHA-256 (zorunlu).
    Sha256,
    /// SHA-1 (eski imaj uyumluluğu).
    Sha1,
    /// MD5 (eski imaj uyumluluğu).
    Md5,
}

impl HashAlgorithm {
    /// Algoritmanın adı.
    pub fn name(&self) -> &'static str {
        match self {
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha1 => "SHA-1",
            HashAlgorithm::Md5 => "MD5",
        }
    }

    /// Bu algoritma güvenlik için esas alınır mı?
    pub fn is_security_baseline(&self) -> bool {
        matches!(self, HashAlgorithm::Sha256)
    }
}

/// Hash hesaplama aşaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HashStage {
    /// Kaynak okuma.
    SourceRead,
    /// Hedef kopya okuma.
    TargetRead,
    /// İmaj oluşturma.
    ImageCreated,
    /// İmaj doğrulama.
    ImageVerified,
    /// Rapor üretimi.
    ReportGenerated,
    /// İmza öncesi.
    PreSignature,
    /// İmza sonrası.
    PostSignature,
}

/// Tek bir hash kaydı (6.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRecord {
    /// Dosya kimliği.
    pub file_id: FileId,
    /// Algoritma.
    pub algorithm: HashAlgorithm,
    /// Hash değeri (küçük harf onaltılık).
    pub value: String,
    /// Hesaplama zamanı.
    pub computed_at: Timestamp,
    /// Hesaplama aşaması.
    pub stage: HashStage,
    /// Kaynak veya hedef türü.
    pub side: HashSide,
    /// Doğrulama sonucu (karşılaştırma yapıldıysa).
    pub verification: Option<HashVerification>,
}

/// Hash'in hangi tarafa ait olduğu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HashSide {
    /// Kaynak materyal.
    Source,
    /// Hedef kopya.
    Target,
    /// Üretilen imaj.
    Image,
    /// Rapor veya belge.
    Document,
}

/// Hash doğrulama sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashVerification {
    /// Karşılaştırılan diğer hash değeri.
    pub expected: String,
    /// Eşleşti mi?
    pub matched: bool,
    /// Doğrulama zamanı.
    pub verified_at: Timestamp,
}

impl HashRecord {
    /// Yeni hash kaydı.
    pub fn new(
        file_id: FileId,
        algorithm: HashAlgorithm,
        value: impl Into<String>,
        stage: HashStage,
        side: HashSide,
    ) -> Self {
        Self {
            file_id,
            algorithm,
            value: value.into(),
            computed_at: Timestamp::now(),
            stage,
            side,
            verification: None,
        }
    }

    /// Kaynak ve hedef hash'lerini karşılaştırır.
    pub fn verify_against(&self, other: &HashRecord) -> HashVerification {
        HashVerification {
            expected: other.value.clone(),
            matched: self.value.eq_ignore_ascii_case(&other.value),
            verified_at: Timestamp::now(),
        }
    }
}

/// Dosya nesnesi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileObject {
    /// Kimlik.
    pub id: FileId,
    /// Dosya adı.
    pub name: String,
    /// Kaynak yolu.
    pub source_path: String,
    /// Hedef (kasa) yolu.
    pub vault_path: Option<String>,
    /// Boyut (bayt).
    pub size_bytes: u64,
    /// Son değişiklik zamanı (dosya sisteminden).
    pub modified_at: Option<Timestamp>,
    /// Oluşturma zamanı (dosya sisteminden).
    pub created_at_fs: Option<Timestamp>,
    /// Erişim zamanı (dosya sisteminden).
    pub accessed_at_fs: Option<Timestamp>,
    /// İçerik imzası (magic bytes) tespiti.
    pub detected_kind: DetectedKind,
    /// Uzantı.
    pub extension: Option<String>,
    /// Hash kayıtları.
    pub hashes: Vec<HashRecord>,
    /// Kasa göreli yolu.
    pub relative_path: String,
}

/// Dosya türü tespiti (yalnızca uzantıya güvenilmez).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "kind", content = "detail")]
pub enum DetectedKind {
    /// Bilinmeyen veya tanınmayan.
    Unknown,
    /// PDF.
    Pdf,
    /// ZIP arşivi.
    Zip,
    /// 7z arşivi.
    SevenZip,
    /// RAR arşivi.
    Rar,
    /// DOCX (ZIP/OPC).
    Docx,
    /// DOC (OLE2).
    Doc,
    /// RTF.
    Rtf,
    /// ODT (ZIP/OPC).
    Odt,
    /// UDF (UYAP doküman formatı).
    Udf,
    /// Düz metin.
    Text,
    /// JPEG görüntü.
    Jpeg,
    /// PNG görüntü.
    Png,
    /// TIFF görüntü.
    Tiff,
    /// Ses dosyası; kapsayıcı adı ile.
    Audio(String),
    /// Video/kapsayıcı dosyası; kapsayıcı adı ile.
    Container(String),
    /// EWF imajı (E01/L01).
    Ewf,
    /// Ham imaj.
    Raw,
}

impl DetectedKind {
    /// Türkçe etiket.
    pub fn label(&self) -> String {
        match self {
            DetectedKind::Unknown => "Bilinmeyen".to_string(),
            DetectedKind::Pdf => "PDF belgesi".to_string(),
            DetectedKind::Zip => "ZIP arşivi".to_string(),
            DetectedKind::SevenZip => "7z arşivi".to_string(),
            DetectedKind::Rar => "RAR arşivi".to_string(),
            DetectedKind::Docx => "DOCX belgesi".to_string(),
            DetectedKind::Doc => "DOC belgesi".to_string(),
            DetectedKind::Rtf => "RTF belgesi".to_string(),
            DetectedKind::Odt => "ODT belgesi".to_string(),
            DetectedKind::Udf => "UDF belgesi".to_string(),
            DetectedKind::Text => "Metin belgesi".to_string(),
            DetectedKind::Jpeg => "JPEG görüntü".to_string(),
            DetectedKind::Png => "PNG görüntü".to_string(),
            DetectedKind::Tiff => "TIFF görüntü".to_string(),
            DetectedKind::Audio(c) => format!("Ses dosyası ({c})"),
            DetectedKind::Container(c) => format!("Kapsayıcı ({c})"),
            DetectedKind::Ewf => "EWF imajı (E01/L01)".to_string(),
            DetectedKind::Raw => "Ham imaj".to_string(),
        }
    }

    /// Bu tür ses içerebilir mi?
    pub fn may_contain_audio(&self) -> bool {
        matches!(
            self,
            DetectedKind::Audio(_) | DetectedKind::Container(_)
        )
    }
}

/// Fiziksel aygıt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalDevice {
    /// Kimlik.
    pub id: DeviceId,
    /// Üretici.
    pub vendor: Option<String>,
    /// Model.
    pub model: Option<String>,
    /// Seri numarası.
    pub serial: Option<String>,
    /// Kapasite (bayt).
    pub capacity_bytes: Option<u64>,
    /// Disk numarası.
    pub disk_number: Option<u32>,
    /// Bölüm bilgileri.
    pub partitions: Vec<PartitionInfo>,
    /// Dosya sistemi.
    pub filesystem: Option<String>,
    /// Bağlantı türü (USB, SD, SATA, NVMe...).
    pub connection: ConnectionKind,
    /// Sürücü harfi.
    pub drive_letter: Option<String>,
    /// Bağlanma zamanı.
    pub attached_at: Timestamp,
    /// Ayrılma zamanı.
    pub detached_at: Option<Timestamp>,
    /// Yazma koruması durumu.
    pub write_protection: WriteProtectionState,
    /// Sistem diski mi?
    pub is_system_disk: bool,
    /// Ek notlar.
    pub notes: Vec<String>,
}

/// Bölüm bilgisi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    /// Bölüm numarası.
    pub index: u32,
    /// Başlangıç sektörü.
    pub start_sector: Option<u64>,
    /// Sektör sayısı.
    pub sector_count: Option<u64>,
    /// Dosya sistemi.
    pub filesystem: Option<String>,
    /// Sürücü harfi.
    pub drive_letter: Option<String>,
    /// Etiket.
    pub label: Option<String>,
}

/// Aygıt bağlantı türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionKind {
    /// USB.
    Usb,
    /// SD / microSD kart.
    SdCard,
    /// Harici SATA/SSD.
    ExternalSata,
    /// Dahili SATA.
    InternalSata,
    /// NVMe.
    Nvme,
    /// Optik sürücü (CD/DVD).
    Optical,
    /// Ağ kaynağı.
    Network,
    /// Bilinmiyor.
    #[default]
    Unknown,
}

/// Yazma koruması durumu (5.3).
///
/// Yazılımsal koruma donanımsal korumaya eşdeğer gösterilmez.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "state", content = "detail")]
pub enum WriteProtectionState {
    /// Donanımsal yazma engelleyici.
    HardwareBlocker,
    /// Aygıtın kendi salt okunur anahtarı/özelliği.
    DeviceReadOnlySwitch,
    /// Windows yazılımsal salt okunur özelliği.
    WindowsSoftwareReadOnly,
    /// Koruma yok.
    #[default]
    None,
    /// Durum doğrulanamadı.
    Unverified(String),
}

impl WriteProtectionState {
    /// Türkçe etiket.
    pub fn label(&self) -> String {
        match self {
            WriteProtectionState::HardwareBlocker => "Donanımsal yazma engelleyici".to_string(),
            WriteProtectionState::DeviceReadOnlySwitch => {
                "Aygıtın salt okunur anahtarı".to_string()
            }
            WriteProtectionState::WindowsSoftwareReadOnly => {
                "Windows yazılımsal salt okunur".to_string()
            }
            WriteProtectionState::None => "Koruma yok".to_string(),
            WriteProtectionState::Unverified(r) => format!("Durum doğrulanamadı: {r}"),
        }
    }

    /// Bu durum "korumalı" olarak sunulabilir mi?
    ///
    /// Doğrulanamayan durum korumalı sayılmaz.
    pub fn is_protected(&self) -> bool {
        matches!(
            self,
            WriteProtectionState::HardwareBlocker
                | WriteProtectionState::DeviceReadOnlySwitch
                | WriteProtectionState::WindowsSoftwareReadOnly
        )
    }

    /// Donanımsal düzeyde koruma mı?
    ///
    /// Yüksek güvence gerektiren işlemler bu ayrımı kullanır.
    pub fn is_hardware_level(&self) -> bool {
        matches!(
            self,
            WriteProtectionState::HardwareBlocker | WriteProtectionState::DeviceReadOnlySwitch
        )
    }
}

/// Edinim yöntemi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AcquisitionMethod {
    /// Dosya bazlı edinim.
    FileBased,
    /// Mantıksal imaj (L01).
    LogicalImage,
    /// Fiziksel imaj (E01).
    PhysicalImage,
    /// Ham fiziksel imaj.
    RawImage,
    /// Arşivden çıkarma.
    ArchiveExtraction,
}

impl AcquisitionMethod {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            AcquisitionMethod::FileBased => "Dosya bazlı edinim",
            AcquisitionMethod::LogicalImage => "Mantıksal imaj (L01)",
            AcquisitionMethod::PhysicalImage => "Fiziksel imaj (E01)",
            AcquisitionMethod::RawImage => "Ham fiziksel imaj",
            AcquisitionMethod::ArchiveExtraction => "Arşivden çıkarma",
        }
    }
}

/// Edinim işlemi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionOperation {
    /// Kimlik.
    pub id: AcquisitionId,
    /// Kaynak aygıt.
    pub device: Option<DeviceId>,
    /// Yöntem.
    pub method: AcquisitionMethod,
    /// Kaynak yolu.
    pub source: String,
    /// Hedef yolu.
    pub target: String,
    /// Başlangıç zamanı.
    pub started_at: Timestamp,
    /// Bitiş zamanı.
    pub finished_at: Option<Timestamp>,
    /// İşlem durumu.
    pub status: OperationStatus,
    /// Okunan bayt.
    pub bytes_read: u64,
    /// Toplam bayt (biliniyorsa).
    pub bytes_total: Option<u64>,
    /// Okunamayan sektörler.
    pub unreadable_sectors: Vec<u64>,
    /// Hata mesajları.
    pub errors: Vec<String>,
    /// Uyarılar.
    pub warnings: Vec<String>,
    /// Devam edebilmek için son kontrol noktası.
    pub checkpoint: Option<String>,
    /// Kaynak hash kaydı.
    pub source_hashes: Vec<HashRecord>,
    /// Hedef hash kaydı.
    pub target_hashes: Vec<HashRecord>,
}

/// İşlem durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationStatus {
    /// Bekliyor.
    Pending,
    /// Sürüyor.
    Running,
    /// Duraklatıldı.
    Paused,
    /// Tamamlandı.
    Completed,
    /// Hata ile bitti.
    Failed,
    /// İptal edildi.
    Cancelled,
    /// Kısmen tamamlandı.
    Partial,
}

impl AcquisitionOperation {
    /// İlerleme oranı (0.0 - 1.0); toplam bilinmiyorsa `None`.
    pub fn progress(&self) -> Option<f64> {
        self.bytes_total
            .filter(|t| *t > 0)
            .map(|t| (self.bytes_read as f64 / t as f64).clamp(0.0, 1.0))
    }
}

/// Delil imajı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceImage {
    /// Kimlik.
    pub id: ImageId,
    /// Format.
    pub format: ImageFormat,
    /// Segment dosyaları.
    pub segments: Vec<String>,
    /// Mantıksal boyut.
    pub logical_size: Option<u64>,
    /// Metaveri.
    pub metadata: Vec<(String, String)>,
    /// İmaj hash kayıtları.
    pub hashes: Vec<HashRecord>,
    /// Doğrulama durumu.
    pub verified: bool,
    /// İçindeki dosya sayısı (katalog çıkarıldıysa).
    pub file_count: Option<u64>,
    /// Desteklenmeyen özellikler (şifreleme, sıkıştırma).
    pub unsupported_features: Vec<String>,
}

/// İmaj formatı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImageFormat {
    /// E01 (EWF).
    E01,
    /// L01 (EWF mantıksal).
    L01,
    /// Ex01.
    Ex01,
    /// Lx01.
    Lx01,
    /// Ham (dd).
    Raw,
}

impl ImageFormat {
    /// Dosya uzantıları.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            ImageFormat::E01 => &["E01", "e01"],
            ImageFormat::L01 => &["L01", "l01"],
            ImageFormat::Ex01 => &["Ex01", "ex01", "EX01"],
            ImageFormat::Lx01 => &["Lx01", "lx01", "LX01"],
            ImageFormat::Raw => &["dd", "raw", "img", "bin"],
        }
    }
}

/// Delil kaydı — kasadaki bir materyal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Kimlik.
    pub id: EvidenceId,
    /// Kaynak türü.
    pub source: EvidenceSource,
    /// Kaynak açıklaması.
    pub description: String,
    /// İlgili fiziksel aygıt.
    pub device: Option<DeviceId>,
    /// İlgili edinim işlemi.
    pub acquisition: Option<AcquisitionId>,
    /// İlgili imaj.
    pub image: Option<ImageId>,
    /// Kasadaki dosyalar.
    pub files: Vec<FileId>,
    /// Teslim kaydı.
    pub custody: Vec<CustodyRecord>,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
}

/// Delil kaynağı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "kind", content = "detail")]
pub enum EvidenceSource {
    /// Bilgisayar klasörü.
    Folder(String),
    /// Tekil dosya.
    File(String),
    /// USB bellek.
    Usb,
    /// SD/microSD kart.
    SdCard,
    /// Harici HDD/SSD.
    ExternalDrive,
    /// CD/DVD.
    OpticalDisc,
    /// Arşiv.
    Archive(String),
    /// UYAP'tan indirilmiş.
    UyapDownload,
    /// Önceden edinilmiş imaj.
    PreexistingImage(String),
    /// Bilirkişi tarafından seçilmiş dosyalar.
    ExpertSelection,
}

/// Fiziksel teslim kaydı (5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyRecord {
    /// Teslim eden kişi.
    pub delivered_by: String,
    /// Teslim alan kişi.
    pub received_by: String,
    /// Tarih ve saat.
    pub at: Timestamp,
    /// Materyal türü.
    pub material_kind: String,
    /// Üretici/model.
    pub make_model: Option<String>,
    /// Seri numarası.
    pub serial: Option<String>,
    /// Mühür numarası.
    pub seal_number: Option<String>,
    /// Ambalaj bilgisi.
    pub packaging: Option<String>,
    /// Fiziksel durum.
    pub physical_condition: Option<String>,
    /// Teslim notu.
    pub note: Option<String>,
    /// İade tarihi.
    pub returned_at: Option<Timestamp>,
    /// İade alan.
    pub return_receiver: Option<String>,
    /// İade veren.
    pub return_giver: Option<String>,
    /// İade durumu.
    pub return_status: Option<String>,
    /// Fotoğraf/tutanak ekleri.
    pub attachments: Vec<AttachmentId>,
}

impl CustodyRecord {
    /// Yeni teslim kaydı.
    pub fn new(delivered_by: impl Into<String>, received_by: impl Into<String>) -> Self {
        Self {
            delivered_by: delivered_by.into(),
            received_by: received_by.into(),
            at: Timestamp::now(),
            material_kind: String::new(),
            make_model: None,
            serial: None,
            seal_number: None,
            packaging: None,
            physical_condition: None,
            note: None,
            returned_at: None,
            return_receiver: None,
            return_giver: None,
            return_status: None,
            attachments: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_security_baseline() {
        assert!(HashAlgorithm::Sha256.is_security_baseline());
        assert!(!HashAlgorithm::Md5.is_security_baseline());
    }

    #[test]
    fn hash_verification_matches_case_insensitively() {
        let fid = FileId::new();
        let a = HashRecord::new(
            fid.clone(),
            HashAlgorithm::Sha256,
            "ABCDEF",
            HashStage::SourceRead,
            HashSide::Source,
        );
        let b = HashRecord::new(
            fid,
            HashAlgorithm::Sha256,
            "abcdef",
            HashStage::TargetRead,
            HashSide::Target,
        );
        assert!(a.verify_against(&b).matched);
    }

    #[test]
    fn unverified_write_protection_is_not_protected() {
        let s = WriteProtectionState::Unverified("yönetici yetkisi yok".into());
        assert!(!s.is_protected());
        assert!(!s.is_hardware_level());
    }

    #[test]
    fn software_read_only_is_not_hardware_level() {
        let s = WriteProtectionState::WindowsSoftwareReadOnly;
        assert!(s.is_protected());
        assert!(!s.is_hardware_level());
    }

    #[test]
    fn hardware_blocker_is_hardware_level() {
        assert!(WriteProtectionState::HardwareBlocker.is_hardware_level());
        assert!(WriteProtectionState::DeviceReadOnlySwitch.is_hardware_level());
    }

    #[test]
    fn acquisition_progress_is_none_without_total() {
        let op = AcquisitionOperation {
            id: AcquisitionId::new(),
            device: None,
            method: AcquisitionMethod::FileBased,
            source: "C:/a".into(),
            target: "D:/b".into(),
            started_at: Timestamp::now(),
            finished_at: None,
            status: OperationStatus::Running,
            bytes_read: 100,
            bytes_total: None,
            unreadable_sectors: vec![],
            errors: vec![],
            warnings: vec![],
            checkpoint: None,
            source_hashes: vec![],
            target_hashes: vec![],
        };
        assert!(op.progress().is_none());
    }

    #[test]
    fn container_kinds_may_contain_audio() {
        assert!(DetectedKind::Container("MP4".into()).may_contain_audio());
        assert!(!DetectedKind::Pdf.may_contain_audio());
    }
}
