//! # BİRKÂT delil edinimi ve doğrulama
//!
//! 5 ve 6. bölümlerdeki delil yönetimi işlevlerini uygular:
//!
//! * [`hash`] — SHA-256 (zorunlu), SHA-1 ve MD5 (eski imaj uyumluluğu).
//! * [`detect`] — gerçek dosya imzasına dayalı tür tespiti.
//! * [`archive`] — güvenli arşiv açma (zip bomb, yol kaçışı denetimi).
//! * [`acquisition`] — dosya bazlı edinim ve bütünlük doğrulaması.
//! * [`write_protection`] — yazma koruması sorgulama ve uygulama (5.3).
//! * [`device`] — aygıt tanımlama (5.2).
//! * [`ewf`] — E01/L01 imaj okuma/üretme (5.5).
//! * [`manifest`] — edinim manifestosu.
//!
//! ## Dürüstlük sınırları
//!
//! * Dosya bazlı kopya, fiziksel imaj veya L01 imajı olarak
//!   adlandırılmaz (bkz. [`acquisition::AcquisitionResult::label`]).
//! * Yazılımsal yazma koruması donanımsal korumaya eşdeğer gösterilmez.
//! * Bir imajın hash doğrulamasının başarılı olması, kaynak aygıtın
//!   tarihsel olarak orijinal olduğunu kanıtlamaz.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod acquisition;
pub mod archive;
pub mod detect;
pub mod device;
pub mod ewf;
pub mod hash;
pub mod manifest;
pub mod write_protection;

pub use acquisition::{acquire_path, AcquisitionOptions, AcquisitionResult};
pub use detect::{detect_kind, detect_kind_from_bytes};
pub use ewf::{EwfImage, EwfReader, EwfWriter};
pub use hash::{hash_bytes, hash_file, HashDigest};
pub use manifest::{AcquisitionManifest, ManifestEntry};
pub use write_protection::{WriteProtectionPlan, WriteProtectionVerdict};
