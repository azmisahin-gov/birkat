//! BİRKÂT masaüstü uygulaması giriş noktası.
//!
//! Bu ikili, egui/eframe tabanlı yerel masaüstü arayüzünü başlatır. Uygulama
//! ana işlevlerini internet olmadan tamamlayabilir.

use birkat_desktop::BirkatApp;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    // Veri kök dizini: komut satırı argümanı veya varsayılan.
    let data_root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var("BIRKAT_VERI")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./birkat-veri"))
        });

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("BİRKÂT — 56.12 Bilirkişi Dijital Çalışma Operatörü"),
        ..Default::default()
    };

    eframe::run_native(
        "BİRKÂT",
        native_options,
        Box::new(move |_cc| Ok(Box::new(BirkatApp::new(data_root)))),
    )
}
