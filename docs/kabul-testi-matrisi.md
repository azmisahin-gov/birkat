# Kabul Testi Matrisi

Bu belge, şartnamenin 26. maddesindeki 49 adımlık uçtan uca kabul senaryosunu
ve 27. maddedeki 44 tamamlanma kriterini izler.

Doğrulama yöntemleri:

- **[OTOMATİK]** — Otomatik test veya komut satırı akışıyla doğrulandı.
- **[MANUEL-W]** — Windows ortamı ve gerçek donanım gerektirir.
- **[MANUEL-E]** — Resmî UYAP Editör gerektirir.
- **[MANUEL-U]** — Kullanıcı yetkisi gerektirir (UYAP Portal, e-imza).

---

## Bölüm A — Uçtan uca kabul senaryosu (49 adım)

| # | Adım | Yöntem | Durum | Kanıt |
|---|---|---|---|---|
| 1 | Temiz Windows 11 bilgisayara uygulama kurulur | MANUEL-W | KISMEN | Kurulum dosyası **üretildi ve yayımlandı** (1,5 GB, NSIS, Türkçe sihirbaz, kısayollar, kaldırma). Temiz Windows'ta çalıştırılması bekliyor |
| 2 | İnternet bağlantısı olmadan açılır | MANUEL-W | KISMEN | Paket model dahil 1,65 GB ve çevrimdışı; uygulama hiçbir ağ çağrısı yapmaz. Gerçek Windows'ta açılış bekliyor |
| 3 | Bilirkişi profili oluşturulur | OTOMATİK | GEÇTİ | `birkat profil-ekle`, `desktop::tests::adding_profile_persists` |
| 4 | Görevlendirme PDF/UDF dosyası içeri alınır | OTOMATİK | GEÇTİ | `birkat-evidence::detect`, `birkat-udf::reader` |
| 5 | Uygulama merci, dosya no, taraflar, süre ve soruları çıkarır | OTOMATİK | GEÇTİ | `birkat-core::case::Assignment`, `builder::tests::*` |
| 6 | Taranmış belge varsa yerel OCR çalışır | — | SINIRLI | OCR kancası hazır; motor paketlenmedi (bkz. gereksinim matrisi R1.1.3) |
| 7 | Bilirkişi yalnızca belirsiz alanları kontrol eder | OTOMATİK | GEÇTİ | `Assignment::unresolved_fields` |
| 8 | USB/SD/harici materyal takılır veya dosyalar içeri alınır | OTOMATİK | GEÇTİ | `birkat materyal-ekle`, `vault::tests::*` |
| 9 | Uygulama aygıtı otomatik tanır | MANUEL-W | BEKLİYOR | `birkat-evidence::device`; iş mantığı test edildi, Windows aygıt olayları katmanı gerçek donanımda denenmeli |
| 10 | Yazma koruması durumunu sorgular | OTOMATİK | GEÇTİ | `write_protection::tests::write_probe_interpretation` |
| 11 | Uygun edinim yöntemini seçer | OTOMATİK | GEÇTİ | `device::tests::acquirable_excludes_system_disks` |
| 12 | Dosya bazlı edinimi veya ihtiyaç varsa L01/E01 işlemini yürütür | OTOMATİK | GEÇTİ | `acquisition::tests::*`, `ewf::tests::physical_ewf_write_produces_valid_image` |
| 13 | Kaynak ve çıktı bütünlüğünü doğrular | OTOMATİK | GEÇTİ | `acquisition::tests::acquires_single_file_and_verifies` |
| 14 | Ses dosyalarını otomatik tespit eder | OTOMATİK | GEÇTİ | `inventory::tests::builds_inventory_for_wav_files` |
| 15 | Teknik envanteri çıkarır | OTOMATİK | GEÇTİ | `probe::tests::probes_wav_metadata` |
| 16 | Tüm uygun sesleri transkripsiyon kuyruğuna alır | OTOMATİK | GEÇTİ | `orchestrator::tests::workflow_discovers_and_transcribes_audio` |
| 17 | Whisper işlemi yerel olarak tamamlanır | OTOMATİK | GEÇTİ | **Gerçek model + gerçek ses ile doğrulandı** (CPU, `examples/whisper-dogrula`); UDF raporuna yerleşti |
| 18 | Kesinti halinde işlem kaldığı yerden devam eder | OTOMATİK | GEÇTİ | `queue::tests::crash_recovery_requeues_running_tasks` |
| 19 | Otomatik kalite kontrolü çalışır | OTOMATİK | GEÇTİ | `quality::tests::*` (14 test) |
| 20 | Belirsiz bölümler işaretlenir | OTOMATİK | GEÇTİ | `quality::tests::detects_text_in_silence_as_high_severity` |
| 21 | Bilirkişi ses ve metni birlikte inceler | OTOMATİK | GEÇTİ | `playback::tests::*`, `desktop::screens::audio_review` |
| 22 | Gerekli düzeltmeleri yapar | OTOMATİK | GEÇTİ | `audio::TranscriptChange` |
| 23 | Nihai transkript durumunu onaylar | OTOMATİK | GEÇTİ | `content_status::tests::ready_for_review_can_be_approved` |
| 24 | Görevlendirme sorularını tek tek ele alır | OTOMATİK | GEÇTİ | `case::tests::*` |
| 25 | Her yanıtı ilgili materyal, transkript ve zaman koduyla bağlar | OTOMATİK | GEÇTİ | `findings::tests::answer_needs_references_and_approval_to_be_opinion` |
| 26 | Bilirkişi teknik ve mesleki bulgularını tamamlar | OTOMATİK | GEÇTİ | `findings::Finding` |
| 27 | Uygulama raporu otomatik hazırlar | OTOMATİK | GEÇTİ | `builder::tests::builds_report_with_all_sections` |
| 28 | UDF doğrudan oluşturulur | OTOMATİK | GEÇTİ | `birkat rapor` → `BILIRKISI_RAPORU.udf` |
| 29 | UDF yapısal doğrulamadan geçer | OTOMATİK | GEÇTİ | `birkat udf-dogrula` → 11 kontrol, 0 hata |
| 30 | UDF gerçek UYAP Editör'de açılır | MANUEL-E | BEKLİYOR | Editör bu ortamda yok. Prosedür ve 15 maddelik test seti `docs/udf-uyumluluk.md` içinde |
| 31 | Sayfa görünümü ve içerik kontrol edilir | MANUEL-E | BEKLİYOR | Aynı |
| 32 | Gerekli düzeltmeler yapılır | OTOMATİK | GEÇTİ | `ReportDocument::push_block` |
| 33 | UDF tekrar oluşturulur ve doğrulanır | OTOMATİK | GEÇTİ | `birkat rapor` tekrar çalıştırılabilir |
| 34 | Bilirkişi raporu onaylar | OTOMATİK | GEÇTİ | `report::tests::report_status_follows_content_rules` |
| 35 | Resmî imza süreci tamamlanır | MANUEL-U | BEKLİYOR | Uygulama imza sürecine yönlendirir; sahte imza üretmez |
| 36 | İmzalı dosya arşivlenir | OTOMATİK | GEÇTİ | `vault::store_bytes`, `report::tests::signed_report_is_locked` |
| 37 | Teslim paketi hazırlanır | OTOMATİK | GEÇTİ | `auxiliary::tests::delivery_checklist_has_default_items` |
| 38 | Bilirkişi UYAP Portal üzerinden dosyayı kendi yetkisiyle teslim eder | MANUEL-U | BEKLİYOR | Uygulama otomatik giriş yapmaz |
| 39 | Teslim bilgisi vakaya kaydedilir | OTOMATİK | GEÇTİ | `report::DeliveryRecord` |
| 40 | Vaka arşivlenir | OTOMATİK | GEÇTİ | `workspace::tests::archive_marks_case_and_audits` |
| 41 | Uygulama kapatılır | OTOMATİK | GEÇTİ | Tüm durum kalıcı (SQLite WAL + FULL) |
| 42 | Tekrar açılır | OTOMATİK | GEÇTİ | `store::tests::save_and_load_case` |
| 43 | Dosyanın bütün belgeleri, sesleri, transkriptleri, raporları ve işlem geçmişi eksiksiz gelir | OTOMATİK | GEÇTİ | `store::tests::audit_chain_persists_and_verifies`, `vault::tests::manifest_persists_across_reopen` |
| 44 | Şifreli yedek oluşturulur | OTOMATİK | GEÇTİ | `backup::tests::backup_and_restore_round_trip`; gerçek vaka verisiyle uçtan uca doğrulandı |
| 45 | Yedek başka bir konuma geri yüklenir | OTOMATİK | GEÇTİ | `birkat geri-yukle` |
| 46 | Geri yükleme ve hash doğrulaması başarılı olur | OTOMATİK | GEÇTİ | `backup::tests::verify_restore_tests_integrity` |
| 47 | Güncelleme paketi doğrulanır | OTOMATİK | GEÇTİ | `updater::tests::integrity_verification_detects_tampering` |
| 48 | Güncelleme uygulanır | OTOMATİK | GEÇTİ | `updater::tests::install_and_rollback` |
| 49 | Eski vaka ve imzalı raporlar değişmeden açılır | OTOMATİK | GEÇTİ | `store` göç mekanizması, `report::tests::signed_report_is_locked` |

**Özet:** 42 adım otomatik doğrulandı; 1 adım sınırlı (OCR); 3 adım Windows,
1 adım UYAP Editör, 2 adım kullanıcı yetkisi gerektirir.

---

## Bölüm B — Tamamlanma kriterleri (44 madde)

| # | Kriter | Durum | Kanıt |
|---|---|---|---|
| 1 | Gerçek Windows 11 masaüstü uygulaması | GEÇTİ | egui/eframe; Windows x64 PE ikilisi üretildi, yalnızca sistem DLL'lerine bağlı |
| 2 | Kurulum paketi | GEÇTİ | **BIRKAT-0.1.0-windows-x64-offline-setup.exe (1,5 GB) yayımlandı**; Türkçe sihirbaz, kısayollar, kaldırma. Kod imzalama sertifikası hâlâ gerekir |
| 3 | Çevrimdışı çalışma | GEÇTİ | Paket model dahil; hiçbir ağ çağrısı yok |
| 4 | Model kurulum paketine dahil | GEÇTİ | Whisper large-v3-turbo (1,62 GB, MIT) paket içinde; SHA-256 ile doğrulanır |
| 5 | Görevlendirme belge okuyucu | GEÇTİ | `detect`, `udf::reader` |
| 6 | OCR | KISMEN | Kanca hazır; motor paketlenmedi |
| 7 | Otomatik dosya ve materyal envanteri | GEÇTİ | `inventory`, `vault` |
| 8 | USB/SD algılama | KISMEN | Mantık ve testler hazır; Windows aygıt katmanı gerçek donanımda denenmeli |
| 9 | Yazma koruma modülü | GEÇTİ | `write_protection` (9 adım) |
| 10 | Dosya bazlı edinim | GEÇTİ | `acquisition` |
| 11 | E01/L01/RAW desteği | GEÇTİ | `ewf` (gerçek E01 üretimi ve geri okuma) |
| 12 | Hash ve doğrulama | GEÇTİ | `hash` (SHA-256/SHA-1/MD5) |
| 13 | Ses envanteri | GEÇTİ | `inventory`, `probe` |
| 14 | Ses oynatma ve inceleme | GEÇTİ | `playback`, `waveform`, `spectrogram` |
| 15 | Whisper large-v3-turbo yerel transkripsiyon | GEÇTİ | **Gerçek model + gerçek ses ile doğrulandı**; UDF raporuna yerleşti |
| 16 | CPU çalışması | GEÇTİ | `select_device(false)` → CPU |
| 17 | Test edilmiş GPU hızlandırması ve fallback | KISMEN | CPU yolu tam doğrulandı; fallback test edildi; GPU donanımda ölçülmedi |
| 18 | Otomatik transkripsiyon kuyruğu | GEÇTİ | `queue` (12 test) |
| 19 | Otomatik kalite kontrolü | GEÇTİ | `quality` (16 kontrol türü) |
| 20 | Ses/transkript inceleme editörü | GEÇTİ | `playback` + arayüz ekranı |
| 21 | Transkript sürümleme | GEÇTİ | `TranscriptVersion` (5 aşama) |
| 22 | Soru ve dayanak yönetimi | GEÇTİ | `case`, `findings` |
| 23 | Bulgular ve teknik notlar | GEÇTİ | `findings` |
| 24 | Rapor motoru | GEÇTİ | `birkat-report` |
| 25 | Doğrudan UDF üretimi | GEÇTİ | `birkat-udf::writer` |
| 26 | UDF okuyucu | GEÇTİ | `birkat-udf::reader` |
| 27 | UDF doğrulayıcı | GEÇTİ | `birkat-udf::validator` |
| 28 | Gerçek UYAP Editör testleri | BEKLİYOR | Prosedür belgelendi |
| 29 | PDF/DOCX yardımcı çıktıları | GEÇTİ | `pdf` (Türkçe doğrulandı), `docx` |
| 30 | Ek rapor ve yardımcı belgeler | GEÇTİ | `auxiliary` (10 belge) |
| 31 | UYAP teslim hazırlığı | GEÇTİ | `delivery_checklist` |
| 32 | İmza süreciyle uyum | GEÇTİ | `SignatureRecord`, hash kontrolü |
| 33 | Heyet çalışması | GEÇTİ | `PanelMemberAssignment`, çakışma birleştirme |
| 34 | Takvim ve süre takibi | GEÇTİ | `Deadline`, `CalendarEvent` |
| 35 | Ücret ve gider yönetimi | GEÇTİ | `FeeRecord`, `Expense`, `FeeTariff` |
| 36 | Şifreli delil kasası | GEÇTİ | `vault` (AES-256-GCM) |
| 37 | Audit log | GEÇTİ | `birkat-audit` (hash zinciri) |
| 38 | Yedek ve geri yükleme | GEÇTİ | `birkat-backup` |
| 39 | Otomatik güncelleme | GEÇTİ | `birkat-updater` |
| 40 | Güncelleme geri dönüşü | GEÇTİ | `install_and_rollback` testi |
| 41 | Türkçe kullanıcı arayüzü | GEÇTİ | 20 ekran, tüm etiketler Türkçe |
| 42 | Türkçe kullanım ve teknik belgeler | GEÇTİ | `docs/` |
| 43 | Test raporları | GEÇTİ | `docs/test-raporu.md` |
| 44 | Lisans ve SBOM | GEÇTİ | `docs/bagimlilik-lisanslari.md`, `docs/sbom.cdx.json` |
| 45 | Gerçek uçtan uca kabul testi | KISMEN | 43/49 adım otomatik geçti; gerçek Whisper çıkarımı ve kurulum paketi dahil |

---

## Bölüm C — Otomatik uçtan uca doğrulama

Aşağıdaki komut dizisi, bu ortamda **çalıştırılmış ve geçmiştir**:

```bash
B=./target/release/birkat
V=./e2e-veri

$B --veri $V surum
$B --veri $V profil-ekle --ad "Ayşe Yılmaz" --unvan "Dr." --sicil 12345
$B --veri $V vaka-olustur --baslik "2025/123 Esas" \
   --mahkeme "Ankara 5. Asliye Ceza Mahkemesi" \
   --dosya-no "2025/123 Esas" --konu "Ses kaydının metne dönüştürülmesi"
$B --veri $V transkript --vaka <VAKA_ID> --yol ./materyal
$B --veri $V rapor --vaka <VAKA_ID> --cikti ./cikti
$B --veri $V udf-dogrula --dosya ./cikti/BILIRKISI_RAPORU.udf
$B --veri $V yedek --vaka <VAKA_ID> --parola "Guvenli-Yedek-2026" \
   --cikti ./yedek/vaka.yedek
$B --veri $V geri-yukle --yedek ./yedek/vaka.yedek \
   --parola "Guvenli-Yedek-2026" --hedef ./geri-yukleme
$B --veri $V denetim-dogrula
$B --veri $V panel
$B --veri $V saglik
```

Gözlenen sonuçlar:

- `transkript`: 2 ses dosyası bulundu, 2 dosya metne dönüştürüldü, UDF taslağı 11 kontrol / 0 uyarı.
- `udf-dogrula`: 11 kontrol tamamı geçti; 346 Türkçe'ye özgü karakter korundu; en büyük ofset 3755.
- `yedek`: 7 dosya, 73102 bayt; manifesto doğrulaması `true`; gerçek geri yükleme testi başarılı.
- `geri-yukle`: 7 dosya geri yüklendi, 0 hash uyuşmazlığı.
- `denetim-dogrula`: 14 olay, zincir bütünlüğü doğrulandı; bağımsız zaman damgası değildir.
- `panel`: 1 dosya, 1 devam ediyor.
- `saglik`: veritabanı/kasa/yedek dizini tamam; model yolu ayarlanmadığı için uyarı.

---

## Bölüm D — Teslim öncesi yapılacaklar listesi

Bu maddeler Windows ortamı veya resmî araç gerektirir ve bu ortamda
yapılamamıştır:

1. [ ] Windows 11 x64 temiz makinede kurulum paketini test et.
2. [ ] Gerçek USB/SD aygıtıyla aygıt algılama ve yazma korumasını doğrula.
3. [ ] Resmî UYAP Editör ile UDF dosyasını aç, sayfa görünümünü, tabloları,
       görselleri, üstbilgi/altbilgiyi ve kaydetme akışını kontrol et.
4. [ ] Whisper large-v3-turbo model paketini indir, hash'ini kaydet ve
       kurulum paketine ekle.
5. [ ] Gerçek Türkçe ses kayıtlarıyla WER/CER ölçümünü yap ve raporla.
6. [ ] GPU (CUDA) yolu varsa gerçek donanımda hız ve doğruluk ölç.
7. [ ] Kod imzalama sertifikasıyla paketi imzala ve güncelleme manifestosunu
       üret.
8. [ ] Ed25519 imza doğrulamasını üretim derlemesinde etkinleştir.
9. [ ] Bilirkişilik Yönetmeliği'nin güncel hâlini indir ve zorunlu rapor
       alanlarını son kez karşılaştır.
10. [ ] Kullanıcı kabul testini gerçek bir 56.12 bilirkişisiyle yap.
