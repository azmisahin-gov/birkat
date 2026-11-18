# BİRKÂT

**56.12 Bilirkişi Dijital Çalışma Operatörü**

BİRKÂT, 56.12 — *Ses Kayıtlarının Metin Hâline Dönüştürülmesi* alanında görev
yapan bilirkişiler için geliştirilmiş, **yerel ve çevrimdışı çalışabilen** bir
Windows masaüstü uygulamasıdır. Görevlendirmenin alınmasından raporun UYAP UDF
biçiminde hazırlanıp teslim sürecinin tamamlanmasına ve dosyanın arşivlenmesine
kadar olan dijital çalışma yükünü üstlenir.

> **Ürünün temel vaadi:** Bilirkişi teknik işlerle değil, bilirkişilik işiyle
> uğraşır.

---

## İçindekiler

1. [Ne yapar](#ne-yapar)
2. [Ne yapmaz](#ne-yapmaz)
3. [Mimari](#mimari)
4. [Kurulum ve çalıştırma](#kurulum-ve-çalıştırma)
5. [Uçtan uca akış](#uçtan-uca-akış)
6. [Komut satırı arayüzü](#komut-satırı-arayüzü)
7. [Test](#test)
8. [Belgeler](#belgeler)
9. [Bilinen sınırlar](#bilinen-sınırlar)

---

## Ne yapar

| Alan | İşlev |
|---|---|
| Görevlendirme | PDF/UDF/DOCX/RTF/ODT/TXT/görüntü ve arşiv kabulü; gerçek dosya imzası denetimi; arşiv bombası ve yol kaçışı koruması |
| Görev analizi | Merci, dosya no, taraflar, süre, inceleme konusu, sorular ve materyal listesinin çıkarımı; kapsam dışı talep uyarısı ve açıklama taslağı |
| Delil yönetimi | Dosya bazlı edinim + SHA-256 doğrulaması; aygıt tanımlama; yazma koruması planı; gerçek E01/L01/Ex01/Lx01 okuma ve E01/Ex01 yazma; RAW imaj |
| Ses | WAV/BWF/MP3/M4A/AAC/FLAC/OGG/Opus/AIFF/CAF/MP4/MOV/MKV/WebM tespiti ve teknik envanteri; dalga biçimi; spektrogram; oynatma denetimi |
| Transkripsiyon | Yerel Whisper large-v3-turbo (Candle); otomatik kuyruk; kesinti sonrası devam; otomatik kalite kontrolü (16 kontrol) |
| Transkript | Birebir (verbatim) yaklaşım; belirsizlik işaretleri; geçici konuşmacı etiketleri; 5 aşamalı sürüm koruması |
| Rapor | Tek kanonik veri modelinden **doğrudan UDF** üretimi; UDF okuyucu ve doğrulayıcı; PDF/DOCX/TXT/JSON/SRT/VTT yardımcı çıktıları |
| Yardımcı belgeler | Ek rapor, süre uzatım talebi, eksik materyal talebi, kapsam açıklama talebi, teslim/iade tutanağı, dizi pusulası, teslim kontrol listesi |
| Güvenlik | AES-256-GCM şifreli kasa; Argon2id anahtar türetme; şifreli yedek + gerçek geri yükleme testi; hash zincirli işlem geçmişi |
| Güncelleme | İmzalı manifest; paket bütünlüğü; imza kapısı; ön yedek; otomatik geri dönüş |

## Ne yapmaz

Bu sınırlar ürünün **dürüstlük sözleşmesidir**; teknik eksiklik değildir.

- Konuşmacının **biyometrik kimliğini tespit etmez.**
- Bir kaydın montajlı, sahte veya değiştirilmiş olduğunu **hükme bağlamaz.**
- Sesin **tarihsel özgünlüğünü** belirlemez.
- **Deepfake** tespiti konusunda kesin adli kanaat oluşturmaz.
- Kişinin **niyetini, duygusunu veya yalan söyleyip söylemediğini** belirlemez.
- **Hukuki nitelendirme** veya suçluluk değerlendirmesi yapmaz.
- **Silinmiş verileri kurtarmaz**; genel dijital delil incelemesi yapmaz.
- 56.11 kapsamındaki adli ses/görüntü incelemesini 56.12 işlemi gibi **sunmaz.**
- Yazılımsal yazma korumasını **donanımsal korumaya eşdeğer göstermez.**
- Dosya bazlı kopyayı **E01/L01 olarak adlandırmaz.**
- Makine transkriptini **insan tarafından doğrulanmış göstermez.**
- Model güven göstergesini **doğruluk yüzdesi olarak sunmaz.**
- Kelime düzeyi zaman kodu yoksa **sahte kesinlik üretmez.**
- Konuşmacı etiketini **gerçek kişi kimliği gibi göstermez.**
- Yapay zekâ **bilirkişi yerine mesleki kanaat oluşturmaz.**
- UYAP'a **yetkisiz otomatik giriş** yapmaz; sahte e-imza oluşturmaz.
- Veriyi, raporu, sesi veya transkripti **buluta göndermez.**

Bu talepler görevlendirmede görülürse uygulama **kapsam uyarısı** verir ve
bilirkişinin görev veren merciden açıklama istemesine yardımcı olacak taslağı
hazırlar.

---

## Mimari

Uygulama on üç Rust paketinden oluşur. Her paket bağımsız test edilebilir ve
bağımlılığı düşüktür.

```text
crates/
  birkat-core            Alan modeli, içerik durumu kuralları, iş akışı motoru
  birkat-crypto          AES-256-GCM, Argon2id, anahtar sarma, deneme kilidi
  birkat-audit           Hash zincirli işlem geçmişi
  birkat-evidence        Hash, tür tespiti, güvenli arşiv, edinim, aygıt,
                         yazma koruması, gerçek EWF imaj desteği
  birkat-audio           Ses sondajı, çözümleme, dalga biçimi, spektrogram,
                         oynatma denetimi, envanter, türetilmiş ses
  birkat-transcription   Whisper/Candle motoru, iş kuyruğu, kalite kontrolü,
                         yazım standardı, halüsinasyon tespiti, model paketi
  birkat-udf             UDF okuyucu, yazıcı ve doğrulayıcı
  birkat-report          Kanonik rapor modeli, UDF köprüsü, şablonlar,
                         PDF/DOCX/TXT/JSON/SRT/VTT, yardımcı belgeler, önizleme
  birkat-backup          Şifreli yedek, gerçek geri yükleme testi, saklama
  birkat-updater         İmzalı güncelleme, ön yedek, geri dönüş
  birkat-app             Vaka deposu (SQLite), şifreli kasa, panel,
                         otomatik iş akışı, ayarlar
  birkat-cli             Komut satırı arayüzü
  birkat-desktop         egui/eframe masaüstü arayüzü (20 çalışma alanı)
```

Veri akışı:

```text
Görevlendirme belgesi ─┐
Materyal (ses/dosya)  ─┼─► birkat-app (kasa + depo)
                       │        │
                       │        ├─► birkat-evidence  (edinim, hash, imaj)
                       │        ├─► birkat-audio     (envanter, çözümleme)
                       │        ├─► birkat-transcription (metin + kalite)
                       │        └─► birkat-report ──► birkat-udf  (UDF)
                       │                    │
                       │                    └─► PDF / DOCX / TXT / JSON
                       └─► birkat-audit (hash zinciri) ◄─ her adım kaydedilir
```

Ayrıntı: [`docs/teknik-mimari.md`](docs/teknik-mimari.md)

---

## Kurulum ve çalıştırma

### Gereksinimler

- Windows 11 x64 (ana destek platformu)
- Rust 1.82 veya üzeri (yalnızca kaynaktan derleme için)

### Derleme

```bash
# Tüm paketler (varsayılan: Whisper/Candle motoru hariç)
cargo build --release

# Whisper/Candle yerel transkripsiyon motoru ile
cargo build --release --features whisper -p birkat-transcription

# Masaüstü arayüzü
cargo run --release -p birkat-desktop -- ./birkat-veri

# Komut satırı
cargo run --release -p birkat-cli -- --veri ./birkat-veri surum
```

### Kurulum paketi

```bash
# Windows x64 paketi üretir (NSIS/WiX betikleri ve model paketi ile)
./scripts/package-windows.sh
```

Ayrıntı: [`docs/kurulum-kilavuzu.md`](docs/kurulum-kilavuzu.md)

---

## Uçtan uca akış

Aşağıdaki akış gerçek verilerle doğrulanmıştır:

```bash
B=birkat
V=./birkat-veri

# 1. Bilirkişi profili
$B --veri $V profil-ekle --ad "Ayşe Yılmaz" --unvan "Dr." --sicil 12345

# 2. Vaka
$B --veri $V vaka-olustur --baslik "2025/123" \
     --mahkeme "Ankara 5. Asliye Ceza Mahkemesi" \
     --dosya-no "2025/123 Esas" --konu "Ses kaydının metne dönüştürülmesi"

# 3. Materyal + otomatik transkripsiyon + kalite kontrolü + rapor taslağı
$B --veri $V transkript --vaka <VAKA_ID> --yol ./materyal

# 4. Rapor + UDF (birincil çıktı) ve yardımcı çıktılar
$B --veri $V rapor --vaka <VAKA_ID> --cikti ./cikti

# 5. UDF doğrulama
$B --veri $V udf-dogrula --dosya ./cikti/BILIRKISI_RAPORU.udf

# 6. Şifreli yedek + gerçek geri yükleme testi
$B --veri $V yedek --vaka <VAKA_ID> --parola "Guvenli-Yedek-2026" \
     --cikti ./yedek/vaka.yedek

# 7. İşlem geçmişi bütünlüğü
$B --veri $V denetim-dogrula
```

---

## Komut satırı arayüzü

| Komut | İşlev |
|---|---|
| `surum` | Sürüm, uzmanlık kodu ve veri modeli sürümü |
| `profil-ekle`, `profiller` | Bilirkişi profili yönetimi |
| `vaka-olustur`, `vakalar` | Vaka yönetimi |
| `materyal-ekle` | Materyali kasaya alır (isteğe bağlı şifreli) |
| `transkript` | Otomatik iş akışı: envanter → transkripsiyon → kalite → rapor taslağı |
| `rapor` | Rapor + UDF + PDF/DOCX/TXT/JSON üretimi |
| `udf-dogrula`, `udf-oku` | UDF doğrulama ve okuma |
| `ses-incele` | Ses dosyası teknik envanteri |
| `yedek`, `geri-yukle` | Şifreli yedek ve geri yükleme |
| `denetim-dogrula` | İşlem geçmişi hash zinciri doğrulaması |
| `saglik`, `panel` | Sistem sağlığı ve ana kontrol paneli |

---

## Test

```bash
# Tüm testler
cargo test --workspace

# Whisper özelliği ile birlikte
cargo test --workspace --features whisper -p birkat-transcription

# Kapsam raporu (cargo-llvm-cov gerekir)
cargo llvm-cov --workspace --html
```

Test türleri:

- **Birim testleri:** Her pakette alan mantığı, sınır durumları ve hata yolları.
- **Entegrasyon testleri:** Gerçek dosya üretimi ve geri okuma (WAV, E01, UDF, PDF, DOCX, yedek).
- **Uçtan uca test:** `birkat-cli` ile tam kabul senaryosu.

Test raporu: [`docs/test-raporu.md`](docs/test-raporu.md)

---

## Belgeler

| Belge | İçerik |
|---|---|
| [`docs/teknik-mimari.md`](docs/teknik-mimari.md) | Paket yapısı, veri akışı, teknoloji kararları |
| [`docs/veri-modeli.md`](docs/veri-modeli.md) | Varlıklar, durumlar, sürümleme |
| [`docs/guvenlik-modeli.md`](docs/guvenlik-modeli.md) | Tehdit modeli, şifreleme, anahtar yönetimi |
| [`docs/udf-uyumluluk.md`](docs/udf-uyumluluk.md) | UDF biçimi, doğrulama, UYAP Editör testi |
| [`docs/asr-benchmark.md`](docs/asr-benchmark.md) | Türkçe transkripsiyon test seti ve ölçümler |
| [`docs/e01-l01-uyumluluk.md`](docs/e01-l01-uyumluluk.md) | EWF imaj desteği ve doğrulama |
| [`docs/ses-format-matrisi.md`](docs/ses-format-matrisi.md) | Desteklenen kapsayıcı/codec matrisi |
| [`docs/kurulum-kilavuzu.md`](docs/kurulum-kilavuzu.md) | Windows kurulumu |
| [`docs/kullanici-kilavuzu.md`](docs/kullanici-kilavuzu.md) | Bilirkişi kullanım kılavuzu |
| [`docs/gereksinim-matrisi.md`](docs/gereksinim-matrisi.md) | Gereksinim → modül → test eşlemesi |
| [`docs/kabul-testi-matrisi.md`](docs/kabul-testi-matrisi.md) | 49 maddelik kabul senaryosu |
| [`docs/test-raporu.md`](docs/test-raporu.md) | Test sonuçları |
| [`docs/bagimlilik-lisanslari.md`](docs/bagimlilik-lisanslari.md) | Bağımlılık lisansları |
| [`docs/sbom.cdx.json`](docs/sbom.cdx.json) | CycloneDX SBOM |
| [`docs/surum-ve-guncelleme.md`](docs/surum-ve-guncelleme.md) | Sürüm ve güncelleme süreci |

---

## Bilinen sınırlar

Bu sınırlar **açıkça belgelenmiştir**; gizlenmez.

1. **Windows'a özgü işlevler.** Aygıt algılama, yazma koruması uygulaması ve
   fiziksel sektör okuma Windows API'lerine dayanır. Bu işlevlerin platform
   katmanı Windows'ta gerçek donanımla test edilmelidir; kitaplık katmanı
   platformdan bağımsız kural ve durum mantığını taşır ve test edilmiştir.
2. **UYAP Editör uyumluluğu.** UDF biçimi, açık kaynak dönüştürücülerin
   belgelerine ve gerçek UDF yapısına göre uygulanmıştır. Nihai uyumluluk
   kontrolü, Adalet Bakanlığı'nın resmî UYAP Editör'ünde yapılmalıdır
   (bkz. `docs/udf-uyumluluk.md`).
3. **GPU hızlandırması.** CPU üzerinde çalışmak zorunludur ve CPU yolu
   uygulanmıştır. CUDA/Metal desteği derleme zamanında etkinleştirilebilir;
   başlatılamazsa CPU'ya düşülür. Windows GPU performansı gerçek sistemde
   ölçülmelidir.
4. **7z ve RAR arşivleri.** Bu biçimler açık bir hata ile bildirilir; sessizce
   atlanmaz. ZIP, DOCX, ODT ve UDF arşivleri güvenli biçimde açılır.
5. **Gömülü model.** Whisper model dosyaları boyutu nedeniyle depoda
   bulunmaz; kurulum paketine eklenir ve hash ile doğrulanır
   (bkz. `docs/kurulum-kilavuzu.md`).
6. **Görüntü sunucusu olmayan ortamlar.** Masaüstü arayüzü başlatılabilmesi
   için bir görüntü sunucusu (X11/Wayland/Windows) gerektirir.

---

## Lisans

Apache-2.0. Bağımlılık lisansları için bkz.
[`docs/bagimlilik-lisanslari.md`](docs/bagimlilik-lisanslari.md).
