//! # BİRKÂT yerel transkripsiyon motoru
//!
//! 9, 10 ve 11. bölümlerdeki işlevleri uygular:
//!
//! * [`engine`] — transkripsiyon motoru soyutlaması ve yerel Whisper/Candle
//!   uygulaması.
//! * [`queue`] — otomatik iş kuyruğu (öncelik, duraklat/devam/iptal,
//!   yeniden deneme, kontrol noktası, kesinti sonrası devam).
//! * [`quality`] — otomatik transkripsiyon kalite kontrolü.
//! * [`conventions`] — transkripsiyon yazım standardı (birebir yaklaşım,
//!   belirsizlik işaretleri, konuşmacı etiketleri).
//! * [`hallucination`] — olası halüsinasyon ve tekrar tespiti.
//! * [`model_package`] — model paketleme, hash ve sürüm doğrulama.
//!
//! ## Dürüstlük sınırları
//!
//! * Model log olasılığı veya güven göstergesi **gerçek doğruluk yüzdesi
//!   gibi sunulmaz** (bkz. [`quality::confidence_note`]).
//! * Kelime düzeyi zaman kodu doğrulanmış yöntemle elde edilmiyorsa
//!   üretilmez (bkz. [`engine::WordTimestamps`]).
//! * Makine transkripti insan tarafından doğrulanmış gösterilmez.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod conventions;
pub mod engine;
pub mod hallucination;
pub mod model_package;
pub mod quality;
pub mod queue;
pub mod runtime;

#[cfg(feature = "whisper")]
pub mod whisper_candle;

pub use engine::{
    EngineCapabilities, TranscriptSegmentDraft, TranscriptionEngine, TranscriptionOutput,
    TranscriptionRequest,
};
pub use model_package::{ModelPackage, ModelVerification};
pub use quality::{run_quality_control, QualityControlInput};
pub use queue::{JobQueue, QueueSnapshot};
pub use runtime::{discover_model_dir, select_engine, EngineKind, EngineSelection, ModelPaths};
