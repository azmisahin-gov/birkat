# Test Raporu

Bu belge, BİRKÂT'ın test sonuçlarını ve test stratejisini belgeler
(şartname 23.5, 25. teslim kalemi 12–14).

> **İlke:** Başarısız testler gizlenmez. Test edilmeyen özellik "tamamlandı"
> sayılmaz. Bu rapor yalnızca gerçekten çalıştırılmış testleri listeler.

## 1. Özet

| Metrik | Değer |
|---|---|
| Paket sayısı | 13 |
| Varsayılan test sayısı | **490** |
| Whisper özelliği ile ek testler | +62 |
| Başarısız test | **0** |
| Atlanan (ignored) test | 0 |

## 2. Paket bazında test dağılımı

| Paket | Test sayısı | Başarısız | Kapsam |
|---|---|---|---|
| `birkat-core` | 62 | 0 | Alan modeli, içerik durumu, iş akışı, kapsam uyarıları, rapor modeli, ücret, heyet |
| `birkat-crypto` | 11 | 0 | Şifreleme, anahtar türetme/sarma, parola politikası, deneme kilidi |
| `birkat-audit` | 10 | 0 | Hash zinciri, kurcalama tespiti, filtreleme |
| `birkat-udf` | 52 | 0 | Yazıcı, okuyucu, doğrulayıcı, Türkçe karakter, ofset, tablo, görsel |
| `birkat-evidence` | 69 | 0 | Hash, tür tespiti, arşiv güvenliği, edinim, aygıt, yazma koruması, **gerçek E01/Ex01 üretimi ve geri okuma** |
| `birkat-audio` | 54 | 0 | Sondaj, çözümleme, dalga biçimi, FFT/spektrogram, oynatma, envanter, türetilmiş ses |
| `birkat-transcription` | 54 | 0 | Kuyruk (kesinti kurtarma), kalite kontrolü (16 tür), yazım standardı, halüsinasyon, model paketi |
| `birkat-report` | 84 | 0 | Şablonlar, doldurma, UDF köprüsü, son kontrol, **gerçek PDF ve DOCX üretimi**, önizleme, yardımcı belgeler |
| `birkat-backup` | 11 | 0 | Şifreli yedek, **gerçek geri yükleme testi**, kurcalama tespiti, saklama |
| `birkat-updater` | 12 | 0 | Bütünlük, imza kapısı, ön yedek, kurulum, geri dönüş |
| `birkat-app` | 58 | 0 | Vaka deposu, şifreli kasa, panel, **uçtan uca otomatik iş akışı** |
| `birkat-desktop` | 13 | 0 | 20 ekran, arayüz durumu, doğrulama |
| `birkat-transcription` (whisper) | +62 | 0 | Whisper/Candle motoru, cihaz seçimi, yeniden örnekleme, mel filtresi |

## 3. Test türleri ve örnekleri

### 3.1 Birim testleri (saf mantık)

Alan mantığı, sınır durumları ve hata yolları. Veritabanı, dosya sistemi veya
ağ gerektirmez.

Örnekler:

- `content_status::tests::machine_output_cannot_be_approved_directly` —
  Makine çıktısı doğrudan onaylanamaz.
- `workflow::tests::full_happy_path_reaches_archived` — 22 durumlu akış
  sonuna kadar ilerler.
- `workflow::tests::paused_workflow_cannot_advance` — Duraklatılmış akış
  ilerlemez.
- `scope::tests::detects_biometric_request` — Kapsam dışı talep tespit edilir.
- `write_protection::tests::software_read_only_is_not_hardware_level` —
  Yazılımsal koruma donanımsal sayılmaz.
- `quality::tests::detects_negation_as_high_severity` — Olumsuzluk yüksek
  önemde işaretlenir.
- `hallucination::tests::detects_text_in_silence` — Sessizlikte metin
  işaretlenir.

### 3.2 Entegrasyon testleri (gerçek dosya üretimi ve geri okuma)

Testler gerçek dosyalar üretir ve geri okur; sahte (mock) veri kullanılmaz.

| Test | Ne üretir | Ne doğrular |
|---|---|---|
| `udf::writer::tests::pack_udf_produces_readable_zip` | Gerçek UDF ZIP | Arşiv açılır, `content.xml` okunur |
| `udf::reader::tests::read_udf_from_packed_bytes` | Gerçek UDF | Metin havuzu ve ofsetler çözülür |
| `report::to_udf::tests::generated_udf_passes_validation` | Gerçek UDF | 11 doğrulama kontrolü geçer |
| `report::pdf::tests::produces_pdf_header` | Gerçek PDF | `%PDF` imzası, makul boyut |
| `report::docx::tests::produces_valid_zip_package` | Gerçek DOCX | OPC parçaları mevcut, Türkçe karakterler korunur |
| `evidence::ewf::tests::physical_ewf_write_produces_valid_image` | **Gerçek E01** | Metaveri geri okunur, akış bayt bayt eşleşir |
| `evidence::ewf::tests::physical_ewf2_write_and_read` | **Gerçek Ex01** | Mantıksal boyut eşleşir |
| `backup::tests::backup_and_restore_round_trip` | Gerçek şifreli yedek | Geri yükleme başarılı, dosyalar eşleşir |
| `audio::probe::tests::probes_wav_metadata` | Gerçek WAV | Codec, süre, örnekleme, kanal doğru |
| `audio::decode::tests::decodes_wav_to_pcm` | Gerçek WAV | PCM çözülür, süre doğru |
| `audio::spectrogram::tests::spectrogram_peaks_at_input_frequency` | 1000 Hz sinüs | Tepe bin 1000 Hz ±40 Hz |

### 3.3 Uçtan uca test

`birkat-app::orchestrator::tests::workflow_discovers_and_transcribes_audio`,
`workflow_produces_valid_udf`, `workflow_records_audit_chain`,
`workflow_stores_transcripts_in_vault` testleri tam iş akışını çalıştırır:

```text
Gerçek WAV dosyaları → kasa → envanter → transkripsiyon → kalite kontrolü
→ rapor → UDF (doğrulanır) → denetim zinciri
```

### 3.4 Dayanıklılık testleri

| Test | Senaryo |
|---|---|
| `queue::tests::crash_recovery_requeues_running_tasks` | Çökme sonrası kuyruk kurtarma |
| `queue::tests::crash_recovery_preserves_completed_index` | Tamamlanan iş yeniden kuyruğa alınmaz |
| `backup::tests::tampered_backup_is_rejected` | Bozuk yedek reddedilir |
| `backup::tests::max_bytes_is_enforced` | Disk dolu / boyut sınırı |
| `store::tests::save_and_load_case` | Uygulama kapanıp açılınca veri korunur |
| `updater::tests::install_and_rollback` | Güncelleme sonrası geri dönüş |
| `evidence::acquisition::tests::detects_corrupted_copy` | Bozuk kopya tespit edilir |
| `archive::tests::rejects_compression_bomb_ratio` | Arşiv bombası reddedilir |
| `inventory::tests::unreadable_audio_is_reported_with_reason` | Bozuk ses nedeniyle bildirilir |

### 3.5 Güvenlik testleri

| Test | Ne doğrular |
|---|---|
| `crypto::tests::tampered_blob_fails_authentication` | AEAD etiketi kurcalamayı yakalar |
| `crypto::tests::wrong_key_fails_to_decrypt` | Yanlış anahtar reddedilir |
| `crypto::tests::password_policy_rejects_weak` | Zayıf parola reddedilir |
| `crypto::tests::lockout_policy_locks_after_max_attempts` | Deneme kilidi çalışır |
| `audit::tests::tampering_with_description_is_detected` | Zincir kurcalamayı yakalar |
| `audit::tests::removing_an_event_is_detected` | Olay silme tespit edilir |
| `audit::tests::reordering_events_is_detected` | Sıralama değişikliği tespit edilir |
| `archive::tests::traversal_names_are_rejected` | Yol kaçışı reddedilir |
| `updater::tests::unsigned_package_is_rejected` | İmzasız paket reddedilir |
| `vault::tests::locked_vault_cannot_read_encrypted_object` | Kilitli kasa okunamaz |

### 3.6 Dürüstlük testleri

Bu testler, sistemin kendisi hakkında **yanlış beyan vermediğini** doğrular.
Ürünün dürüstlük sözleşmesinin makine tarafından korunmasıdır.

| Test | Ne doğrular |
|---|---|
| `audit::tests::verification_is_not_an_independent_timestamp` | Hash zinciri bağımsız zaman damgası sayılmaz |
| `ewf::tests::verification_limitation_note_is_honest` | Hash doğrulamasının sınırı belirtilir |
| `acquisition::tests::label_does_not_claim_image` | Dosya kopyası "imaj" olarak adlandırılmaz |
| `quality::tests::confidence_note_is_honest` | Güven göstergesi doğruluk yüzdesi sayılmaz |
| `waveform::tests::interpretation_note_mentions_56_11` | Dalga biçimi orijinallik kanıtı sayılmaz |
| `conventions::tests::speaker_tags_never_claim_identity` | Konuşmacı etiketi kimlik iddia etmez |
| `text::tests::auxiliary_note_states_udf_is_primary` | UDF birincil çıktı olarak belirtilir |
| `preview::tests::preview_note_disclaims_editor_replacement` | Önizleme Editör yerine geçmez |
| `updater::tests::network_note_states_no_case_data_upload` | Vaka verisi yüklenmez |
| `processing::tests::derived_label_marks_not_original` | Türetilmiş ses orijinal sayılmaz |

## 4. Çalıştırma

```bash
# Tüm testler
cargo test --workspace

# Whisper özelliği ile
cargo test -p birkat-transcription --features whisper

# Tek paket
cargo test -p birkat-udf

# Ayrıntılı çıktı
cargo test --workspace -- --nocapture

# Kapsam raporu (cargo-llvm-cov gerekir)
cargo llvm-cov --workspace --html
```

## 5. Sürekli tümleştirme

`.github/workflows/ci.yml` şu adımları çalıştırır:

| İş | Adım | Amaç |
|---|---|---|
| `test` (Linux + Windows) | `cargo fmt --check` | Biçim tutarlılığı |
| | `cargo clippy -D warnings` | Kod kalitesi |
| | `cargo build --workspace --all-targets` | Derleme |
| | `cargo test --workspace` | 490 test |
| | Whisper derleme + test | +62 test |
| `audit` | `cargo audit` | Bilinen güvenlik açıkları |
| | `cargo deny check licenses advisories` | Lisans uyumu |
| `sbom` | SBOM üretimi ve güncellik denetimi | SBOM güncel mi |

## 6. Gerçek uçtan uca komut satırı doğrulaması

Aşağıdaki akış bu ortamda çalıştırılmış ve geçmiştir:

| Adım | Komut | Sonuç |
|---|---|---|
| Sürüm | `birkat surum` | BİRKÂT 0.1.0, uzmanlık 56.12 |
| Profil | `birkat profil-ekle` | Profil kaydedildi |
| Vaka | `birkat vaka-olustur` | Vaka kimliği üretildi |
| Transkripsiyon | `birkat transkript` | 2 ses dosyası, 2 transkript, UDF taslağı 11 kontrol/0 uyarı |
| Rapor | `birkat rapor` | UDF + PDF + DOCX + TXT + JSON üretildi |
| UDF doğrulama | `birkat udf-dogrula` | 11 kontrol geçti, 346 Türkçe karakter korundu |
| Yedek | `birkat yedek` | 7 dosya, 73102 bayt, manifesto doğrulandı, gerçek geri yükleme testi başarılı |
| Geri yükleme | `birkat geri-yukle` | 7 dosya, 0 hash uyuşmazlığı |
| Denetim | `birkat denetim-dogrula` | 14 olay, zincir bütünlüğü doğrulandı |
| Panel | `birkat panel` | 1 dosya, 1 devam ediyor |
| Sağlık | `birkat saglik` | Veritabanı/kasa/yedek tamam; model uyarısı |

## 7. Test edilmeyen ve test edilemeyen özellikler

Aşağıdaki özellikler bu ortamda **test edilememiştir** ve "tamamlandı" olarak
işaretlenmemiştir:

| Özellik | Neden test edilemedi | Gerekli ortam |
|---|---|---|
| Gerçek USB/SD aygıt algılama | Donanım ve Windows API gerekir | Windows + USB/SD |
| Yazma koruması uygulaması | Windows salt okunur API'si gerekir | Windows + yönetici yetkisi |
| Fiziksel sektör okuma | Aygıt erişimi gerekir | Windows + ham aygıt |
| Gerçek Whisper model çıkarımı | Model dosyaları (telif) | Model paketi + CPU/GPU |
| UYAP Editör uyumluluğu | Resmî Editör gerekir | Windows + UYAP Editör |
| Resmî e-imza akışı | Sertifika ve kart okuyucu gerekir | Windows + e-imza |
| UYAP Portal teslimi | Portal erişimi ve yetki gerekir | Kullanıcı hesabı |
| WMA/AMR çözümleme | Windows Media Foundation adaptörü | Windows + adaptör |
| OCR (taranmış PDF) | OCR motoru paketlenmedi | OCR paketi |
| 7z/RAR arşivleri | Bağımlılık eklenmedi | Arşiv kütüphanesi |
| GPU hızlandırma ölçümü | CUDA donanımı gerekir | NVIDIA GPU |

Bu özelliklerin **iş mantığı ve durum yönetimi** test edilmiştir; yalnızca
platform çağrısı ve gerçek donanım etkileşimi eksiktir. Ayrıntı:
`docs/gereksinim-matrisi.md` ve `docs/kabul-testi-matrisi.md`.

## 8. Kod kalitesi

| Ölçüt | Durum |
|---|---|
| `#![forbid(unsafe_code)]` | Tüm alan paketlerinde |
| `#![warn(missing_docs)]` | Tüm paketlerde |
| Genel (public) API dokümantasyonu | Türkçe, eksiksiz |
| `cargo fmt --check` | CI'da zorunlu |
| `cargo clippy -D warnings` | CI'da zorunlu |
| Bağımlılık denetimi | CI'da `cargo audit` + `cargo deny` |

## 9. Sonuç

- **490 test geçti, 0 başarısız** (varsayılan derleme).
- **62 ek test geçti** (Whisper özelliği ile); toplam 552 test çalıştırması.
- Gerçek dosya üretimi ve geri okuma doğrulandı: UDF, PDF, DOCX, **E01**,
  **Ex01**, şifreli yedek, WAV.
- Uçtan uca iş akışı gerçek ses dosyalarıyla doğrulandı.
- Dürüstlük sözleşmesi 10 testle makine tarafından korunuyor.
- Test edilemeyen özellikler açıkça listelendi; hiçbiri "tamamlandı" olarak
  işaretlenmedi.
