//! Aygıt tanımlama (5.2).
//!
//! Aygıt için üretici, model, seri numarası, kapasite, disk numarası, bölüm
//! bilgileri, dosya sistemi, bağlantı türü, sürücü harfi, bağlanma/ayrılma
//! zamanı ve yazma koruması durumu kaydedilir.
//!
//! **İşletim sistemi tarafından sunulmayan bilgiler tahmin edilmez.**

use birkat_core::evidence::{ConnectionKind, PartitionInfo, PhysicalDevice, WriteProtectionState};
use birkat_core::ids::DeviceId;
use birkat_core::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Aygıt tanımlama sırasında toplanan ham bilgi.
///
/// Platform katmanı bu yapıyı doldurur; bu kitaplık yalnızca iş kuralını
/// uygular. Bilinmeyen alanlar `None` bırakılır ve tahmin edilmez.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawDeviceInfo {
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
    /// Bölümler.
    pub partitions: Vec<PartitionInfo>,
    /// Dosya sistemi.
    pub filesystem: Option<String>,
    /// Bağlantı türü.
    pub connection: ConnectionKind,
    /// Sürücü harfi.
    pub drive_letter: Option<String>,
    /// Aygıt sistem diski mi?
    pub is_system_disk: bool,
    /// Yazma koruması durumu.
    pub write_protection: WriteProtectionState,
}

impl RawDeviceInfo {
    /// Bilinmeyen alanları listeler.
    ///
    /// Bu liste raporda "işletim sistemi tarafından sunulmadı" olarak
    /// gösterilir; hiçbir alan tahmin edilmez.
    pub fn unknown_fields(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.vendor.is_none() {
            v.push("üretici");
        }
        if self.model.is_none() {
            v.push("model");
        }
        if self.serial.is_none() {
            v.push("seri numarası");
        }
        if self.capacity_bytes.is_none() {
            v.push("kapasite");
        }
        if self.disk_number.is_none() {
            v.push("disk numarası");
        }
        if self.filesystem.is_none() {
            v.push("dosya sistemi");
        }
        if self.drive_letter.is_none() {
            v.push("sürücü harfi");
        }
        v
    }

    /// Ham bilgiden fiziksel aygıt kaydı üretir.
    pub fn into_device(self) -> PhysicalDevice {
        PhysicalDevice {
            id: DeviceId::new(),
            vendor: self.vendor,
            model: self.model,
            serial: self.serial,
            capacity_bytes: self.capacity_bytes,
            disk_number: self.disk_number,
            partitions: self.partitions,
            filesystem: self.filesystem,
            connection: self.connection,
            drive_letter: self.drive_letter,
            attached_at: Timestamp::now(),
            detached_at: None,
            write_protection: self.write_protection,
            is_system_disk: self.is_system_disk,
            notes: Vec::new(),
        }
    }
}

/// Bağlantı türünü Türkçe etikete çevirir.
pub fn connection_label(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::Usb => "USB",
        ConnectionKind::SdCard => "SD/microSD kart",
        ConnectionKind::ExternalSata => "Harici SATA/SSD",
        ConnectionKind::InternalSata => "Dahili SATA",
        ConnectionKind::Nvme => "NVMe",
        ConnectionKind::Optical => "Optik sürücü (CD/DVD)",
        ConnectionKind::Network => "Ağ kaynağı",
        ConnectionKind::Unknown => "Bilinmiyor",
    }
}

/// Aygıtın edinim için uygun olup olmadığını değerlendirir.
pub fn assess_device(device: &PhysicalDevice) -> DeviceAssessment {
    let mut warnings = Vec::new();
    let mut blockers = Vec::new();

    if device.is_system_disk {
        blockers.push(
            "Sistem diski edinim kaynağı olarak kullanılamaz.".to_string(),
        );
    }
    if device.capacity_bytes.is_none() {
        warnings.push(
            "Kapasite bilgisi işletim sistemi tarafından sunulmadı.".to_string(),
        );
    }
    if device.serial.is_none() {
        warnings.push(
            "Seri numarası sunulmadı; aygıt kimliği sınırlı kesinliktedir.".to_string(),
        );
    }
    match &device.write_protection {
        WriteProtectionState::Unverified(r) => warnings.push(format!(
            "Yazma koruması durumu doğrulanamadı: {r}. Bu durum 'korumalı' olarak sunulmaz."
        )),
        WriteProtectionState::None => warnings.push(
            "Donanımsal veya yazılımsal yazma koruması algılanmadı.".to_string(),
        ),
        s if s.is_protected() => {}
        _ => {}
    }
    if device.connection == ConnectionKind::Network {
        warnings.push("Ağ kaynağı üzerinden edinim güvenlik açısından sınırlıdır.".to_string());
    }

    DeviceAssessment {
        eligible: blockers.is_empty(),
        warnings,
        blockers,
    }
}

/// Aygıt değerlendirme sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAssessment {
    /// Edinim için uygun mu?
    pub eligible: bool,
    /// Uyarılar.
    pub warnings: Vec<String>,
    /// Engeller.
    pub blockers: Vec<String>,
}

/// Bağlı aygıtların listesini tutar (Windows aygıt olaylarını yansıtır).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceRegistry {
    /// Bilinen aygıtlar.
    pub devices: Vec<PhysicalDevice>,
}

impl DeviceRegistry {
    /// Yeni kayıt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Aygıt ekler.
    pub fn attach(&mut self, device: PhysicalDevice) -> DeviceId {
        let id = device.id.clone();
        self.devices.push(device);
        id
    }

    /// Aygıtı ayırır (ayrılma zamanını kaydeder).
    pub fn detach(&mut self, id: &DeviceId) {
        if let Some(d) = self.devices.iter_mut().find(|d| &d.id == id) {
            d.detached_at = Some(Timestamp::now());
        }
    }

    /// Şu anda bağlı aygıtlar.
    pub fn attached(&self) -> Vec<&PhysicalDevice> {
        self.devices
            .iter()
            .filter(|d| d.detached_at.is_none())
            .collect()
    }

    /// Sistem diskleri hariç, edinime uygun aygıtlar.
    pub fn acquirable(&self) -> Vec<&PhysicalDevice> {
        self.attached()
            .into_iter()
            .filter(|d| assess_device(d).eligible)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usb() -> RawDeviceInfo {
        RawDeviceInfo {
            vendor: Some("SanDisk".into()),
            model: Some("Ultra".into()),
            serial: Some("1234ABCD".into()),
            capacity_bytes: Some(64_000_000_000),
            disk_number: Some(2),
            partitions: vec![PartitionInfo {
                index: 1,
                start_sector: Some(2048),
                sector_count: Some(125_000_000),
                filesystem: Some("exFAT".into()),
                drive_letter: Some("E:".into()),
                label: Some("DELIL".into()),
            }],
            filesystem: Some("exFAT".into()),
            connection: ConnectionKind::Usb,
            drive_letter: Some("E:".into()),
            is_system_disk: false,
            write_protection: WriteProtectionState::None,
        }
    }

    #[test]
    fn unknown_fields_are_not_guessed() {
        let r = RawDeviceInfo {
            connection: ConnectionKind::Usb,
            ..Default::default()
        };
        let u = r.unknown_fields();
        assert!(u.contains(&"üretici"));
        assert!(u.contains(&"seri numarası"));
        assert!(u.contains(&"kapasite"));
    }

    #[test]
    fn full_info_has_no_unknown_fields() {
        assert!(usb().unknown_fields().is_empty());
    }

    #[test]
    fn device_creation_preserves_fields() {
        let d = usb().into_device();
        assert_eq!(d.vendor.as_deref(), Some("SanDisk"));
        assert_eq!(d.connection, ConnectionKind::Usb);
        assert_eq!(d.partitions.len(), 1);
    }

    #[test]
    fn system_disk_is_blocked() {
        let mut r = usb();
        r.is_system_disk = true;
        let d = r.into_device();
        let a = assess_device(&d);
        assert!(!a.eligible);
        assert!(a.blockers.iter().any(|b| b.contains("Sistem diski")));
    }

    #[test]
    fn unverified_protection_produces_warning() {
        let mut r = usb();
        r.write_protection = WriteProtectionState::Unverified("yetki yok".into());
        let d = r.into_device();
        let a = assess_device(&d);
        assert!(a.eligible);
        assert!(a.warnings.iter().any(|w| w.contains("doğrulanamadı")));
    }

    #[test]
    fn registry_tracks_attach_and_detach() {
        let mut reg = DeviceRegistry::new();
        let id = reg.attach(usb().into_device());
        assert_eq!(reg.attached().len(), 1);
        reg.detach(&id);
        assert_eq!(reg.attached().len(), 0);
        assert!(reg.devices[0].detached_at.is_some());
    }

    #[test]
    fn acquirable_excludes_system_disks() {
        let mut reg = DeviceRegistry::new();
        reg.attach(usb().into_device());
        let mut sys = usb();
        sys.is_system_disk = true;
        reg.attach(sys.into_device());
        assert_eq!(reg.acquirable().len(), 1);
    }

    #[test]
    fn connection_labels_are_turkish() {
        assert_eq!(connection_label(ConnectionKind::Usb), "USB");
        assert_eq!(connection_label(ConnectionKind::SdCard), "SD/microSD kart");
        assert_eq!(connection_label(ConnectionKind::Optical), "Optik sürücü (CD/DVD)");
    }
}
