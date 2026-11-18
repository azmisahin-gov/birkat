# Teknik Mimari

## 1. Genel bakış

BİRKÂT, tek bir vaka etrafında birleşen yerel bir masaüstü uygulamasıdır.
Mimari üç katmana ayrılır:

```text
┌─────────────────────────────────────────────────────────────┐
│  Sunum katmanı                                              │
│  birkat-desktop (egui/eframe, 20 çalışma alanı)             │
│  birkat-cli     (komut satırı)                              │
├─────────────────────────────────────────────────────────────┤
│  Uygulama katmanı                                           │
│  birkat-app   (vaka deposu, kasa, panel, otomatik iş akışı) │
├─────────────────────────────────────────────────────────────┤
│  Alan katmanı                                               │
│  birkat-core  birkat-crypto  birkat-audit  birkat-evidence  │
│  birkat-audio birkat-transcription birkat-udf               │
│  birkat-report birkat-backup birkat-updater                 │
└─────────────────────────────────────────────────────────────┘
```

**Kural:** İş mantığı alan katmanındadır ve platformdan bağımsızdır. Dosya
sistemi, aygıt, ses çıkışı, imza ve güncelleme gibi platforma bağlı işlemler
ince bir platform katmanı üzerinden çağrılır. Bu sayede iş mantığı Linux
üzerinde de test edilebilir.

## 2. Paket sorumlulukları

| Paket | Sorumluluk | Bağımlılık |
|---|---|---|
| `birkat-core` | Alan modeli, içerik durumu kuralları, iş akışı motoru, kapsam uyarıları | yok (saf Rust) |
| `birkat-crypto` | AES-256-GCM, Argon2id, anahtar sarma, deneme kilidi | core |
| `birkat-audit` | Hash zincirli işlem geçmişi | core |
| `birkat-evidence` | Hash, tür tespiti, arşiv güvenliği, edinim, aygıt, yazma koruması, EWF | core, `ewf-image` |
| `birkat-audio` | Ses sondajı, çözümleme, dalga biçimi, spektrogram, oynatma, envanter | core, `symphonia` |
| `birkat-transcription` | Whisper/Candle, kuyruk, kalite, yazım standardı, model paketi | core, audio, `candle` (opsiyonel) |
| `birkat-udf` | UDF okuyucu/yazıcı/doğrulayıcı | core, `zip`, `quick-xml` |
| `birkat-report` | Kanonik rapor modeli, UDF köprüsü, şablonlar, PDF/DOCX/TXT/JSON, önizleme | core, udf, `printpdf` |
| `birkat-backup` | Şifreli yedek, geri yükleme, saklama | core, crypto |
| `birkat-updater` | İmzalı güncelleme, ön yedek, geri dönüş | core |
| `birkat-app` | Vaka deposu (SQLite), kasa, panel, otomatik iş akışı, ayarlar | tüm alan paketleri |
| `birkat-cli` | Komut satırı | app |
| `birkat-desktop` | Masaüstü arayüz | app |

## 3. Bağımlılık yönü

Bağımlılıklar tek yönlüdür; döngü yoktur. `birkat-core` hiçbir pakete bağımlı
değildir. Bu, alan modelinin bağımsız test edilmesini ve başka bir sunum
katmanıyla yeniden kullanılmasını sağlar.

## 4. Veri akışı

### 4.1 Otomatik iş akışı (şartname 2.2)

```text
Kullanıcı: yeni vaka + materyal yolu
        │
        ▼
birkat_app::run_automatic_workflow
        │
        ├─ 1. Materyalleri kasaya al .................. birkat_app::vault
        ├─ 2. Ses dosyalarını tespit et .............. birkat_audio::inventory
        ├─ 3. Teknik envanteri çıkar ................. birkat_audio::probe
        ├─ 4. Transkripsiyon kuyruğunu kur ............ birkat_transcription::queue
        ├─ 5. Her dosyayı metne dönüştür ............. birkat_transcription::engine
        ├─ 6. Otomatik kalite kontrolü ................ birkat_transcription::quality
        ├─ 7. Şüpheli bölümleri işaretle .............. quality::review_priorities
        ├─ 8. Transkriptleri kasaya yaz ................ vault::store_bytes
        ├─ 9. Rapor taslağını kur ..................... birkat_report::builder
        ├─ 10. UDF'yi üret ............................ birkat_report::report_to_udf
        └─ 11. Son kontrol listesini oluştur
        │
        ▼
    WorkflowOutcome (özet, uyarılar, UDF, kontrol listesi)
```

Her adım işlem geçmişine kaydedilir. İş akışı durumu, `path_to` fonksiyonuyla
bulunan yol üzerinden adım adım ilerler; böylece her ara durum denetlenebilir
kalır.

### 4.2 Rapor üretimi (şartname 14)

```text
Case + ExpertProfile[] + ReportBuildInput
        │
        ▼
birkat_report::build_report
        │
        ▼
ReportDocument  ◄── TEK DOĞRULUK KAYNAĞI
        │
        ├─► birkat_report::report_to_udf ─► UdfDocument ─► birkat_udf::write_udf ─► .udf (BİRİNCİL)
        ├─► birkat_report::pdf::report_to_pdf ─────────────────────────────────► .pdf
        ├─► birkat_report::docx::report_to_docx ───────────────────────────────► .docx
        └─► birkat_report::text::{to_text,to_json,to_csv,to_srt,to_vtt} ───────► .txt/.json/.csv/.srt/.vtt
```

UDF, PDF ve DOCX için ayrı rapor yazma sistemleri **yoktur**; tüm çıktılar
aynı `ReportDocument` verisinden üretilir.

## 5. Teknoloji kararları ve gerekçeleri

| Karar | Seçim | Gerekçe |
|---|---|---|
| Dil | Rust | Bellek güvenliği, tek ikili dağıtım, Windows'ta yerel çalışma, `#![forbid(unsafe_code)]` |
| Arayüz | egui/eframe | Saf Rust, GPU/CPU render, Windows'ta ek çalışma zamanı gerektirmez |
| Konuşma tanıma | Candle + Whisper large-v3-turbo | Yerel çıkarım, çevrimdışı, saf Rust, CPU zorunlu + GPU opsiyonel |
| Ses çözümleme | Symphonia | Saf Rust, WAV/MP3/AAC/FLAC/OGG/MP4/MKV kapsayıcıları |
| Delil imajı | `ewf-image` | Gerçek EWF (E01/L01/Ex01/Lx01) okuma ve yazma; Apache-2.0 |
| Veritabanı | SQLite (`rusqlite`, bundled) | Yerel, tek dosya, WAL + `synchronous=FULL` ile dayanıklı |
| Şifreleme | AES-256-GCM + Argon2id | Doğrulanmış AEAD; standart anahtar türetme |
| UDF paketleme | `zip` (Deflate) + `quick-xml` | UDF, ZIP içinde `content.xml` taşır |
| PDF | `printpdf` + gömülü DejaVu alt kümesi | Türkçe karakter desteği; saf Rust |
| Anahtar bellekten silme | `zeroize` | Anahtar yaşam süresi kontrolü |

### İstenmeyen teknolojiler (şartname 20.3)

Kullanılmadığı doğrulanmıştır: Electron, Tauri, Node.js çalışma zamanı,
Python çalışma zamanı, bulut ASR/OCR, kullanıcı tarafından kurulan FFmpeg,
Cargo derlemesi gerektiren teslim, ilk çalıştırmada model indirme.

## 6. Hata yönetimi

Tek bir hata tipi (`birkat_core::error::CoreError`) tüm alan katmanında
kullanılır. Kategoriler:

- `NotFound` — Kayıt bulunamadı.
- `Validation` — Doğrulama hatası.
- `InvalidTransition` — Geçersiz durum geçişi.
- `ContentStatusRule` — İçerik durumu kuralı ihlali (A.2).
- `ScopeWarning` — Kapsam uyarısı.
- `Data` — Biçim/çözümleme hatası.
- `Io` — Girdi/çıktı hatası.

Tüm kullanıcıya görünen hata mesajları Türkçedir.

## 7. Dayanıklılık

| Senaryo | Mekanizma | Test |
|---|---|---|
| Elektrik kesintisi | SQLite WAL + `synchronous=FULL` | `store::tests::open_and_migrate` |
| Uygulama çökmesi | Kuyruk durumu diske yazılır; çalışırken kesilen görevler kuyruğa döner | `queue::tests::crash_recovery_requeues_running_tasks` |
| Disk dolması | Edinim ve yedek boyut sınırları | `backup::tests::max_bytes_is_enforced` |
| Bozuk dosya | Okunamayan dosya atlanmaz, neden bildirilir | `inventory::tests::unreadable_audio_is_reported_with_reason` |
| Yedek bozulması | Hash manifesti + AEAD etiketi | `backup::tests::tampered_backup_is_rejected` |
| Güncelleme başarısızlığı | Ön yedek + otomatik geri dönüş | `updater::tests::install_and_rollback` |
| Aynı işin tekrar çalışması | Tamamlanmış görev yeniden kuyruğa alınmaz | `queue::tests::completed_task_is_not_duplicated` |
| Veritabanı migrasyonu | Şema sürümü `meta` tablosunda | `store::tests::open_and_migrate` |
| Büyük dosyalar | Akış hâlinde hash; parça bazlı transkripsiyon | `hash::tests::file_hashing_matches_in_memory_hashing` |

## 8. Performans notları

- Hash hesaplama 1 MiB bloklarla akış hâlinde yapılır; dosya tamamen belleğe
  yüklenmez.
- Ses çözümleme Symphonia ile paket paket yapılır.
- Transkripsiyon 30 saniyelik parçalara ayrılır; 2 saniye örtüşme ile parça
  sınırında kelime kaybı önlenir.
- UDF metin havuzu ve ofsetler tek geçişte üretilir; karakter sayımı
  `chars().count()` ile yapılır (bayt değil).

## 9. Sürümleme

- Uygulama sürümü: `birkat_core::APP_VERSION` (Cargo.toml).
- Veri modeli sürümü: `birkat_core::MODEL_VERSION`. Yeni alanlar eklendiğinde
  artırılır ve göç yordamı yazılır.
- UDF biçim sürümü: `format_id="1.8"`.
- Yedek biçim sürümü: `BACKUP_FORMAT_VERSION`.
- Güncelleme manifestosu sürümü: `UPDATE_MANIFEST_VERSION`.

## 10. Modül haritası (şartname 20.4)

Şartnamede önerilen modül yapısı, Rust paketlerine ve modüllerine şöyle
eşlenir:

| Şartname modülü | Gerçek konum |
|---|---|
| `desktop` | `birkat-desktop` |
| `application` | `birkat-app` |
| `user-profile` | `birkat-core::profile`, `birkat-app::store` |
| `case-management` | `birkat-core::case`, `birkat-app::workspace` |
| `assignment-reader` | `birkat-evidence::detect`, `birkat-udf::reader` |
| `document-reader` | `birkat-evidence::detect` |
| `local-ocr` | (kanca hazır; motor paketlenmedi) |
| `task-scope` | `birkat-core::scope` |
| `question-manager` | `birkat-core::case` |
| `calendar-and-fees` | `birkat-core::admin`, `birkat-core::case::Deadline` |
| `evidence-vault` | `birkat-app::vault` |
| `device-detection` | `birkat-evidence::device` |
| `write-protection` | `birkat-evidence::write_protection` |
| `evidence-acquisition` | `birkat-evidence::acquisition` |
| `file-acquisition` | `birkat-evidence::acquisition` |
| `ewf-reader-writer` | `birkat-evidence::ewf` |
| `hash-verification` | `birkat-evidence::hash` |
| `audit-history` | `birkat-audit` |
| `audio-inventory` | `birkat-audio::inventory`, `probe` |
| `audio-decoding` | `birkat-audio::decode` |
| `audio-playback` | `birkat-audio::playback` |
| `audio-review` | `birkat-audio::waveform`, `spectrogram`, arayüz ekranı |
| `audio-processing` | `birkat-audio::processing` |
| `transcription-engine` | `birkat-transcription::engine`, `whisper_candle` |
| `model-runtime` | `birkat-transcription::whisper_candle` |
| `model-package` | `birkat-transcription::model_package` |
| `transcription-queue` | `birkat-transcription::queue` |
| `transcription-quality` | `birkat-transcription::quality`, `hallucination` |
| `transcript-editor` | arayüz ekranı + `birkat-audio::playback` |
| `transcript-versioning` | `birkat-core::audio::TranscriptVersion` |
| `findings` | `birkat-core::findings` |
| `evidence-references` | `birkat-core::findings::EvidenceReference` |
| `report-model` | `birkat-core::report` |
| `report-templates` | `birkat-report::templates` |
| `udf-reader` | `birkat-udf::reader` |
| `udf-writer` | `birkat-udf::writer` |
| `udf-validator` | `birkat-udf::validator` |
| `pdf-export` | `birkat-report::pdf` |
| `docx-export` | `birkat-report::docx` |
| `attachment-manager` | `birkat-report::auxiliary` |
| `signature-handoff` | `birkat-core::report::SignatureRecord` |
| `uyap-workflow` | `birkat-report::auxiliary`, arayüz ekranı |
| `delivery-manager` | `birkat-core::report::DeliveryRecord` |
| `archive-manager` | `birkat-app::workspace::archive` |
| `backup-restore` | `birkat-backup` |
| `security` | `birkat-crypto`, `birkat-app::settings` |
| `updater` | `birkat-updater` |
| `logging` | `tracing` |
| `diagnostics` | `birkat-app::settings::check_health` |
| `test-data` | Paket içi test yardımcıları (`probe::tests::write_test_wav`) |
