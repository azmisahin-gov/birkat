# Gereksinim Matrisi

Bu belge, şartnamedeki her gereksinimi modül, uygulama durumu, ilgili test ve
bilinen sınırlama ile eşler (şartname 29. madde).

Durum kodları:

- **UYGULANDI** — Gerçek kod yazıldı, test edildi ve çalışıyor.
- **UYGULANDI (platform)** — İş mantığı ve testleri tamam; gerçek işletim
  sistemi/donanım çağrısı platform katmanında yapılır ve Windows'ta
  doğrulanmalıdır.
- **SINIRLI** — Bilinen ve belgelenmiş sınırı vardır.

---

## 1. Ürün modülleri (şartname 1.1)

| Kimlik | Gereksinim | Modül | Durum | Test | Sınırlama |
|---|---|---|---|---|---|
| R1.1.1 | Bilirkişi profili ve çalışma yönetimi | `birkat-core::profile`, `birkat-app` | UYGULANDI | `profile::tests::*`, `store::tests::save_and_list_profiles` | — |
| R1.1.2 | Görevlendirme belgesi ve dosya kabulü | `birkat-evidence::detect`, `birkat-app::workspace` | UYGULANDI | `detect::tests::*` | — |
| R1.1.3 | Belge okuma ve OCR | `birkat-evidence::detect`, `birkat-udf::reader` | SINIRLI | `detect::tests::detects_pdf`, `reader::tests::*` | Çevrimdışı OCR motoru paketlenmedi; metin tabanlı PDF/UDF/DOCX okunur, taranmış PDF için OCR kancası hazırdır |
| R1.1.4 | Görev kapsamı ve soru analizi | `birkat-core::scope`, `birkat-core::case` | UYGULANDI | `scope::tests::*`, `case::tests::*` | — |
| R1.1.5 | Materyal teslimi ve delil envanteri | `birkat-app::vault`, `birkat-core::evidence` | UYGULANDI | `vault::tests::*` | — |
| R1.1.6 | USB, SD ve harici aygıt yönetimi | `birkat-evidence::device` | UYGULANDI (platform) | `device::tests::*` | Windows aygıt olayları platform katmanında |
| R1.1.7 | Yazma koruma ve edinim güvenliği | `birkat-evidence::write_protection` | UYGULANDI (platform) | `write_protection::tests::*` | Salt okunur uygulama Windows API'siyle |
| R1.1.8 | Dosya bazlı delil edinimi | `birkat-evidence::acquisition` | UYGULANDI | `acquisition::tests::*` | — |
| R1.1.9 | E01/L01/RAW imaj okuma, doğrulama, üretim | `birkat-evidence::ewf` | UYGULANDI | `ewf::tests::*` (gerçek E01 üretimi ve geri okuma) | — |
| R1.1.10 | Hash ve işlem geçmişi yönetimi | `birkat-evidence::hash`, `birkat-audit` | UYGULANDI | `hash::tests::*`, `audit::tests::*` | — |
| R1.1.11 | Ses dosyası tespiti ve teknik envanter | `birkat-audio::inventory`, `birkat-audio::probe` | UYGULANDI | `inventory::tests::*`, `probe::tests::*` | — |
| R1.1.12 | Ses oynatma ve inceleme | `birkat-audio::playback`, `waveform`, `spectrogram` | UYGULANDI | `playback::tests::*`, `waveform::tests::*`, `spectrogram::tests::*` | Gerçek ses çıkışı platform katmanında |
| R1.1.13 | Yerel transkripsiyon motoru | `birkat-transcription::whisper_candle` | UYGULANDI | `whisper_candle::tests::*` (whisper özelliği) | Model dosyası pakette gelir |
| R1.1.14 | Otomatik transkripsiyon kalite kontrolü | `birkat-transcription::quality` | UYGULANDI | `quality::tests::*` (16 kontrol türü) | — |
| R1.1.15 | Ses-transkript eşzamanlı inceleme | `birkat-audio::playback`, `birkat-desktop::screens::audio_review` | UYGULANDI | `playback::tests::*` | — |
| R1.1.16 | Transkript düzenleme, düzeltme, sürümleme | `birkat-core::audio` | UYGULANDI | `audio::tests::version_progression_is_monotonic` | — |
| R1.1.17 | Görevlendirme soruları ve dayanak ilişkileri | `birkat-core::case`, `birkat-core::findings` | UYGULANDI | `case::tests::*`, `findings::tests::*` | — |
| R1.1.18 | Teknik bulgular ve mesleki notlar | `birkat-core::findings` | UYGULANDI | `findings::tests::*` | — |
| R1.1.19 | UDF merkezli rapor motoru | `birkat-report::builder`, `templates` | UYGULANDI | `builder::tests::*`, `templates::tests::*` | — |
| R1.1.20 | UDF okuyucu, üretici ve doğrulayıcı | `birkat-udf` | UYGULANDI | 52 test | — |
| R1.1.21 | DOCX/PDF ve yardımcı belge çıktıları | `birkat-report::docx`, `pdf`, `text` | UYGULANDI | `docx::tests::*`, `pdf::tests::*`, `text::tests::*` | — |
| R1.1.22 | Ek rapor ve mesleki yazışma belgeleri | `birkat-report::auxiliary` | UYGULANDI | `auxiliary::tests::*` (10 belge türü) | — |
| R1.1.23 | Elektronik imza ve UYAP teslim hazırlığı | `birkat-report::auxiliary`, `birkat-desktop::screens::signature` | UYGULANDI | `auxiliary::tests::delivery_checklist_has_default_items` | Gerçek imza resmî altyapıyla |
| R1.1.24 | Vaka arşivi ve şifreli yedek | `birkat-backup`, `birkat-app::workspace` | UYGULANDI | `backup::tests::*` | — |
| R1.1.25 | Süre, takvim, ücret ve masraf takibi | `birkat-core::admin`, `birkat-core::case` | UYGULANDI | `admin::tests::*` | — |
| R1.1.26 | Çoklu bilirkişi/heyet çalışması | `birkat-core::admin`, `birkat-report::validate` | UYGULANDI | `admin::tests::merge_*`, `validate::tests::panel_*` | — |
| R1.1.27 | Otomatik güncelleme | `birkat-updater` | UYGULANDI | `updater::tests::*` | İmza doğrulaması üretimde etkinleştirilir |
| R1.1.28 | Güvenlik ve sistem yönetimi | `birkat-crypto`, `birkat-app::settings` | UYGULANDI | `crypto::tests::*`, `settings::tests::*` | — |

---

## 2. Uzmanlık kapsamı (şartname 1.2)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R1.2.1 | Kapsam dışı talepler için uyarı | `birkat-core::scope` | UYGULANDI | `scope::tests::detects_biometric_request` |
| R1.2.2 | Açıklama isteme taslağı | `birkat-core::scope` | UYGULANDI | `scope::tests::clarification_template_mentions_56_12` |
| R1.2.3 | 8 kapsam dışı kategori | `birkat-core::scope` | UYGULANDI | `scope::tests::every_category_has_explanation` |

---

## 3. Kullanıcı deneyimi ve otomasyon (şartname 2)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R2.1 | Tek vaka merkezli çalışma | `birkat-app::workspace`, `birkat-desktop` | UYGULANDI | `workspace::tests::*`, `desktop::tests::creating_case_opens_workspace` |
| R2.2 | Tek başlangıç, 16 adımlı otomatik iş akışı | `birkat-app::orchestrator` | UYGULANDI | `orchestrator::tests::workflow_discovers_and_transcribes_audio` |
| R2.3 | İstisna odaklı çalışma | `birkat-app::orchestrator`, `dashboard` | UYGULANDI | `dashboard::tests::unanswered_questions_appear_as_exceptions` |
| R2.4 | Gereksiz onay istememe | `birkat-app::orchestrator` | UYGULANDI | Otomatik akış testleri |

---

## 4. Görevlendirme ve dosya yönetimi (şartname 4)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R4.1 | 15+ belge türü kabulü, imza denetimi | `birkat-evidence::detect` | UYGULANDI | `detect::tests::*` |
| R4.1b | Arşiv bombası ve yol kaçışı denetimi | `birkat-evidence::archive` | UYGULANDI | `archive::tests::*` |
| R4.2 | Görevlendirme verisi çıkarımı + kaynak referansı | `birkat-core::case::SourceRef` | UYGULANDI | `case::tests::*` |
| R4.3 | Soru kayıtları ve cevaplanma takibi | `birkat-core::case::AssignmentQuestion` | UYGULANDI | `case::tests::unanswered_question_produces_warning` |
| R4.4 | Süre takibi | `birkat-core::case::Deadline`, `birkat-core::admin` | UYGULANDI | `admin::tests::tariff_*` |

---

## 5. Materyal kabulü ve delil yönetimi (şartname 5)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R5.1 | 10+ kaynak türü | `birkat-core::evidence::EvidenceSource` | UYGULANDI | `evidence::tests::*` |
| R5.2 | Aygıt tanımlama, tahmin yok | `birkat-evidence::device` | UYGULANDI | `device::tests::unknown_fields_are_not_guessed` |
| R5.3 | Yazma koruması, 9 adım, dürüst sınıflandırma | `birkat-evidence::write_protection` | UYGULANDI | `write_protection::tests::*` (10 test) |
| R5.4 | Dosya bazlı edinim + hash doğrulama | `birkat-evidence::acquisition` | UYGULANDI | `acquisition::tests::acquires_single_file_and_verifies` |
| R5.4b | "İmaj" olarak adlandırmama | `birkat-evidence::acquisition::label` | UYGULANDI | `acquisition::tests::label_does_not_claim_image` |
| R5.5 | E01/L01/Ex01/Lx01 okuma, E01/Ex01 yazma, RAW | `birkat-evidence::ewf` | UYGULANDI | `ewf::tests::physical_ewf_write_produces_valid_image` |
| R5.5b | Hash doğrulamasının sınırı belgelenir | `birkat-evidence::ewf::limitation_note` | UYGULANDI | `ewf::tests::verification_limitation_note_is_honest` |
| R5.6 | Fiziksel teslim kayıtları (14 alan) | `birkat-core::evidence::CustodyRecord` | UYGULANDI | `auxiliary::tests::handover_form_includes_custody_fields` |

---

## 6. Delil kasası, hash ve denetim (şartname 6)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R6.1 | SHA-256 zorunlu, SHA-1/MD5 uyumluluk | `birkat-evidence::hash` | UYGULANDI | `hash::tests::sha256_of_hello_is_known_value` |
| R6.2 | 18 işlem türü kaydı + hash zinciri | `birkat-audit` | UYGULANDI | `audit::tests::tampering_with_description_is_detected` |
| R6.2b | Bağımsız zaman damgası olmadığı belgelenir | `birkat-audit::ChainVerification` | UYGULANDI | `audit::tests::verification_is_not_an_independent_timestamp` |

---

## 7-8. Ses tespiti ve inceleme (şartname 7-8)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R7.1 | 16+ kapsayıcı/codec | `birkat-audio::probe::supported_format_matrix` | UYGULANDI | `probe::tests::format_matrix_covers_required_containers` |
| R7.2 | 19 teknik metadata alanı, tahmin yok | `birkat-core::audio::AudioMetadata` | UYGULANDI | `audio::tests::unknown_metadata_fields_are_reported` |
| R7.3 | Yinelenen dosya: işaretle ama sil | `birkat-audio::inventory` | UYGULANDI | `inventory::tests::duplicate_files_are_flagged_but_kept` |
| R8 | 15 oynatma/inceleme işlevi | `birkat-audio::playback`, `waveform`, `spectrogram` | UYGULANDI | `playback::tests::*` (12 test) |
| R8b | Türetilmiş ses ayrı tutulur | `birkat-audio::processing` | UYGULANDI | `processing::tests::derived_label_marks_not_original` |
| R8c | Dalga/spektrogram yorum sınırı | `birkat-audio::waveform::interpretation_note` | UYGULANDI | `waveform::tests::interpretation_note_mentions_56_11` |

---

## 9-11. Transkripsiyon (şartname 9-11)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R9.1 | Whisper large-v3-turbo / Candle, yerel | `birkat-transcription::whisper_candle` | UYGULANDI | `whisper_candle::tests::*` |
| R9.2 | CPU zorunlu, GPU fallback | `whisper_candle::select_device` | UYGULANDI | `whisper_candle::tests::device_selection_falls_back_to_cpu` |
| R9.2b | 9 işlem bilgisi kaydı | `birkat-core::audio::InferenceParams` | UYGULANDI | `engine::tests::capabilities_are_described` |
| R9.3 | Model pakette, hash doğrulamalı, çevrimdışı | `birkat-transcription::model_package` | UYGULANDI | `model_package::tests::*` (7 test) |
| R9.4 | Kuyruk: öncelik, duraklat, devam, iptal, retry, checkpoint | `birkat-transcription::queue` | UYGULANDI | `queue::tests::*` (12 test) |
| R9.4b | Mükerrer işlem engeli | `queue::enqueue` | UYGULANDI | `queue::tests::completed_task_is_not_duplicated` |
| R9.4c | ETA yalnızca ölçüme dayalı | `queue::eta_secs` | UYGULANDI | `queue::tests::eta_requires_measurement` |
| R9.5 | Türkçe önceliği, otomatik dil tespiti | `birkat-core::audio::LanguageAssignment` | UYGULANDI | `engine::tests::capabilities_are_described` |
| R10.1 | Birebir yaklaşım | `birkat-transcription::conventions` | UYGULANDI | `conventions::tests::verbatim_note_explains_approach` |
| R10.2 | 6 belirsizlik işareti, zaman kodu korunur | `birkat-core::audio::Annotation` | UYGULANDI | `audio::tests::annotations_have_default_markers` |
| R10.3 | Geçici konuşmacı etiketleri | `birkat-core::audio::SpeakerLabel` | UYGULANDI | `conventions::tests::speaker_tags_never_claim_identity` |
| R10.4 | Düzenlenmiş transkript ayrı sürüm | `conventions::EditedTranscriptDiff` | UYGULANDI | `conventions::tests::edited_diff_note_explains_separation` |
| R11 | 16 kalite kontrol türü | `birkat-transcription::quality` | UYGULANDI | `quality::tests::*` (14 test) |
| R11b | Güven göstergesi doğruluk yüzdesi değil | `quality::confidence_note` | UYGULANDI | `quality::tests::confidence_note_is_honest` |
| R11c | Halüsinasyon tespiti | `birkat-transcription::hallucination` | UYGULANDI | `hallucination::tests::*` (9 test) |

---

## 12-13. İnceleme ve dayanaklar (şartname 12-13)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R12.1 | 14 düzenleme işlevi | `birkat-audio::playback`, `birkat-desktop::screens::audio_review` | UYGULANDI | `playback::tests::*` |
| R12.2 | Sahte kelime zamanı üretmeme | `birkat-transcription::engine::WordTimestamps` | UYGULANDI | `engine::tests::capabilities_are_described` |
| R12.3 | 5 aşamalı sürüm koruması | `birkat-core::audio::TranscriptVersion` | UYGULANDI | `audio::tests::machine_raw_transcript_is_immutable` |
| R13 | Soru-dayanak ilişkileri, dayanak açma | `birkat-core::findings` | UYGULANDI | `findings::tests::answer_needs_references_and_approval_to_be_opinion` |

---

## 14. Rapor motoru (şartname 14)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R14.1 | 15 zorunlu alan | `birkat-report::validate::required_fields` | UYGULANDI | `validate::tests::required_fields_list_covers_mandatory_sections` |
| R14.2 | 27 standart bölüm, zorunlu kaldırılamaz | `birkat-core::report::SectionId` | UYGULANDI | `templates::tests::mandatory_sections_cannot_be_hidden_by_template` |
| R14.3 | Otomatik doldurma | `birkat-report::builder` | UYGULANDI | `builder::tests::builds_report_with_all_sections` |
| R14.4 | Ek rapor önceki raporu değiştirmez | `birkat-report::auxiliary::build_supplementary_report` | UYGULANDI | `auxiliary::tests::supplementary_report_does_not_replace_previous` |
| R14.5 | 12 yardımcı mesleki belge | `birkat-report::auxiliary` | UYGULANDI | `auxiliary::tests::all_auxiliary_kinds_have_titles_and_file_names` |

---

## 15. UDF (şartname 15)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R15.1 | UDF yazıcı: 17 öğe türü | `birkat-udf::writer` | UYGULANDI | `writer::tests::*` (14 test) |
| R15.1b | Karakter (rune) ofset hesabı | `writer::UdfWriter` | UYGULANDI | `writer::tests::turkish_characters_count_as_one` |
| R15.2 | UDF okuyucu | `birkat-udf::reader` | UYGULANDI | `reader::tests::*` (17 test) |
| R15.3 | 9 yönlü doğrulama | `birkat-udf::validator` | UYGULANDI | `validator::tests::*` (10 test) |
| R15.4 | Gerçek UYAP Editör testi | — | SINIRLI | — | Editör bu ortamda yok; test seti ve prosedür `docs/udf-uyumluluk.md` içinde |
| R15.5 | Sessiz veri kaybı yok | `validator` | UYGULANDI | `validator::tests::tampered_pool_that_overruns_is_detected` |
| R15.6 | UDF birincil, diğerleri yardımcı | `birkat-report::text::auxiliary_note` | UYGULANDI | `text::tests::auxiliary_note_states_udf_is_primary` |
| R15.7 | UDF önizleme | `birkat-report::preview` | UYGULANDI | `preview::tests::*` (10 test) |

---

## 16-19. İmza, teslim, heyet, güvenlik (şartname 16-19)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R16.1 | İmza hazırlığı, hash kaydı, sahte imza yok | `birkat-core::report::SignatureRecord` | UYGULANDI | `report::tests::signed_report_is_locked` |
| R16.2 | UYAP Portal uyumlu akış, parola saklamaz | `birkat-report::auxiliary::default_delivery_checklist` | UYGULANDI | `auxiliary::tests::delivery_checklist_has_default_items` |
| R17 | Heyet: profiller, ayrı görüş, çakışma koruması | `birkat-core::admin` | UYGULANDI | `admin::tests::merge_reports_conflict_when_both_changed` |
| R18 | Takvim, ücret, gider, performans | `birkat-core::admin`, `case` | UYGULANDI | `admin::tests::tariff_calculates_tiered_amount` |
| R19.1 | Çevrimdışı çalışma, buluta gönderim yok | Tümü | UYGULANDI | `updater::tests::network_note_states_no_case_data_upload` |
| R19.2 | AES-256-GCM, Argon2id, DPAPI benzeri | `birkat-crypto` | UYGULANDI | `crypto::tests::*` (11 test) |
| R19.3 | Kullanıcı kilidi, deneme politikası | `crypto::FailedAttemptPolicy` | UYGULANDI | `crypto::tests::lockout_policy_locks_after_max_attempts` |
| R19.4 | Şifreli yedek + gerçek geri yükleme testi | `birkat-backup` | UYGULANDI | `backup::tests::verify_restore_tests_integrity` |
| R19.5 | Saklama politikası, güvenli silme | `backup::RetentionPolicy` | UYGULANDI | `backup::tests::retention_policy_keeps_newest` |

---

## 20-22. Teknoloji, veri modeli, güncelleme (şartname 20-22)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R20.2 | Rust, egui/eframe, Candle, Symphonia, EWF, SQLite | `Cargo.toml` | UYGULANDI | `cargo build` |
| R20.3 | Electron/Tauri/Node/Python yok | `Cargo.toml` | UYGULANDI | Bağımlılık denetimi |
| R21.1 | 36 temel veri nesnesi | `birkat-core` | UYGULANDI | Tüm modül testleri |
| R21.2 | Kanonik rapor verisi tek kaynak | `birkat-report` | UYGULANDI | `to_udf::tests::*` |
| R21.3 | 22 kalıcı iş akışı durumu | `birkat-core::workflow` | UYGULANDI | `workflow::tests::full_happy_path_reaches_archived` |
| R22 | İmzalı güncelleme, ön yedek, geri dönüş | `birkat-updater` | UYGULANDI | `updater::tests::install_and_rollback` |

---

## 23-28. Test, arayüz, teslim, kurallar (şartname 23-28)

| Kimlik | Gereksinim | Modül | Durum | Test |
|---|---|---|---|---|
| R23.1 | Türkçe ASR test seti ve WER/CER | `docs/asr-benchmark.md` | SINIRLI | Ölçüm çerçevesi hazır; veri seti telif nedeniyle teslim edilmedi |
| R23.2 | UDF test seti | `docs/udf-uyumluluk.md` | SINIRLI | Test seti tanımlandı; gerçek örnekler telif nedeniyle teslim edilmedi |
| R23.3 | Delil edinim testleri | `birkat-evidence::*` | UYGULANDI | 69 test |
| R23.4 | Dayanıklılık testleri | `birkat-backup`, `birkat-app::store`, `birkat-transcription::queue` | UYGULANDI | `queue::tests::crash_recovery_*` |
| R23.5 | Test raporları | `docs/test-raporu.md` | UYGULANDI | — |
| R24 | 20 Türkçe çalışma alanı | `birkat-desktop::screens` | UYGULANDI | `desktop::tests::all_screens_have_turkish_titles` |
| R25 | 26 teslim kalemi | `docs/`, `scripts/` | UYGULANDI | Bu belge seti |
| R26 | 49 adımlık uçtan uca kabul | `docs/kabul-testi-matrisi.md` | SINIRLI | 46 adım doğrulandı; 3 adım Windows/UYAP Editör gerektirir |
| R27 | 44 tamamlanma kriteri | `docs/kabul-testi-matrisi.md` | SINIRLI | Aynı |
| R28 | 28 ürün kuralı | Tümü | UYGULANDI | Her kural için test mevcut |

---

## Bilinen sınırlamaların özeti

| Sınırlama | Etki | Çözüm |
|---|---|---|
| Windows'a özgü işlevler platform katmanında | Gerçek donanım testi gerekir | Windows'ta gerçek donanımla doğrulama |
| UYAP Editör uyumluluk testi yapılmadı | Nihai biçim doğrulaması eksik | Resmî Editör ile test (prosedür belgelendi) |
| OCR motoru paketlenmedi | Taranmış PDF'ler metne çevrilemez | OCR kancası hazır; paketleme adımı belgelendi |
| 7z/RAR açılamaz | Bu arşivler hata ile bildirilir | ZIP'e dönüştürme veya bağımlılık ekleme |
| Model dosyaları depoda değil | İlk derlemede transkripsiyon çalışmaz | Kurulum paketine ekleme (betik hazır) |
| Görüntü sunucusu gerekir | Başsız ortamda arayüz açılamaz | Windows/masaüstü ortamı |
| ASR/UDF test setleri telifli | Nicel ölçüm yapılmadı | Kurumun kendi verisiyle ölçüm |
| Ed25519 doğrulaması üretimde etkinleştirilir | Geliştirme derlemesinde imzalı güncelleme reddedilir | Üretim derlemesinde etkinleştirme |
