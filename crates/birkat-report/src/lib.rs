//! # BİRKÂT rapor motoru
//!
//! Rapor üretimi **tek bir kanonik veri modelinden** yapılır. UDF, PDF ve
//! DOCX için ayrı rapor yazma sistemleri oluşturulmaz; tüm çıktılar aynı
//! onaylanmış [`birkat_core::report::ReportDocument`] verisinden üretilir
//! (14. bölüm).
//!
//! ## Modüller
//!
//! * [`templates`] — rapor şablonları ve bölüm doldurma.
//! * [`builder`] — vaka verisinden kanonik rapor belgesi kurma.
//! * [`to_udf`] — kanonik modelden UDF belgesine köprü (birincil çıktı).
//! * [`pdf`] — PDF yardımcı çıktısı (Türkçe karakter destekli).
//! * [`docx`] — DOCX yardımcı çıktısı (OPC/ZIP).
//! * [`text`] — TXT, CSV, JSON, SRT/VTT yardımcı çıktıları.
//! * [`auxiliary`] — ek rapor ve yardımcı mesleki belgeler (14.4, 14.5).
//! * [`preview`] — UDF önizlemesi (15.7).
//! * [`validate`] — rapor son kontrolü (zorunlu alan denetimi).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod auxiliary;
pub mod builder;
pub mod docx;
pub mod pdf;
pub mod preview;
pub mod templates;
pub mod text;
pub mod to_udf;
pub mod validate;

/// A4 sayfa genişliği (punto).
pub const A4_WIDTH_PT: f64 = 595.28;

/// A4 sayfa yüksekliği (punto).
pub const A4_HEIGHT_PT: f64 = 841.89;

/// Tipik kenar boşlukları çıkarıldıktan sonra kullanılabilir genişlik (punto).
pub const A4_USABLE_WIDTH_PT: f64 = 595.28 - 42.52 - 28.35;

/// Transkripsiyon yazım yöntemi açıklaması (10. bölüm).
///
/// Bu metin rapora olduğu gibi girer; birebir yaklaşımı ve belirsizlik
/// işaretlerini açıklar.
pub fn conventions_note() -> String {
    "Transkript birebir (verbatim) yaklaşımla üretilmiştir. Konuşmacının \
     söylediği kelimeler, tekrarlar, yarım bırakılan cümleler, konuşma \
     kesintileri ve dolgu sözcükleri mümkün olduğunca korunmuştur. Metin, \
     konuşmacının ifade biçimini bozacak şekilde Türkçe dilbilgisine göre \
     kendiliğinden yeniden yazılmamıştır. Kesin olarak anlaşılamayan bölümler \
     [anlaşılamayan bölüm], [belirsiz: ...], [üst üste konuşma], [konuşma \
     kesildi] ve [ses anlaşılmıyor] gibi işaretlerle gösterilmiş ve bu \
     bölümlerin başlangıç-bitiş zaman kodları korunmuştur. Konuşmacı \
     etiketleri (Konuşmacı 1, Konuşmacı 2 ...) geçici ayrımdır; gerçek kişi \
     kimliği veya biyometrik tanımlama değildir."
        .to_string()
}

pub use builder::build_report;
pub use templates::ReportTemplate;
pub use to_udf::{report_to_udf, ReportToUdfOptions};
pub use validate::{final_review, FinalReview};
