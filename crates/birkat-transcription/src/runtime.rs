//! Üretim transkripsiyon motoru seçimi.
//!
//! Uygulama, model paketi bulunduğunda **gerçek yerel Whisper/Candle**
//! motorunu kullanır. Model yoksa veya `whisper` özelliği derlemede
//! etkinleştirilmemişse, kullanıcıya açık bir uyarı verilir ve iş akışı
//! çalışmaya devam eder (rapor ve UDF üretimi tamamlanır).
//!
//! ## Neden bu katman var
//!
//! Ürünün ana vaadi yerel ve çevrimdışı transkripsiyondur. Bu nedenle motor
//! seçimi tek bir yerde yapılır ve hangi motorun kullanıldığı **açıkça**
//! bildirilir; sessizce sahte bir motora düşülmez.

use crate::engine::{MockEngine, TranscriptionEngine};
#[allow(unused_imports)]
use birkat_core::error::{CoreError, Result};
use std::path::{Path, PathBuf};

/// Motor seçim sonucu.
pub struct EngineSelection {
    /// Seçilen motor.
    pub engine: Box<dyn TranscriptionEngine>,
    /// Motorun gerçek mi, yedek mi olduğu.
    pub kind: EngineKind,
    /// Kullanıcıya gösterilecek açıklama.
    pub notice: String,
}

/// Seçilen motorun türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// Gerçek yerel Whisper/Candle motoru.
    WhisperCandle,
    /// Yedek motor (test/arayüz akışı). Gerçek transkripsiyon yapmaz.
    Mock,
}

impl EngineKind {
    /// Türkçe etiket.
    pub fn label(&self) -> &'static str {
        match self {
            EngineKind::WhisperCandle => "Whisper large-v3-turbo (yerel, Candle)",
            EngineKind::Mock => "Yedek motor (gerçek transkripsiyon yapmaz)",
        }
    }

    /// Gerçek transkripsiyon yapıyor mu?
    pub fn is_real(&self) -> bool {
        matches!(self, EngineKind::WhisperCandle)
    }
}

/// Model paketi yolu.
#[derive(Debug, Clone)]
pub struct ModelPaths {
    /// Dizin.
    pub root: PathBuf,
    /// `model.safetensors`.
    pub weights: PathBuf,
    /// `config.json`.
    pub config: PathBuf,
    /// `tokenizer.json`.
    pub tokenizer: PathBuf,
    /// `melfilters128.bytes`.
    pub mel_filters: PathBuf,
}

impl ModelPaths {
    /// Model dizininden yolları türetir.
    pub fn from_dir(dir: impl AsRef<Path>) -> Self {
        let root = dir.as_ref().to_path_buf();
        Self {
            weights: root.join("model.safetensors"),
            config: root.join("config.json"),
            tokenizer: root.join("tokenizer.json"),
            mel_filters: root.join("melfilters128.bytes"),
            root,
        }
    }

    /// Tüm gerekli dosyalar mevcut mu?
    pub fn is_complete(&self) -> bool {
        self.weights.is_file()
            && self.config.is_file()
            && self.tokenizer.is_file()
            && self.mel_filters.is_file()
    }

    /// Eksik dosyaların listesi.
    pub fn missing(&self) -> Vec<String> {
        let mut v = Vec::new();
        for (ad, p) in [
            ("model.safetensors", &self.weights),
            ("config.json", &self.config),
            ("tokenizer.json", &self.tokenizer),
            ("melfilters128.bytes", &self.mel_filters),
        ] {
            if !p.is_file() {
                v.push(ad.to_string());
            }
        }
        v
    }
}

/// Model dizinini arar.
///
/// Arama sırası:
/// 1. `BIRKAT_MODEL_DIR` ortam değişkeni (kurulum paketinde ayarlanır).
/// 2. Çalışan ikilinin bulunduğu klasördeki ve üst klasörlerdeki `models/`.
///    Böylece `target/debug/birkat` gibi bir geliştirme ikilisi, proje
///    kökündeki `models/` klasörünü de bulur.
/// 3. Veri kökü altındaki `models/`.
/// 4. Geçerli çalışma dizinindeki `models/`.
pub fn discover_model_dir(data_root: impl AsRef<Path>) -> Option<ModelPaths> {
    let mut adaylar: Vec<PathBuf> = Vec::new();

    if let Ok(d) = std::env::var("BIRKAT_MODEL_DIR") {
        adaylar.push(PathBuf::from(d));
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        // İkilinin bulunduğu klasörden köke doğru en fazla 4 seviye yukarı bak.
        for _ in 0..4 {
            let Some(d) = dir else { break };
            adaylar.push(d.join("models"));
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }

    adaylar.push(data_root.as_ref().join("models"));
    adaylar.push(PathBuf::from("models"));

    adaylar
        .into_iter()
        .map(|a| ModelPaths::from_dir(a))
        .find(|p| p.is_complete())
}

/// Gerçek Whisper motorunu yükler.
#[cfg(feature = "whisper")]
fn load_whisper(paths: &ModelPaths, prefer_gpu: bool) -> Result<Box<dyn TranscriptionEngine>> {
    let engine = crate::whisper_candle::WhisperCandleEngine::load(
        &paths.weights,
        &paths.config,
        &paths.tokenizer,
        Some(&paths.mel_filters),
        "whisper-large-v3-turbo",
        env!("CARGO_PKG_VERSION"),
        prefer_gpu,
    )?;
    Ok(Box::new(engine))
}

/// `whisper` özelliği derlenmemişse gerçek motor yüklenemez.
#[cfg(not(feature = "whisper"))]
fn load_whisper(_paths: &ModelPaths, _prefer_gpu: bool) -> Result<Box<dyn TranscriptionEngine>> {
    Err(CoreError::Validation(
        "Bu derlemede Whisper/Candle desteği etkin değil. Üretim paketini \
         kullanın veya '--features whisper' ile derleyin."
            .to_string(),
    ))
}

/// Motor seçer.
///
/// Model paketi bulunur ve Whisper desteği derlenmişse gerçek motor
/// kullanılır. Aksi hâlde açık bir uyarıyla yedek motora düşülür; **sessiz
/// düşme yoktur.**
pub fn select_engine(data_root: impl AsRef<Path>, prefer_gpu: bool) -> EngineSelection {
    match discover_model_dir(&data_root) {
        Some(paths) => match load_whisper(&paths, prefer_gpu) {
            Ok(engine) => EngineSelection {
                engine,
                kind: EngineKind::WhisperCandle,
                notice: format!("Yerel Whisper modeli yüklendi: {}", paths.root.display()),
            },
            Err(e) => EngineSelection {
                engine: Box::new(MockEngine::default()),
                kind: EngineKind::Mock,
                notice: format!(
                    "UYARI: Whisper modeli yüklenemedi ({e}). \
                     Gerçek transkripsiyon yapılmayacak; rapor ve UDF üretimi \
                     çalışmaya devam eder ancak transkriptler makine çıktısı \
                     değildir."
                ),
            },
        },
        None => EngineSelection {
            engine: Box::new(MockEngine::default()),
            kind: EngineKind::Mock,
            notice: format!(
                "UYARI: Whisper model paketi bulunamadı. Aranan yerler: \
                 BIRKAT_MODEL_DIR, uygulama yanındaki 'models/', \
                 '{}' ve './models'. Gerçek transkripsiyon yapılmayacak.",
                data_root.as_ref().join("models").display()
            ),
        },
    }
}

/// Whisper desteğinin bu derlemede etkin olup olmadığını bildirir.
pub fn whisper_compiled() -> bool {
    cfg!(feature = "whisper")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_paths_report_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = ModelPaths::from_dir(dir.path());
        assert!(!p.is_complete());
        let eksik = p.missing();
        assert_eq!(eksik.len(), 4);
        assert!(eksik.contains(&"model.safetensors".to_string()));
    }

    #[test]
    fn complete_model_dir_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "model.safetensors",
            "config.json",
            "tokenizer.json",
            "melfilters128.bytes",
        ] {
            std::fs::write(dir.path().join(f), b"x").unwrap();
        }
        let p = ModelPaths::from_dir(dir.path());
        assert!(p.is_complete());
        assert!(p.missing().is_empty());
    }

    #[test]
    fn missing_model_falls_back_with_explicit_warning() {
        let dir = tempfile::tempdir().unwrap();
        // Hiçbir model dosyası yok.
        let sel = select_engine(dir.path(), false);
        assert_eq!(sel.kind, EngineKind::Mock);
        assert!(!sel.kind.is_real());
        assert!(sel.notice.contains("UYARI"));
        assert!(sel.notice.contains("bulunamadı"));
    }

    #[test]
    fn engine_kind_labels_are_turkish() {
        assert!(EngineKind::WhisperCandle.label().contains("yerel"));
        assert!(EngineKind::Mock
            .label()
            .contains("gerçek transkripsiyon yapmaz"));
        assert!(EngineKind::WhisperCandle.is_real());
        assert!(!EngineKind::Mock.is_real());
    }

    #[test]
    fn discovery_finds_model_in_data_root() {
        let dir = tempfile::tempdir().unwrap();
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        for f in [
            "model.safetensors",
            "config.json",
            "tokenizer.json",
            "melfilters128.bytes",
        ] {
            std::fs::write(models.join(f), b"x").unwrap();
        }
        let found = discover_model_dir(dir.path()).unwrap();
        assert_eq!(found.root, models);
    }
}
