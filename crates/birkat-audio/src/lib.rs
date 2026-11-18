//! # BİRKÂT ses envanteri, çözümleme ve inceleme çekirdeği
//!
//! 7 ve 8. bölümlerdeki ses işlevlerini uygular:
//!
//! * [`probe`] — teknik envanter (kapsayıcı, codec, süre, örnekleme, kanal...).
//! * [`decode`] — Symphonia ile ses çözümleme ve PCM üretimi.
//! * [`waveform`] — dalga biçimi (tepe noktaları).
//! * [`spectrogram`] — spektrogram (STFT).
//! * [`playback`] — oynatma denetimi (hız, döngü, kanal seçimi, bölüm).
//! * [`inventory`] — ses dosyası tespiti, envanter, yinelenen içerik.
//! * [`processing`] — türetilmiş ses (gürültü azaltma, normalizasyon) ve
//!   orijinalden ayrı tutma.
//!
//! ## Dürüstlük sınırları
//!
//! * Kaynak dosyada olmayan metadata üretilmez.
//! * Desteklenmeyen codec sessizce atlanmaz; açık hata nedeni gösterilir.
//! * Dalga biçimi veya spektrogram tek başına montaj/orijinallik sonucu
//!   olarak gösterilmez (bkz. [`waveform::interpretation_note`]).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod decode;
pub mod inventory;
pub mod playback;
pub mod probe;
pub mod processing;
pub mod spectrogram;
pub mod waveform;

pub use decode::{decode_to_mono, decode_to_pcm, PcmBuffer};
pub use inventory::{build_inventory, AudioInventoryEntry, InventoryResult};
pub use playback::{PlaybackController, PlaybackState};
pub use probe::{probe_file, ProbeResult};
pub use waveform::WaveformPeaks;
