//! # BİRKÂT masaüstü uygulaması (egui/eframe)
//!
//! 24. maddede tanımlanan yirmi ana çalışma alanı Türkçe arayüzle sunulur.
//! Arayüz tamamen Türkçedir; görsel kimlik ciddi, sade, kurumsal ve
//! meslekidir.
//!
//! ## Tasarım ilkeleri
//!
//! * Tek vaka merkezli çalışma: aktif dosya içindeyken dosyanın tüm
//!   işlevlerine bağlam içinde erişilir.
//! * Otomatik kaydetme, klavye kısayolları, arama, filtreleme,
//!   aydınlık/karanlık tema ve yüksek çözünürlük desteği.
//! * Sahte düğme veya maket ekran yoktur; her ekran gerçek servis katmanına
//!   bağlıdır.
//!
//! ## Modüller
//!
//! * [`app`] — uygulama durumu ve ana döngü.
//! * [`screens`] — yirmi çalışma alanı.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod app;
pub mod screens;

pub use app::BirkatApp;
