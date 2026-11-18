//! Transkripsiyon yazım standardı (10).
//!
//! Uygulamanın varsayılan çıktısı bilirkişilik amacıyla kullanılabilir
//! **birebir transkript taslağı** olmalıdır. Makine metni, Türkçe dilbilgisine
//! göre kendiliğinden yeniden yazılmaz; konuşmacının ifade biçimini bozacak
//! şekilde düzeltilmez.
//!
//! ## Belirsiz konuşma
//!
//! Konuşma kesin olarak anlaşılmıyorsa sistem kelime uydurmaz; belirsizlik
//! işaretleri kullanır. Belirsiz bölümün başlangıç ve bitiş zaman kodu korunur.

use birkat_core::audio::{Annotation, SpeakerId};
use serde::{Deserialize, Serialize};

/// Yazım standardı ayarları.
///
/// İşaretlerin kesin standardı kullanıcı tarafından düzenlenebilir; ancak her
/// dosyada aynı kuralla uygulanır.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conventions {
    /// Anlaşılamayan bölüm işareti.
    pub unintelligible: String,
    /// Belirsiz ifade öneki (ör. `[belirsiz: `).
    pub uncertain_prefix: String,
    /// Belirsiz ifade soneki (ör. `]`).
    pub uncertain_suffix: String,
    /// Üst üste konuşma işareti.
    pub overlapping: String,
    /// Konuşma kesildi işareti.
    pub speech_cut_off: String,
    /// Ses anlaşılmıyor işareti.
    pub inaudible: String,
    /// Konuşmacı etiketi belirsiz işareti.
    pub speaker_uncertain: String,
    /// Dolgu sözcükleri korunsun mu?
    pub keep_fillers: bool,
    /// Tekrarlar korunsun mu?
    pub keep_repetitions: bool,
    /// Yarım bırakılan cümleler korunsun mu?
    pub keep_incomplete_sentences: bool,
    /// Konuşmacı etiketleri geçici mi (ör. "Konuşmacı 1")?
    pub speaker_labels_temporary: bool,
}

impl Default for Conventions {
    fn default() -> Self {
        Self {
            unintelligible: Annotation::Unintelligible.default_marker().to_string(),
            uncertain_prefix: "[belirsiz: ".to_string(),
            uncertain_suffix: "]".to_string(),
            overlapping: Annotation::OverlappingSpeech.default_marker().to_string(),
            speech_cut_off: Annotation::SpeechCutOff.default_marker().to_string(),
            inaudible: Annotation::Inaudible.default_marker().to_string(),
            speaker_uncertain: Annotation::SpeakerUncertain.default_marker().to_string(),
            keep_fillers: true,
            keep_repetitions: true,
            keep_incomplete_sentences: true,
            speaker_labels_temporary: true,
        }
    }
}

impl Conventions {
    /// Belirsiz bir ifade için işaret metni üretir.
    pub fn uncertain(&self, best_guess: &str) -> String {
        format!(
            "{}{best_guess}{}",
            self.uncertain_prefix, self.uncertain_suffix
        )
    }

    /// Birebir transkriptin korunması gereken özelliklerini açıklar.
    pub fn verbatim_note(&self) -> String {
        format!(
            "Bu transkript birebir yaklaşımla üretilmiştir. Dolgu sözcükleri \
             {}{}, tekrarlar {}{}, yarım bırakılan cümleler {}{}. Metin Türkçe \
             dilbilgisine göre kendiliğinden yeniden yazılmamıştır. Modelin \
             kendiliğinden kaldırdığı tekrar veya dolgu ifadeleri varsa ses \
             üzerinden kontrol edilebilir.",
            if self.keep_fillers {
                ""
            } else {
                "korunmamıştır ("
            },
            if self.keep_fillers {
                "korunmuştur"
            } else {
                "ayar kapalı)"
            },
            if self.keep_repetitions {
                ""
            } else {
                "korunmamıştır ("
            },
            if self.keep_repetitions {
                "korunmuştur"
            } else {
                "ayar kapalı)"
            },
            if self.keep_incomplete_sentences {
                ""
            } else {
                "korunmamıştır ("
            },
            if self.keep_incomplete_sentences {
                "korunmuştur"
            } else {
                "ayar kapalı)"
            }
        )
    }

    /// Konuşmacı etiketinin geçici olduğunu açıklar.
    pub fn speaker_note(&self) -> &'static str {
        "Konuşmacı etiketleri (Konuşmacı 1, Konuşmacı 2 ...) gerçek kişi \
         kimliği veya biyometrik tanımlama değildir. Konuşmacı ayrımı otomatik \
         öneri olarak sunulur; bilirkişi doğrulamadan kesin kişi adıyla \
         ilişkilendirilmez."
    }

    /// Segment metnini standarda göre işler.
    ///
    /// Bu işlem metni **yeniden yazmaz**; yalnızca belirsizlik işaretlerini
    /// uygular.
    pub fn apply(&self, text: &str, annotations: &[Annotation]) -> String {
        let mut out = text.to_string();
        for a in annotations {
            let marker = match a {
                Annotation::Unintelligible => &self.unintelligible,
                Annotation::OverlappingSpeech => &self.overlapping,
                Annotation::SpeechCutOff => &self.speech_cut_off,
                Annotation::Inaudible => &self.inaudible,
                Annotation::SpeakerUncertain => &self.speaker_uncertain,
                Annotation::Uncertain => continue, // önek/sonek ile ayrı ele alınır
            };
            if !out.contains(marker.as_str()) {
                out.push(' ');
                out.push_str(marker);
            }
        }
        out
    }
}

/// Bir konuşmacı etiketi ve geçicilik bilgisi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTag {
    /// Geçici kimlik.
    pub id: SpeakerId,
    /// Otomatik öneri mi?
    pub automatic: bool,
    /// Bilirkişi onayladı mı?
    pub confirmed: bool,
}

impl SpeakerTag {
    /// Görüntüleme etiketi.
    pub fn label(&self) -> String {
        self.id.display_label()
    }

    /// Kesin kişi kimliği olarak sunulabilir mi?
    ///
    /// Geçici etiketler asla kesin kişi kimliği sayılmaz.
    pub fn may_claim_identity(&self) -> bool {
        false
    }
}

/// Düzenlenmiş (okunabilir) transkriptin birebir transkriptten farkını
/// kaydeden değişiklik.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditedTranscriptDiff {
    /// Segment indeksi.
    pub segment_index: usize,
    /// Birebir metin.
    pub verbatim: String,
    /// Düzenlenmiş metin.
    pub edited: String,
    /// Bilirkişi onayı.
    pub approved: bool,
}

impl EditedTranscriptDiff {
    /// Düzenlenmiş transkriptin birebir transkriptin yerine geçmediğini
    /// açıklar.
    pub fn note(&self) -> &'static str {
        "Düzenlenmiş transkript ayrı bir sürümdür; birebir transkriptin yerine \
         geçmez. Hangi değişikliklerin yapıldığı kaydedilir ve bilirkişi \
         tarafından onaylanır."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_conventions_match_document_markers() {
        let c = Conventions::default();
        assert_eq!(c.unintelligible, "[anlaşılamayan bölüm]");
        assert_eq!(c.overlapping, "[üst üste konuşma]");
        assert_eq!(c.uncertain_prefix, "[belirsiz: ");
        assert!(c.keep_fillers);
        assert!(c.keep_repetitions);
    }

    #[test]
    fn uncertain_marker_wraps_guess() {
        let c = Conventions::default();
        assert_eq!(c.uncertain("Ahmet"), "[belirsiz: Ahmet]");
    }

    #[test]
    fn apply_appends_markers_without_rewriting_text() {
        let c = Conventions::default();
        let out = c.apply("Görüşme başladı.", &[Annotation::OverlappingSpeech]);
        assert!(out.starts_with("Görüşme başladı."));
        assert!(out.contains("[üst üste konuşma]"));
    }

    #[test]
    fn apply_does_not_duplicate_marker() {
        let c = Conventions::default();
        let text = "X [üst üste konuşma]";
        let out = c.apply(text, &[Annotation::OverlappingSpeech]);
        assert_eq!(out.matches("[üst üste konuşma]").count(), 1);
    }

    #[test]
    fn verbatim_note_explains_approach() {
        let c = Conventions::default();
        let n = c.verbatim_note();
        assert!(n.contains("birebir"));
        assert!(n.contains("kendiliğinden yeniden yazılmamıştır"));
    }

    #[test]
    fn speaker_tags_never_claim_identity() {
        let t = SpeakerTag {
            id: SpeakerId(1),
            automatic: true,
            confirmed: true,
        };
        assert_eq!(t.label(), "Konuşmacı 1");
        assert!(!t.may_claim_identity());
    }

    #[test]
    fn speaker_note_mentions_biometric_limit() {
        let c = Conventions::default();
        assert!(c.speaker_note().contains("biyometrik tanımlama değildir"));
    }

    #[test]
    fn edited_diff_note_explains_separation() {
        let d = EditedTranscriptDiff {
            segment_index: 0,
            verbatim: "ee şey".into(),
            edited: "".into(),
            approved: false,
        };
        assert!(d.note().contains("yerine geçmez"));
    }
}
