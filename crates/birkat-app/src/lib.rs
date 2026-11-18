//! # BİRKÂT uygulama servis katmanı
//!
//! Bu kitaplık, çekirdek alan modelini, kalıcı depoyu ve tüm iş modüllerini
//! tek bir vaka merkezli akışta birleştirir (2.1, 21.3).
//!
//! ## Modüller
//!
//! * [`store`] — SQLite tabanlı vaka deposu (şifreli blob'lar için kanca).
//! * [`vault`] — vaka kasası: dosya edinimi, hash, işlem geçmişi.
//! * [`workspace`] — vaka çalışma alanı: tüm modüllerin bağlandığı yer.
//! * [`dashboard`] — ana kontrol paneli verisi (2.1).
//! * [`orchestrator`] — otomatik iş akışı (2.2).
//! * [`settings`] — uygulama ayarları ve sistem sağlığı.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod dashboard;
pub mod orchestrator;
pub mod settings;
pub mod store;
pub mod vault;
pub mod workspace;

pub use dashboard::{build_dashboard, DashboardData};
pub use orchestrator::{run_automatic_workflow, WorkflowOutcome};
pub use settings::{AppSettings, SystemHealth};
pub use store::CaseStore;
pub use vault::{CaseVault, VaultError};
pub use workspace::CaseWorkspace;
