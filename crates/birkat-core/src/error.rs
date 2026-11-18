//! Çekirdek hata türleri.

use thiserror::Error;

/// Çekirdek işlemlerde oluşabilecek hatalar.
#[derive(Debug, Error)]
pub enum CoreError {
    /// İstenen varlık bulunamadı.
    #[error("kayıt bulunamadı: {kind} ({id})")]
    NotFound {
        /// Varlık türü.
        kind: &'static str,
        /// Varlık kimliği.
        id: String,
    },

    /// Alan doğrulaması başarısız.
    #[error("doğrulama hatası: {0}")]
    Validation(String),

    /// İzin verilmeyen durum geçişi denendi.
    #[error("geçersiz durum geçişi: {from} -> {to}")]
    InvalidTransition {
        /// Mevcut durum.
        from: String,
        /// Hedeflenen durum.
        to: String,
    },

    /// İçerik durumu kuralı ihlal edildi (bkz. A.2).
    #[error("içerik durumu kuralı ihlali: {0}")]
    ContentStatusRule(String),

    /// Görev kapsamı uyarısı gerektiren durum (bkz. 1.2).
    #[error("görev kapsamı uyarısı: {0}")]
    ScopeWarning(String),

    /// Serileştirme veya çözümleme hatası.
    #[error("veri biçimi hatası: {0}")]
    Data(String),

    /// Dosya sistemi hatası.
    #[error("girdi/çıktı hatası: {0}")]
    Io(#[from] std::io::Error),
}

/// Kitaplık sonuç türü.
pub type Result<T> = std::result::Result<T, CoreError>;
