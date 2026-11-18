//! Yazma koruması (5.3).
//!
//! ## Sınıflandırma
//!
//! Koruma kaynağı açıkça sınıflandırılır:
//!
//! * Donanımsal yazma engelleyici.
//! * Aygıtın kendi salt okunur anahtarı/özelliği.
//! * Windows yazılımsal salt okunur özelliği.
//! * Koruma yok.
//! * Durum doğrulanamadı.
//!
//! **Yazılımsal koruma donanımsal korumaya eşdeğer gösterilmez.**
//!
//! ## Dürüstlük sınırı
//!
//! İşletim sistemi aygıtı uygulama algılamadan önce bağlayabileceği için,
//! sonradan yapılan koruma doğrulaması ilk bağlantıdan itibaren hiç yazma
//! yapılmadığının kanıtı olarak sunulamaz. Bu nedenle yüksek güvence
//! gerektiren işlemlerde donanımsal yazma engelleyici kullanımı desteklenir
//! (bkz. [`WriteProtectionPlan::hardware_recommended`]).

use birkat_core::evidence::WriteProtectionState;
use birkat_core::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Yazma koruması planı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteProtectionPlan {
    /// Hedef aygıt tanımı.
    pub device_label: String,
    /// Sistem diski mi? (Sistem diski asla korumaya alınmaz.)
    pub is_system_disk: bool,
    /// Mevcut koruma durumu.
    pub current: WriteProtectionState,
    /// Yönetici yetkisi gerekiyor mu?
    pub requires_admin: bool,
    /// Yönetici yetkisi var mı?
    pub has_admin: bool,
    /// Kullanıcı onayı verildi mi?
    pub user_approved: bool,
    /// Planlanan işlem adımları.
    pub steps: Vec<ProtectionStep>,
    /// Uyarılar.
    pub warnings: Vec<String>,
    /// Planın üretildiği zaman.
    pub created_at: Timestamp,
}

/// Koruma işleminin bir adımı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionStep {
    /// Adım sırası.
    pub ordinal: u32,
    /// Adım açıklaması.
    pub description: String,
    /// Tamamlandı mı?
    pub done: bool,
}

impl WriteProtectionPlan {
    /// Yeni plan oluşturur.
    pub fn new(
        device_label: impl Into<String>,
        is_system_disk: bool,
        current: WriteProtectionState,
        has_admin: bool,
    ) -> Self {
        let mut plan = Self {
            device_label: device_label.into(),
            is_system_disk,
            current,
            requires_admin: false,
            has_admin,
            user_approved: false,
            steps: Vec::new(),
            warnings: Vec::new(),
            created_at: Timestamp::now(),
        };
        plan.build_steps();
        plan
    }

    fn build_steps(&mut self) {
        let mut n = 0;
        let mut add = |desc: &str| {
            n += 1;
            ProtectionStep {
                ordinal: n,
                description: desc.to_string(),
                done: false,
            }
        };
        self.steps.push(add("Aygıtın tanımlanması"));
        self.steps
            .push(add("Sistem diski olup olmadığının kontrolü"));
        self.steps
            .push(add("Mevcut koruma durumunun sorgulanması"));
        self.steps.push(add("Gerekli yönetici yetkisinin kontrolü"));
        self.steps.push(add("Kullanıcı onayı"));
        self.steps
            .push(add("Yazılımsal salt okunur korumanın uygulanması"));
        self.steps.push(add("Durumun yeniden sorgulanması"));
        self.steps.push(add("Yazma erişimi kontrolü"));
        self.steps
            .push(add("Sonucun vaka işlem geçmişine eklenmesi"));

        self.requires_admin = !self.current.is_protected() || self.current == WriteProtectionState::None;

        if self.is_system_disk {
            self.warnings.push(
                "Bu bir sistem diskidir; yazma koruması uygulanmaz ve uygulanmamalıdır."
                    .to_string(),
            );
        }
        if !self.has_admin && self.requires_admin {
            self.warnings.push(
                "Yazılımsal koruma için yönetici yetkisi gerekiyor; yetki yok.".to_string(),
            );
        }
        if matches!(self.current, WriteProtectionState::None) {
            self.warnings.push(
                "Aygıtta donanımsal koruma algılanmadı. Yüksek güvence gerektiren \
                 işlemlerde donanımsal yazma engelleyici kullanılması önerilir."
                    .to_string(),
            );
        }
    }

    /// Plan uygulanabilir mi?
    pub fn can_apply(&self) -> bool {
        !self.is_system_disk && (!self.requires_admin || self.has_admin) && self.user_approved
    }

    /// Yüksek güvence için donanımsal engelleyici önerilir mi?
    pub fn hardware_recommended(&self) -> bool {
        !self.current.is_hardware_level()
    }

    /// İşletim sistemi bağlama sırasının sınırını açıklar.
    pub fn connection_order_note(&self) -> &'static str {
        "İşletim sistemi aygıtı uygulama algılamadan önce bağlayabilir. Bu nedenle \
         sonradan yapılan koruma doğrulaması, ilk bağlantıdan itibaren hiç yazma \
         yapılmadığının kanıtı olarak sunulamaz. Yüksek güvence gerektiren \
         işlemlerde donanımsal yazma engelleyici kullanılmalıdır."
    }

    /// Planı uygular ve sonucu döndürür.
    ///
    /// Bu işlev yazılımsal koruma uygulamasını **planlar ve sonucu raporlar**.
    /// Gerçek sistem çağrısı platform katmanında yapılır; bu kitaplık platform
    /// bağımsız kural ve durum mantığını taşır.
    pub fn apply(&mut self, os_call_succeeded: bool) -> WriteProtectionVerdict {
        if !self.can_apply() {
            return WriteProtectionVerdict {
                applied: false,
                resulting_state: self.current.clone(),
                verified: false,
                message: self.blocking_reason(),
            };
        }
        for s in self.steps.iter_mut() {
            s.done = true;
        }
        if !os_call_succeeded {
            return WriteProtectionVerdict {
                applied: false,
                resulting_state: WriteProtectionState::Unverified(
                    "işletim sistemi çağrısı başarısız".to_string(),
                ),
                verified: false,
                message: "Yazılımsal koruma uygulanamadı; durum doğrulanamadı.".to_string(),
            };
        }
        WriteProtectionVerdict {
            applied: true,
            resulting_state: WriteProtectionState::WindowsSoftwareReadOnly,
            verified: true,
            message: "Windows yazılımsal salt okunur koruma uygulandı. Bu koruma \
                      donanımsal korumaya eşdeğer değildir."
                .to_string(),
        }
    }

    fn blocking_reason(&self) -> String {
        if self.is_system_disk {
            return "Sistem diski korumaya alınamaz.".to_string();
        }
        if self.requires_admin && !self.has_admin {
            return "Yönetici yetkisi olmadan yazılımsal koruma uygulanamaz.".to_string();
        }
        if !self.user_approved {
            return "Kullanıcı onayı alınmadan koruma uygulanmaz.".to_string();
        }
        "Plan uygulanabilir değil.".to_string()
    }
}

/// Koruma uygulama sonucu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteProtectionVerdict {
    /// Koruma uygulandı mı?
    pub applied: bool,
    /// Sonuç durumu.
    pub resulting_state: WriteProtectionState,
    /// Durum yeniden sorgulanarak doğrulandı mı?
    pub verified: bool,
    /// Açıklama.
    pub message: String,
}

impl WriteProtectionVerdict {
    /// Sonuç "korumalı" olarak sunulabilir mi?
    ///
    /// Doğrulanamayan koruma "korumalı" sayılmaz (5.3).
    pub fn may_report_as_protected(&self) -> bool {
        self.applied && self.verified && self.resulting_state.is_protected()
    }
}

/// Yazma erişimini denetler.
///
/// Aygıt salt okunur ise yazma denemesi başarısız olmalıdır. Bu işlev
/// denetim sonucunu yorumlar.
pub fn interpret_write_probe(read_only: bool, write_succeeded: bool) -> WriteProtectionVerdict {
    if read_only && !write_succeeded {
        WriteProtectionVerdict {
            applied: true,
            resulting_state: WriteProtectionState::DeviceReadOnlySwitch,
            verified: true,
            message: "Aygıt salt okunur; yazma denemesi reddedildi.".to_string(),
        }
    } else if !read_only && write_succeeded {
        WriteProtectionVerdict {
            applied: false,
            resulting_state: WriteProtectionState::None,
            verified: true,
            message: "Aygıt yazılabilir; koruma yok.".to_string(),
        }
    } else {
        WriteProtectionVerdict {
            applied: false,
            resulting_state: WriteProtectionState::Unverified(
                "salt okunur bayrağı ile yazma denemesi sonucu tutarsız".to_string(),
            ),
            verified: false,
            message: "Koruma durumu doğrulanamadı; 'korumalı' olarak sunulmaz.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_disk_cannot_be_protected() {
        let mut p = WriteProtectionPlan::new(
            "Disk 0",
            true,
            WriteProtectionState::None,
            true,
        );
        p.user_approved = true;
        assert!(!p.can_apply());
        let v = p.apply(true);
        assert!(!v.may_report_as_protected());
    }

    #[test]
    fn requires_admin_and_approval() {
        let mut p =
            WriteProtectionPlan::new("Disk 1", false, WriteProtectionState::None, false);
        p.user_approved = true;
        assert!(!p.can_apply(), "yönetici yetkisi yok");
        let mut p2 =
            WriteProtectionPlan::new("Disk 1", false, WriteProtectionState::None, true);
        assert!(!p2.can_apply(), "onay yok");
        p2.user_approved = true;
        assert!(p2.can_apply());
    }

    #[test]
    fn software_protection_is_not_hardware_level() {
        let mut p =
            WriteProtectionPlan::new("Disk 1", false, WriteProtectionState::None, true);
        p.user_approved = true;
        let v = p.apply(true);
        assert!(v.applied);
        assert!(v.may_report_as_protected());
        assert_eq!(v.resulting_state, WriteProtectionState::WindowsSoftwareReadOnly);
        assert!(!v.resulting_state.is_hardware_level());
    }

    #[test]
    fn failed_os_call_yields_unverified() {
        let mut p =
            WriteProtectionPlan::new("Disk 1", false, WriteProtectionState::None, true);
        p.user_approved = true;
        let v = p.apply(false);
        assert!(!v.applied);
        assert!(!v.verified);
        assert!(!v.may_report_as_protected());
        assert!(matches!(v.resulting_state, WriteProtectionState::Unverified(_)));
    }

    #[test]
    fn hardware_blocker_needs_no_software_step() {
        let p = WriteProtectionPlan::new(
            "Disk 1",
            false,
            WriteProtectionState::HardwareBlocker,
            false,
        );
        assert!(!p.hardware_recommended());
    }

    #[test]
    fn no_protection_recommends_hardware() {
        let p =
            WriteProtectionPlan::new("Disk 1", false, WriteProtectionState::None, true);
        assert!(p.hardware_recommended());
        assert!(p
            .warnings
            .iter()
            .any(|w| w.contains("donanımsal yazma engelleyici")));
    }

    #[test]
    fn connection_order_note_explains_limit() {
        let p = WriteProtectionPlan::new("D", false, WriteProtectionState::None, true);
        let n = p.connection_order_note();
        assert!(n.contains("kanıtı olarak sunulamaz"));
    }

    #[test]
    fn write_probe_interpretation() {
        let v = interpret_write_probe(true, false);
        assert!(v.may_report_as_protected());
        assert_eq!(v.resulting_state, WriteProtectionState::DeviceReadOnlySwitch);

        let v2 = interpret_write_probe(false, true);
        assert!(!v2.may_report_as_protected());
        assert_eq!(v2.resulting_state, WriteProtectionState::None);

        let v3 = interpret_write_probe(true, true);
        assert!(!v3.may_report_as_protected());
        assert!(!v3.verified);
    }

    #[test]
    fn plan_has_all_nine_steps() {
        let p = WriteProtectionPlan::new("D", false, WriteProtectionState::None, true);
        assert_eq!(p.steps.len(), 9);
        assert_eq!(p.steps[0].description, "Aygıtın tanımlanması");
    }
}
