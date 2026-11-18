//! # BİRKÂT çekirdek kitaplığı
//!
//! Bu kitaplık 56.12 Bilirkişi Dijital Çalışma Operatörü'nün alan modelini,
//! içerik durum kurallarını ve kalıcı iş akışı motorunu barındırır.
//!
//! Tasarım ilkeleri:
//! * İş mantığı burada; arayüz ve dosya sistemi ayrı paketlerdedir.
//! * Her modül bağımsız test edilebilir.
//! * Alan modeli sürümlenebilir ve ileri uyumludur (bkz. [`MODEL_VERSION`]).
//!
//! A.2 maddesindeki mesleki sorumluluk sınırı bu kitaplıkta
//! [`ContentStatus`] tipiyle uygulanır: makine çıktısı, incelemeye hazır
//! içerik ve bilirkişi onaylı içerik hiçbir koşulda birbirine karıştırılmaz.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod admin;
pub mod audio;
pub mod case;
pub mod content_status;
pub mod error;
pub mod evidence;
pub mod findings;
pub mod ids;
pub mod profile;
pub mod report;
pub mod scope;
pub mod time_util;
pub mod workflow;

/// Alan modelinin şema sürümü.
///
/// Veri deposu kayıtları bu sürümle damgalanır. Yeni alanlar eklendiğinde
/// sürüm artırılır ve göç (migration) yordamı yazılır; böylece eski vakalar
/// okunmaya devam eder.
pub const MODEL_VERSION: u32 = 1;

/// Uygulamanın sürüm bilgisi.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ürünün çalışma adı.
pub const APP_NAME: &str = "BİRKÂT";

/// Hedef uzmanlık kodu.
pub const SPECIALTY_CODE: &str = "56.12";

/// Uzmanlık alanının resmî adı.
pub const SPECIALTY_NAME: &str = "Ses Kayıtlarının Metin Hâline Dönüştürülmesi";
