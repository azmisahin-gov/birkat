//! Uygulama ayarları ve sistem sağlığı (24, 19).

use birkat_core::error::Result;
use serde::{Deserialize, Serialize};

/// Arayüz teması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Theme {
    /// Aydınlık tema.
    Light,
    /// Karanlık tema.
    Dark,
    /// Sistem teması.
    System,
}

impl Theme {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            Theme::Light => "Aydınlık",
            Theme::Dark => "Karanlık",
            Theme::System => "Sistem",
        }
    }
}

/// Uygulama ayarları.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Tema.
    pub theme: Theme,
    /// Otomatik kaydetme aralığı (saniye); 0 = kapalı.
    pub autosave_seconds: u32,
    /// Otomatik kilitleme süresi (dakika); 0 = kapalı.
    pub auto_lock_minutes: u32,
    /// Yüksek çözünürlük (DPI) ölçekleme.
    pub high_dpi: bool,
    /// Arayüz ölçeği (1.0 = normal).
    pub ui_scale: f32,
    /// Varsayılan veri kök dizini.
    pub data_root: Option<String>,
    /// Yedek dizini.
    pub backup_dir: Option<String>,
    /// Otomatik yedekleme açık mı?
    pub auto_backup: bool,
    /// Varsayılan dil.
    pub language: String,
    /// Transkripsiyon dilini otomatik algıla.
    pub auto_detect_language: bool,
    /// Model paketi yolu.
    pub model_path: Option<String>,
    /// GPU kullanılsın mı (destekleniyorsa)?
    pub prefer_gpu: bool,
    /// İnceleme önceliği eşiği (bu önem ve üzeri gösterilir).
    pub review_severity_threshold: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            autosave_seconds: 30,
            auto_lock_minutes: 15,
            high_dpi: true,
            ui_scale: 1.0,
            data_root: None,
            backup_dir: None,
            auto_backup: true,
            language: "tr".to_string(),
            auto_detect_language: true,
            model_path: None,
            prefer_gpu: true,
            review_severity_threshold: "MEDIUM".to_string(),
        }
    }
}

impl AppSettings {
    /// JSON'dan yükler.
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s)
            .map_err(|e| birkat_core::error::CoreError::Data(format!("Ayar çözülemedi: {e}")))
    }

    /// JSON'a yazar.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Ayarları doğrular ve sınırlar.
    pub fn validate(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.ui_scale < 0.5 {
            self.ui_scale = 0.5;
            warnings.push("Arayüz ölçeği en az 0.5 olabilir.".to_string());
        }
        if self.ui_scale > 3.0 {
            self.ui_scale = 3.0;
            warnings.push("Arayüz ölçeği en fazla 3.0 olabilir.".to_string());
        }
        if self.auto_lock_minutes == 0 {
            warnings
                .push("Otomatik kilitleme kapalı; güvenlik için açılması önerilir.".to_string());
        }
        if !self.auto_backup {
            warnings.push("Otomatik yedekleme kapalı; vaka kaybı riski vardır.".to_string());
        }
        warnings
    }
}

/// Sistem sağlığı denetimi (24. Sistem sağlığı ekranı).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Veritabanı erişilebilir ve bütünlük denetimi başarılı mı?
    pub database_ok: bool,
    /// Kasa erişilebilir mi?
    pub vault_ok: bool,
    /// Model paketi mevcut ve doğrulandı mı?
    pub model_ok: bool,
    /// Yedek dizini yazılabilir mi?
    pub backup_dir_ok: bool,
    /// Disk kullanımı (bayt).
    pub disk_used_bytes: Option<u64>,
    /// Disk boş alanı (bayt).
    pub disk_free_bytes: Option<u64>,
    /// CPU çekirdek sayısı.
    pub cpu_cores: usize,
    /// GPU desteği derlemede etkin mi?
    pub gpu_support_compiled: bool,
    /// Uyarılar.
    pub warnings: Vec<String>,
    /// Genel durum.
    pub overall_ok: bool,
}

impl SystemHealth {
    /// Türkçe özet.
    pub fn summary(&self) -> String {
        if self.overall_ok {
            format!(
                "Sistem sağlığı iyi. {} çekirdek, GPU derleme desteği: {}.",
                self.cpu_cores,
                if self.gpu_support_compiled {
                    "var"
                } else {
                    "yok (CPU kullanılır)"
                }
            )
        } else {
            format!("Sistem sağlığında {} uyarı var.", self.warnings.len())
        }
    }

    /// Uyarı ekler ve genel durumu günceller.
    pub fn recompute(&mut self) {
        self.overall_ok =
            self.database_ok && self.vault_ok && self.model_ok && self.warnings.is_empty();
    }
}

/// GPU desteğinin bu derlemede etkin olup olmadığını bildirir.
///
/// Derleme zamanında `BIRKAT_GPU` ortam değişkeni ile işaretlenir; aksi hâlde
/// yalnızca CPU kullanılır.
pub fn gpu_support_compiled() -> bool {
    option_env!("BIRKAT_GPU").is_some()
}

/// Sistem sağlığını denetler.
pub fn check_health(
    data_root: impl AsRef<std::path::Path>,
    model_path: Option<&std::path::Path>,
    backup_dir: Option<&std::path::Path>,
) -> SystemHealth {
    let data_root = data_root.as_ref();
    let mut warnings = Vec::new();

    // Veritabanı
    let db_path = data_root.join("birkat.db");
    let database_ok = match crate::store::CaseStore::open(&db_path) {
        Ok(s) => s.integrity_check().unwrap_or(false),
        Err(e) => {
            warnings.push(format!("Veritabanı açılamadı: {e}"));
            false
        }
    };

    // Kasa
    let vault_path = data_root.join("kasalar");
    let vault_ok = match std::fs::create_dir_all(&vault_path) {
        Ok(()) => true,
        Err(e) => {
            warnings.push(format!("Kasa klasörü oluşturulamadı: {e}"));
            false
        }
    };

    // Model
    let model_ok = match model_path {
        Some(p) if p.exists() => true,
        Some(_) => {
            warnings.push("Model paketi bulunamadı; transkripsiyon çalışmayacaktır.".to_string());
            false
        }
        None => {
            warnings
                .push("Model paketi yolu ayarlanmadı; transkripsiyon için gereklidir.".to_string());
            false
        }
    };

    // Yedek dizini
    let backup_dir_ok = match backup_dir {
        Some(d) => match std::fs::create_dir_all(d) {
            Ok(()) => true,
            Err(e) => {
                warnings.push(format!("Yedek dizini hazırlanamadı: {e}"));
                false
            }
        },
        None => {
            warnings.push("Yedek dizini ayarlanmadı.".to_string());
            false
        }
    };

    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let gpu_support_compiled = gpu_support_compiled();

    let mut health = SystemHealth {
        database_ok,
        vault_ok,
        model_ok,
        backup_dir_ok,
        disk_used_bytes: None,
        disk_free_bytes: None,
        cpu_cores,
        gpu_support_compiled,
        warnings,
        overall_ok: false,
    };
    health.recompute();
    health
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_sensible() {
        let s = AppSettings::default();
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.language, "tr");
        assert!(s.auto_backup);
        assert!(s.auto_detect_language);
    }

    #[test]
    fn settings_json_round_trip() {
        let s = AppSettings::default();
        let json = s.to_json();
        let back = AppSettings::from_json(&json).unwrap();
        assert_eq!(back.language, s.language);
        assert_eq!(back.theme, s.theme);
    }

    #[test]
    fn validation_clamps_ui_scale() {
        let mut s = AppSettings {
            ui_scale: 99.0,
            ..Default::default()
        };
        let w = s.validate();
        assert_eq!(s.ui_scale, 3.0);
        assert!(w.iter().any(|x| x.contains("en fazla")));
    }

    #[test]
    fn validation_warns_on_disabled_safety_features() {
        let mut s = AppSettings {
            auto_lock_minutes: 0,
            auto_backup: false,
            ..Default::default()
        };
        let w = s.validate();
        assert!(w.iter().any(|x| x.contains("Otomatik kilitleme")));
        assert!(w.iter().any(|x| x.contains("Otomatik yedekleme")));
    }

    #[test]
    fn health_check_reports_missing_model() {
        let dir = tempfile::tempdir().unwrap();
        let h = check_health(dir.path(), None, Some(&dir.path().join("yedekler")));
        assert!(!h.model_ok);
        assert!(!h.overall_ok);
        assert!(h.warnings.iter().any(|w| w.contains("Model")));
        assert!(h.database_ok, "veritabanı oluşturulabilmeli");
    }

    #[test]
    fn health_check_with_all_ok() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("model");
        std::fs::create_dir_all(&model).unwrap();
        let h = check_health(dir.path(), Some(&model), Some(&dir.path().join("yedekler")));
        assert!(h.database_ok);
        assert!(h.vault_ok);
        assert!(h.model_ok);
        assert!(h.backup_dir_ok);
        assert!(h.overall_ok, "{:?}", h.warnings);
        assert!(h.cpu_cores >= 1);
    }

    #[test]
    fn health_summary_is_turkish() {
        let dir = tempfile::tempdir().unwrap();
        let h = check_health(dir.path(), None, None);
        assert!(h.summary().contains("Sistem sağlığı"));
    }

    #[test]
    fn theme_labels_are_turkish() {
        assert_eq!(Theme::Light.label(), "Aydınlık");
        assert_eq!(Theme::Dark.label(), "Karanlık");
        assert_eq!(Theme::System.label(), "Sistem");
    }
}
