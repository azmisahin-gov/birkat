//! Zaman yardımcıları.
//!
//! Tüm zaman damgaları UTC olarak saklanır ve RFC 3339 biçiminde
//! serileştirilir. Yerel saat yalnızca kullanıcı arayüzünde gösterilir.

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// UTC zaman damgası sarmalayıcısı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(#[serde(with = "rfc3339")] pub OffsetDateTime);

impl Timestamp {
    /// Şu anki UTC zamanı.
    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    /// Altındaki `OffsetDateTime` değerini döndürür.
    pub fn inner(&self) -> OffsetDateTime {
        self.0
    }

    /// RFC 3339 metnine çevirir.
    pub fn to_rfc3339(&self) -> String {
        self.0
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }

    /// RFC 3339 metninden çözer.
    pub fn parse(value: &str) -> Option<Self> {
        OffsetDateTime::parse(value, &Rfc3339).ok().map(Self)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self(OffsetDateTime::UNIX_EPOCH)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_rfc3339())
    }
}

impl From<OffsetDateTime> for Timestamp {
    fn from(value: OffsetDateTime) -> Self {
        Self(value)
    }
}

/// `OffsetDateTime` için RFC 3339 serileştirme.
mod rfc3339 {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};

    pub fn serialize<S: Serializer>(value: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error> {
        let text = value.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
        s.serialize_str(&text)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<OffsetDateTime, D::Error> {
        let text = String::deserialize(d)?;
        OffsetDateTime::parse(&text, &Rfc3339).map_err(serde::de::Error::custom)
    }
}

/// Bir süreyi insan okuyabilir Türkçe metne çevirir.
///
/// Örnek: `"3 gün 4 saat"`, `"2 saat 5 dakika"`, `"45 dakika"`.
pub fn humanize_duration(d: time::Duration) -> String {
    let total_seconds = d.whole_seconds().max(0);
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;

    let mut parts: Vec<String> = Vec::new();
    if days > 0 {
        parts.push(format!("{days} gün"));
    }
    if hours > 0 {
        parts.push(format!("{hours} saat"));
    }
    if minutes > 0 && days == 0 {
        parts.push(format!("{minutes} dakika"));
    }
    if parts.is_empty() {
        parts.push("1 dakikadan az".to_string());
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_round_trip() {
        let now = Timestamp::now();
        let text = now.to_rfc3339();
        let parsed = Timestamp::parse(&text).expect("ayrıştırılmalı");
        assert_eq!(now, parsed);
    }

    #[test]
    fn humanize_reports_days_and_hours() {
        let d = time::Duration::seconds(3 * 86_400 + 4 * 3_600);
        assert_eq!(humanize_duration(d), "3 gün 4 saat");
    }

    #[test]
    fn humanize_reports_minutes() {
        let d = time::Duration::seconds(45 * 60);
        assert_eq!(humanize_duration(d), "45 dakika");
    }
}
