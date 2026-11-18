//! Bilirkişi profili ve kişisel ayarlar — 3. bölüm.

use crate::ids::ExpertProfileId;
use crate::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Bilirkişi profili.
///
/// Kişisel bilgiler her dosyada yeniden yazılmaz; rapor üretiminde otomatik
/// kullanılır.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertProfile {
    /// Kimlik.
    pub id: ExpertProfileId,
    /// Adı ve soyadı.
    pub full_name: String,
    /// Unvanı.
    pub title: Option<String>,
    /// Bilirkişi sicil numarası.
    pub registry_number: Option<String>,
    /// Uzmanlık alanı.
    pub specialty: String,
    /// Alt uzmanlık alanı.
    pub sub_specialty: Option<String>,
    /// Kurum/işletme bilgisi.
    pub institution: Option<String>,
    /// İletişim bilgileri.
    pub contact: ContactInfo,
    /// Varsayılan rapor şablonu.
    pub default_report_template: Option<String>,
    /// Varsayılan imza ve teslim tercihleri.
    pub signature_preferences: SignaturePreferences,
    /// Arşivleme ve yedekleme tercihleri.
    pub archive_preferences: ArchivePreferences,
    /// Çalışma takvimi.
    pub work_calendar: WorkCalendar,
    /// Hatırlatma tercihleri.
    pub reminders: ReminderPreferences,
    /// Tüzel kişi mi?
    pub is_legal_entity: bool,
    /// Tüzel kişi bilgileri (varsa).
    pub legal_entity: Option<LegalEntity>,
    /// Oluşturma zamanı.
    pub created_at: Timestamp,
}

/// Tüzel kişi bilirkişi yapısı (14.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalEntity {
    /// Ticaret unvanı.
    pub trade_name: String,
    /// Vergi dairesi.
    pub tax_office: Option<String>,
    /// Vergi numarası.
    pub tax_number: Option<String>,
    /// MERSİS numarası.
    pub mersis_number: Option<String>,
    /// Adres.
    pub address: Option<String>,
    /// Raporu imzalayacak yetkililer.
    pub authorized_signatories: Vec<String>,
}

/// İletişim bilgileri.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactInfo {
    /// Telefon.
    pub phone: Option<String>,
    /// E-posta.
    pub email: Option<String>,
    /// Adres.
    pub address: Option<String>,
    /// UETS adresi.
    pub uets: Option<String>,
}

/// İmza ve teslim tercihleri.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignaturePreferences {
    /// Rapor imzalanmadan önce zorunlu kontrol listesi gösterilsin mi?
    pub require_precheck: bool,
    /// İmzalı dosya hash'i kaydedilsin mi?
    pub record_signed_hash: bool,
    /// Teslim paketi otomatik hazırlansın mı?
    pub auto_prepare_delivery_package: bool,
}

impl Default for SignaturePreferences {
    fn default() -> Self {
        Self {
            require_precheck: true,
            record_signed_hash: true,
            auto_prepare_delivery_package: true,
        }
    }
}

/// Arşivleme ve yedekleme tercihleri.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePreferences {
    /// Yedek hedef klasörü.
    pub backup_directory: Option<String>,
    /// Otomatik yedekleme açık mı?
    pub auto_backup: bool,
    /// Yedekleme aralığı (saat).
    pub backup_interval_hours: u32,
    /// Saklama süresi (gün); 0 = süresiz.
    pub retention_days: u32,
    /// Yedek şifrelemesi zorunlu mu?
    pub require_encryption: bool,
}

impl Default for ArchivePreferences {
    fn default() -> Self {
        Self {
            backup_directory: None,
            auto_backup: true,
            backup_interval_hours: 24,
            retention_days: 0,
            require_encryption: true,
        }
    }
}

/// Çalışma takvimi.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkCalendar {
    /// Çalışma günleri (0 = Pazartesi .. 6 = Pazar).
    pub work_days: Vec<u8>,
    /// Günlük çalışma başlangıcı (ör. "09:00").
    pub day_start: Option<String>,
    /// Günlük çalışma bitişi.
    pub day_end: Option<String>,
    /// Tatil günleri (ISO tarih).
    pub holidays: Vec<String>,
}

/// Hatırlatma tercihleri.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderPreferences {
    /// Son teslim tarihinden kaç gün önce hatırlatılsın?
    pub lead_days: Vec<u32>,
    /// Masaüstü bildirimi gösterilsin mi?
    pub desktop_notifications: bool,
}

impl Default for ReminderPreferences {
    fn default() -> Self {
        Self {
            lead_days: vec![7, 3, 1],
            desktop_notifications: true,
        }
    }
}

impl ExpertProfile {
    /// Yeni profil oluşturur.
    pub fn new(full_name: impl Into<String>) -> Self {
        Self {
            id: ExpertProfileId::new(),
            full_name: full_name.into(),
            title: None,
            registry_number: None,
            specialty: crate::SPECIALTY_CODE.to_string(),
            sub_specialty: Some(crate::SPECIALTY_NAME.to_string()),
            institution: None,
            contact: ContactInfo::default(),
            default_report_template: None,
            signature_preferences: SignaturePreferences::default(),
            archive_preferences: ArchivePreferences::default(),
            work_calendar: WorkCalendar::default(),
            reminders: ReminderPreferences::default(),
            is_legal_entity: false,
            legal_entity: None,
            created_at: Timestamp::now(),
        }
    }

    /// Profil rapor üretimi için yeterli mi?
    pub fn is_report_ready(&self) -> bool {
        !self.full_name.trim().is_empty() && self.registry_number.is_some()
    }

    /// Rapor imza bloğunda görünecek ad ve unvan.
    pub fn signature_block(&self) -> String {
        match (&self.title, &self.registry_number) {
            (Some(t), Some(r)) => format!("{t} {}\nBilirkişi Sicil No: {r}", self.full_name),
            (Some(t), None) => format!("{t} {}", self.full_name),
            (None, Some(r)) => format!("{}\nBilirkişi Sicil No: {r}", self.full_name),
            (None, None) => self.full_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_defaults_to_56_12() {
        let p = ExpertProfile::new("Ayşe Yılmaz");
        assert_eq!(p.specialty, "56.12");
        assert!(p.signature_preferences.require_precheck);
        assert!(p.archive_preferences.require_encryption);
    }

    #[test]
    fn profile_without_registry_is_not_report_ready() {
        let p = ExpertProfile::new("Ayşe Yılmaz");
        assert!(!p.is_report_ready());
    }

    #[test]
    fn signature_block_includes_registry() {
        let mut p = ExpertProfile::new("Ayşe Yılmaz");
        p.title = Some("Dr.".into());
        p.registry_number = Some("12345".into());
        let b = p.signature_block();
        assert!(b.contains("Dr. Ayşe Yılmaz"));
        assert!(b.contains("12345"));
        assert!(p.is_report_ready());
    }
}
