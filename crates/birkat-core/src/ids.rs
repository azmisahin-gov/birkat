//! Tanımlayıcı (ID) tipleri.
//!
//! Her varlık kendi yeni-tip kimliğine sahiptir. Bu, farklı varlık
//! kimliklerinin birbirine karıştırılmasını derleme zamanında engeller.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Yeni rastgele kimlik üretir.
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            /// Var olan metinden kimlik oluşturur.
            ///
            /// Bu yordam yalnızca veri deposundan okuma sırasında kullanılır;
            /// yeni kayıtlar [`Self::new`] ile üretilmelidir.
            pub fn from_raw(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Kimliğin metin karşılığı.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Kimlik boş mu?
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl std::str::FromStr for $name {
            type Err = std::convert::Infallible;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                Ok(Self(s.to_string()))
            }
        }
    };
}

define_id!(
    /// Bilirkişi profili kimliği.
    ExpertProfileId
);
define_id!(
    /// Vaka kimliği.
    CaseId
);
define_id!(
    /// Görevlendirme kimliği.
    AssignmentId
);
define_id!(
    /// Görevlendirme sorusu kimliği.
    QuestionId
);
define_id!(
    /// Taraf kimliği.
    PartyId
);
define_id!(
    /// Mahkeme veya merci kimliği.
    CourtId
);
define_id!(
    /// Süre kaydı kimliği.
    DeadlineId
);
define_id!(
    /// Delil kaydı kimliği.
    EvidenceId
);
define_id!(
    /// Fiziksel aygıt kimliği.
    DeviceId
);
define_id!(
    /// Edinim işlemi kimliği.
    AcquisitionId
);
define_id!(
    /// Delil imajı kimliği.
    ImageId
);
define_id!(
    /// Dosya nesnesi kimliği.
    FileId
);
define_id!(
    /// Ses varlığı kimliği.
    AudioId
);
define_id!(
    /// Transkripsiyon işi kimliği.
    JobId
);
define_id!(
    /// Transkript kimliği.
    TranscriptId
);
define_id!(
    /// Bulgu kimliği.
    FindingId
);
define_id!(
    /// Delil referansı kimliği.
    ReferenceId
);
define_id!(
    /// Rapor belgesi kimliği.
    ReportId
);
define_id!(
    /// Ek kimliği.
    AttachmentId
);
define_id!(
    /// İmza kaydı kimliği.
    SignatureId
);
define_id!(
    /// Teslim kaydı kimliği.
    DeliveryId
);
define_id!(
    /// Gider kaydı kimliği.
    ExpenseId
);
define_id!(
    /// Ücret kaydı kimliği.
    FeeId
);
define_id!(
    /// Denetim olayı kimliği.
    AuditEventId
);
define_id!(
    /// Yedek kaydı kimliği.
    BackupId
);
define_id!(
    /// Güncelleme kaydı kimliği.
    UpdateId
);
