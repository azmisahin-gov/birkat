//! Görev kapsamı ve soru analizi — 1.2 ve 4.3.
//!
//! 56.12'nin görev alanı dışına taşan talepler otomatik olarak yetkili
//! işler gibi sunulmaz. Uygulama kapsam uyarısı verir ve bilirkişinin
//! görev veren merciden açıklama istemesine yardımcı olacak taslak hazırlar.

use crate::time_util::Timestamp;
use serde::{Deserialize, Serialize};

/// Kapsam dışı talep kategorileri (1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutOfScopeCategory {
    /// Konuşmacının biyometrik kimliğinin tespiti.
    SpeakerBiometricIdentification,
    /// Kaydın montajlı/sahte/değiştirilmiş olduğunun hükme bağlanması.
    AuthenticityRuling,
    /// Sesin tarihsel özgünlüğünün belirlenmesi.
    HistoricalOriginality,
    /// Deepfake tespiti konusunda kesin adli kanaat.
    DeepfakeRuling,
    /// Niyet, duygu veya yalan tespiti.
    IntentEmotionDeception,
    /// Hukuki nitelendirme veya suçluluk değerlendirmesi.
    LegalQualification,
    /// Silinmiş veri kurtarma ve genel dijital delil incelemesi.
    DeletedDataRecovery,
    /// 56.11 kapsamındaki adli ses ve görüntü incelemesi.
    ForensicAudioVideo56_11,
}

impl OutOfScopeCategory {
    /// Türkçe açıklama.
    pub fn explanation(&self) -> &'static str {
        match self {
            OutOfScopeCategory::SpeakerBiometricIdentification => {
                "Konuşmacının biyometrik kimliğinin tespiti 56.12 kapsamı dışındadır; \
                 adli ses incelemesi (56.11) uzmanlığı gerektirebilir."
            }
            OutOfScopeCategory::AuthenticityRuling => {
                "Bir ses kaydının montajlı, sahte veya değiştirilmiş olduğunun hükme bağlanması \
                 56.12 kapsamı dışındadır; adli ses incelemesi (56.11) uzmanlığı gerektirebilir."
            }
            OutOfScopeCategory::HistoricalOriginality => {
                "Sesin tarihsel özgünlüğünün belirlenmesi 56.12 kapsamı dışındadır."
            }
            OutOfScopeCategory::DeepfakeRuling => {
                "Deepfake tespiti konusunda kesin adli kanaat oluşturmak 56.12 kapsamı dışındadır."
            }
            OutOfScopeCategory::IntentEmotionDeception => {
                "Kişinin niyetini, duygusunu veya yalan söyleyip söylemediğini belirlemek \
                 56.12 kapsamı dışındadır."
            }
            OutOfScopeCategory::LegalQualification => {
                "Hukuki nitelendirme veya suçluluk değerlendirmesi bilirkişinin görevi dışındadır."
            }
            OutOfScopeCategory::DeletedDataRecovery => {
                "Silinmiş verilerin kurtarılması ve genel dijital delil incelemesi \
                 56.12 kapsamı dışındadır."
            }
            OutOfScopeCategory::ForensicAudioVideo56_11 => {
                "Adli ses ve görüntü incelemesi 56.11 kapsamındadır; 56.12 işlemi olarak \
                 sunulamaz."
            }
        }
    }

    /// Bu kategori için görev veren mercie yazılacak taslak açıklama talebi.
    pub fn clarification_template(&self, file_number: Option<&str>) -> String {
        let dosya = file_number.unwrap_or("<dosya numarası>");
        format!(
            "{dosya} sayılı dosyada tarafıma verilen görevlendirme kapsamında, \
             görevlendirme metninde aşağıdaki hususa ilişkin bir talep yer almaktadır:\n\n\
             {}\n\n\
             Tarafımın uzmanlık alanı 56.12 (Ses Kayıtlarının Metin Hâline Dönüştürülmesi) \
             ile sınırlıdır. Söz konusu hususun ayrı bir uzmanlık alanı gerektirip \
             gerektirmediği konusunda bilirkişiliğe esas olacak açıklamanın tarafıma \
             bildirilmesini saygılarımla arz ederim.",
            self.explanation()
        )
    }
}

/// Kapsam uyarısı kaydı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeWarningRecord {
    /// Kategori.
    pub category: OutOfScopeCategory,
    /// Uyarıyı tetikleyen metin parçası.
    pub matched_text: String,
    /// Eşleşen kuralın adı.
    pub rule: String,
    /// Kayıt zamanı.
    pub detected_at: Timestamp,
    /// Bilirkişi uyarıyı değerlendirdi mi?
    pub acknowledged: bool,
    /// Bilirkişi notu.
    pub expert_note: Option<String>,
}

/// Kapsam uyarısı üreten kural.
#[derive(Debug, Clone)]
pub struct ScopeRule {
    /// Kategori.
    pub category: OutOfScopeCategory,
    /// Kural adı.
    pub name: &'static str,
    /// Aranacak Türkçe anahtar ifadeler (küçük harfe çevrilerek aranır).
    pub keywords: &'static [&'static str],
}

/// Yerleşik kapsam kuralları.
///
/// Bu liste kapsam dışı taleplerin tespiti için başlangıç kümesidir ve
/// bilirkişi tarafından genişletilebilir.
pub const BUILTIN_SCOPE_RULES: &[ScopeRule] = &[
    ScopeRule {
        category: OutOfScopeCategory::SpeakerBiometricIdentification,
        name: "konusmaci-kimlik",
        keywords: &[
            "konuşmacının kim olduğu",
            "konuşan kişinin kimliği",
            "sesin kime ait olduğu",
            "biyometrik",
            "ses parmak izi",
            "ses kimliği tespiti",
            "konuşmacı tanıma",
        ],
    },
    ScopeRule {
        category: OutOfScopeCategory::AuthenticityRuling,
        name: "montaj-orijinallik",
        keywords: &[
            "montaj",
            "değiştirilmiş",
            "sahte kayıt",
            "orijinal mi",
            "üzerinde oynanmış",
            "kes-yapıştır",
            "tahrifat",
        ],
    },
    ScopeRule {
        category: OutOfScopeCategory::HistoricalOriginality,
        name: "tarihsel-ozgunluk",
        keywords: &[
            "kayıt tarihi tespiti",
            "hangi tarihte kaydedildi",
            "kaydın yaşı",
            "tarihsel özgünlük",
        ],
    },
    ScopeRule {
        category: OutOfScopeCategory::DeepfakeRuling,
        name: "deepfake",
        keywords: &["deepfake", "yapay zekâ ile üretilmiş ses", "sentetik ses"],
    },
    ScopeRule {
        category: OutOfScopeCategory::IntentEmotionDeception,
        name: "niyet-duygu",
        keywords: &[
            "yalan söyleyip",
            "yalan söyledi",
            "niyeti",
            "duygusal durumu",
            "samimiyeti",
            "kızgın mıydı",
            "korkmuş muydu",
        ],
    },
    ScopeRule {
        category: OutOfScopeCategory::LegalQualification,
        name: "hukuki-nit",
        keywords: &[
            "suç teşkil",
            "suçlu mudur",
            "hukuki nitelendirme",
            "suçun unsuru",
            "cezalandırılması",
        ],
    },
    ScopeRule {
        category: OutOfScopeCategory::DeletedDataRecovery,
        name: "silinmis-veri",
        keywords: &[
            "silinmiş kayıt",
            "silinen dosya",
            "kurtarılması",
            "geri getirilmesi",
            "formatlanmış",
        ],
    },
    ScopeRule {
        category: OutOfScopeCategory::ForensicAudioVideo56_11,
        name: "56-11",
        keywords: &[
            "adli ses incelemesi",
            "konuşma çözümlemesi",
            "ses analizi",
            "görüntü incelemesi",
            "adli bilişim",
        ],
    },
];

/// Metni kapsam kurallarına göre tarar.
///
/// Dönen kayıtlar bilirkişiye gösterilir; hiçbir talep otomatik olarak
/// reddedilmez.
pub fn scan_for_scope_warnings(text: &str) -> Vec<ScopeWarningRecord> {
    let lowered = text.to_lowercase();
    let mut out = Vec::new();
    for rule in BUILTIN_SCOPE_RULES {
        for kw in rule.keywords {
            if lowered.contains(&kw.to_lowercase()) {
                out.push(ScopeWarningRecord {
                    category: rule.category,
                    matched_text: (*kw).to_string(),
                    rule: rule.name.to_string(),
                    detected_at: Timestamp::now(),
                    acknowledged: false,
                    expert_note: None,
                });
                break; // aynı kural için tek kayıt yeterli
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_biometric_request() {
        let w = scan_for_scope_warnings(
            "Kayıttaki konuşmacının kim olduğu tespit edilsin ve montaj olup olmadığı belirlensin.",
        );
        assert!(w
            .iter()
            .any(|x| x.category == OutOfScopeCategory::SpeakerBiometricIdentification));
        assert!(w
            .iter()
            .any(|x| x.category == OutOfScopeCategory::AuthenticityRuling));
    }

    #[test]
    fn clean_transcription_request_produces_no_warning() {
        let w =
            scan_for_scope_warnings("Kayıt üzerindeki konuşmalar çözümlenerek yazıya geçirilsin.");
        assert!(w.is_empty());
    }

    #[test]
    fn turkish_case_folding_is_handled() {
        let w = scan_for_scope_warnings("DEEPFAKE olup olmadığı değerlendirilsin.");
        assert!(w
            .iter()
            .any(|x| x.category == OutOfScopeCategory::DeepfakeRuling));
    }

    #[test]
    fn clarification_template_mentions_56_12() {
        let t = OutOfScopeCategory::AuthenticityRuling.clarification_template(Some("2025/1"));
        assert!(t.contains("2025/1"));
        assert!(t.contains("56.12"));
    }

    #[test]
    fn every_category_has_explanation() {
        for c in [
            OutOfScopeCategory::SpeakerBiometricIdentification,
            OutOfScopeCategory::AuthenticityRuling,
            OutOfScopeCategory::HistoricalOriginality,
            OutOfScopeCategory::DeepfakeRuling,
            OutOfScopeCategory::IntentEmotionDeception,
            OutOfScopeCategory::LegalQualification,
            OutOfScopeCategory::DeletedDataRecovery,
            OutOfScopeCategory::ForensicAudioVideo56_11,
        ] {
            assert!(!c.explanation().is_empty());
            assert!(c.clarification_template(None).contains("56.12"));
        }
    }
}
