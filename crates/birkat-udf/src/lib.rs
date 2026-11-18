//! # BİRKÂT UDF (UYAP Doküman Formatı) okuyucu, yazıcı ve doğrulayıcı
//!
//! UDF, 15. bölüm uyarınca uygulamanın **birincil ve zorunlu** rapor
//! çıktısıdır. Opsiyonel bir dışa aktarma biçimi değildir.
//!
//! ## Format
//!
//! UDF dosyası Deflate sıkıştırmalı bir ZIP arşividir. Arşiv içinde tek bir
//! dosya bulunur: `content.xml`.
//!
//! `content.xml`, UTF-8 kodlamalı bir XML'dir ve şu kök elemana sahiptir:
//!
//! ```xml
//! <template format_id="1.8">
//!   <content><![CDATA[ ... tüm düz metin ... ]]></content>
//!   <properties><pageFormat ... /></properties>
//!   <elements resolver="hvl-default"> ... </elements>
//!   <styles> ... </styles>
//! </template>
//! ```
//!
//! **Kritik kural:** Belgenin tüm düz metni tek bir CDATA havuzunda tutulur
//! (önce üstbilgi, sonra gövde, en son altbilgi). `<elements>` içindeki
//! elemanlar bu havuzdaki metni `startOffset` ve `length` ile referans alır.
//! Offset ve length **karakter (rune) sayısıdır**, bayt değil. Türkçe
//! karakterler ve emoji dahil her karakter 1 sayılır.
//!
//! ## Modüller
//!
//! * [`model`] — belge modeli (bölümler, bloklar, paragraflar, tablolar).
//! * [`writer`] — modelden UDF üretimi (metin havuzu + offset hesabı).
//! * [`reader`] — UDF okuma (metin havuzu + offset çözümleme).
//! * [`validator`] — üretilen UDF'nin yapısal doğrulaması.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod model;
pub mod reader;
pub mod validator;
pub mod writer;

pub use model::*;
pub use reader::{read_udf, read_udf_from_bytes, UdfReadResult};
pub use validator::{validate_udf, UdfValidation};
pub use writer::{write_udf, write_udf_to_bytes, UdfWriter};

/// UDF biçim sürümü (`format_id`).
pub const FORMAT_ID: &str = "1.8";

/// Arşiv içindeki içerik dosyasının adı.
pub const CONTENT_XML: &str = "content.xml";

/// Varsayılan yazı tipi.
pub const DEFAULT_FONT: &str = "Times New Roman";

/// Varsayılan punto.
pub const DEFAULT_FONT_SIZE: f64 = 11.0;

/// Varsayılan metin rengi (siyah, işaretli ARGB).
pub const DEFAULT_FOREGROUND: i32 = -16777216;

/// Resim yer tutucusu (Object Replacement Character).
pub const IMAGE_PLACEHOLDER: char = '\u{FFFC}';

/// Sekme karakteri.
pub const TAB_CHAR: char = '\t';

/// Paragraf ayırıcı.
pub const PARAGRAPH_SEPARATOR: char = '\n';

/// Boş paragraf işaretçisi (sıfır genişlikli boşluk).
pub const EMPTY_PARAGRAPH_PLACEHOLDER: char = '\u{200B}';

/// A4 sayfa genişliği (punto).
pub const A4_WIDTH_PT: f64 = 595.28;

/// A4 sayfa yüksekliği (punto).
pub const A4_HEIGHT_PT: f64 = 841.89;

/// Türkçe karakterleri kaybetmeden metnin karakter (rune) sayısını verir.
///
/// Offset hesabı bayt değil karakter üzerinden yapılmalıdır.
pub fn char_len(text: &str) -> usize {
    text.chars().count()
}
