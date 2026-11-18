# AGENTS.md — BİRKÂT geliştirici notları

Bu dosya, gelecek oturumlarda bağlamı hızla geri kazanmak için tutulur.

## Proje kimliği

- Ürün: **BİRKÂT** — 56.12 Bilirkişi Dijital Çalışma Operatörü
- Hedef: Windows 11 x64, yerel ve çevrimdışı Rust masaüstü uygulaması
- Birincil rapor biçimi: **UYAP UDF** (opsiyonel dışa aktarma değil)
- Depo: <https://github.com/azmisahin-gov/birkat>

## Derleme ve test komutları

```bash
# Tüm paketler
cargo build --workspace --all-targets

# Whisper/Candle yerel transkripsiyon motoru ile
cargo build -p birkat-transcription --features whisper

# Testler (490 test, 0 başarısız)
cargo test --workspace
cargo test -p birkat-transcription --features whisper   # +62 test

# Kod kalitesi (ikisi de 0 uyarı vermeli)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets

# Masaüstü arayüzü (görüntü sunucusu gerekir)
cargo run -p birkat-desktop -- ./birkat-veri

# Komut satırı
cargo run -p birkat-cli -- --veri ./birkat-veri surum

# SBOM yeniden üret
cargo metadata --format-version 1 --all-features > /tmp/meta.json
python3 scripts/generate-sbom.py < /tmp/meta.json
```

## Paket haritası

| Paket | Sorumluluk |
|---|---|
| `birkat-core` | Alan modeli, içerik durumu, iş akışı, kapsam uyarıları. **Bağımlılığı yok.** |
| `birkat-crypto` | AES-256-GCM, Argon2id, anahtar sarma, deneme kilidi |
| `birkat-audit` | Hash zincirli işlem geçmişi |
| `birkat-evidence` | Hash, tür tespiti, arşiv güvenliği, edinim, aygıt, yazma koruması, EWF |
| `birkat-audio` | Sondaj, çözümleme, dalga biçimi, spektrogram, oynatma, envanter |
| `birkat-transcription` | Whisper/Candle, kuyruk, kalite kontrolü, model paketi |
| `birkat-udf` | UDF okuyucu/yazıcı/doğrulayıcı |
| `birkat-report` | Kanonik rapor modeli, UDF köprüsü, PDF/DOCX/TXT/JSON, önizleme |
| `birkat-backup` | Şifreli yedek, geri yükleme, saklama |
| `birkat-updater` | İmzalı güncelleme, ön yedek, geri dönüş |
| `birkat-app` | SQLite vaka deposu, şifreli kasa, panel, otomatik iş akışı |
| `birkat-cli` | Komut satırı |
| `birkat-desktop` | egui/eframe arayüzü (20 çalışma alanı) |

## Kritik kurallar (bunları asla bozmayın)

Ürünün **dürüstlük sözleşmesi**. Her biri en az bir testle korunuyor:

1. **Makine çıktısı doğrudan onaylanamaz.** `content_status::apply_transition`
   bu kuralı zorlar; test: `machine_output_cannot_be_approved_directly`.
2. **Yazılımsal yazma koruması donanımsal değildir.**
3. **Dosya kopyası E01/L01 olarak adlandırılmaz.**
4. **Model güven göstergesi doğruluk yüzdesi değildir.**
5. **Kelime düzeyi zaman kodu yoksa sahte zaman üretilmez.**
6. **Konuşmacı etiketi gerçek kişi kimliği değildir.**
7. **Hash zinciri bağımsız zaman damgası değildir.**
8. **UDF birincil çıktıdır; PDF/DOCX yardımcıdır ve yerine geçmez.**
9. **İmzasız güncelleme paketi uygulanmaz.**
10. **Buluta hiçbir vaka verisi gönderilmez.**
11. **İmzalı rapor değiştirilemez** (`ensure_editable`).
12. **UYAP Editör önizlemesi, gerçek Editör doğrulamasının yerine geçmez.**
13. **Desteklenmeyen codec/biçim sessizce atlanmaz**, açık hata verir.
14. **Kaynakta olmayan metadata üretilmez.**

Her yeni özellik için bu kurallardan hangisine dokunduğunu kontrol edin ve
gerekiyorsa dürüstlük testi ekleyin.

## Kod stili

- Tüm kullanıcıya görünen metin **Türkçe** (arayüz, hata mesajları, belgeler).
- Değişken/fonksiyon adları İngilizce; dokümantasyon Türkçe.
- Her pakette `#![forbid(unsafe_code)]` ve `#![warn(missing_docs)]`.
- Genel (public) her öğe Türkçe doc yorumu taşır.
- Yorum yalnızca **neden** açıklanıyorsa yazılır; kodun ne yaptığını
  tekrarlayan yorum yazılmaz.
- Testler gerçek dosya üretir ve geri okur; mock kullanılmaz (MockEngine
  yalnızca arayüz akışı için).

## Bilinen sınırlar (bu ortamda test edilemedi)

Bunlar "tamamlandı" olarak işaretlenmemelidir:

- Windows'a özgü: aygıt algılama, yazma koruması uygulaması, fiziksel sektör okuma.
- Whisper model dosyaları telifli olduğundan depoda yok; gerçek çıkarım ölçülmedi.
- Resmî UYAP Editör ile UDF uyumluluk testi yapılmadı (prosedür `docs/udf-uyumluluk.md`).
- OCR motoru paketlenmedi.
- 7z/RAR arşivleri açılmaz (açık hata verir).
- Görüntü sunucusu olmayan ortamda masaüstü arayüzü açılamaz.
- Ed25519 imza doğrulaması üretimde etkinleştirilmeli.

## Whisper/Candle çıkarımı — üç kritik kural

Gerçek transkripsiyon bu üçü yanlışsa **sessizce bozulur**; hata vermez,
yalnızca anlamsız metin üretir:

1. **Sabit 30 saniyelik pencere.** Kodlayıcı tam `m::N_FRAMES` (3000) mel
   karesi bekler. Kısa kayıt sıfırla doldurulur; candle'ın `pcm_to_mel`
   fonksiyonu ek dolgu kareleri eklediği için `narrow(2, 0, m::N_FRAMES)` ile
   kırpılır. Aksi hâlde `narrow invalid args` veya anlamsız çıktı.
2. **`final_linear` zorunlu.** `decoder.forward()` **gizli durum** döner,
   sözcük dağarcığı logitleri değil. Logitler için
   `decoder.final_linear(&hidden.i((.., seq-1.., ..)))` uygulanır. Doğrudan
   argmax alınırsa çıktı tek harfe düşer ("Merhaba" → "Mer").
3. **KV önbelleği.** İlk adımda istem belirteçlerinin tamamı ve
   `flush_cache=true`; sonraki adımlarda yalnızca son belirteç ve
   `flush_cache=false`. Her adımda tam diziyi beslemek doğrudur ama çok
   yavaştır.

Doğrulama ikilisi: `cargo run -p birkat-transcription --features whisper
--example whisper-dogrula -- --model … --config … --tokenizer … --mel …
--ses kayit.wav --referans metin.txt --dil tr`

Model dosyaları: `model.safetensors`, `config.json`, `tokenizer.json`,
`melfilters128.bytes` (resmî mel filtresi; analitik hesaplama birebir değil).
Hash'ler `model-package.lock.json` içinde sabit.

## Windows çapraz derleme ve paketleme

```bash
# Araçlar (bir kez)
sudo apt-get install -y mingw-w64 nsis zip
rustup target add x86_64-pc-windows-gnu

# Paket (model ZORUNLU; eksikse hata verir)
./scripts/package-windows.sh --model-dir /yol/model
```

- `.cargo/config.toml` linker'ı `x86_64-w64-mingw32-gcc` olarak ayarlar.
- Üretilen `.exe` yalnızca Windows sistem DLL'lerine bağlı olmalı;
  `libgcc`/`libwinpthread`/`libstdc` görünürse paket reddedilir.
- NSIS'te `SetCompressor zlib` kullanılır: LZMA yoğun f16 ağırlıklarda
  saatler sürüp yalnızca %9 kazanç sağlar.
- NSIS mutlak yol ister (`PKG_ABS`); göreli yol lisans dosyasını bulamaz.
- **`set -euo pipefail` tuzağı:** `cmd | grep -q` erken çıkınca SIGPIPE
  (141) oluşur ve denetim yanlışlıkla başarısız görünür. Çıktıyı dosyaya
  alıp `grep -c` kullanın.

## Güvenlik denetimi

`cargo audit` **0 açık** vermeli. `zip` 9.x, `quick-xml` 0.42, `rusqlite`
0.43 ve `candle` 0.11 kullanılır (eski sürümler RustSec bildirimleri taşır;
candle 0.8 vulnerable `zip 1.3.1` çekiyordu). Kalan 3 "unmaintained" uyarısı
`.cargo/audit.toml` içinde gerekçesiyle kabul edilmiştir — gizlenmemiştir.

## CI

- `ci.yml`: Linux + Windows test, `clippy -D warnings`, `fmt --check`,
  `cargo audit`, SBOM güncellik denetimi. **4/4 yeşil olmalı.**
- `release.yml`: etikette Windows kurulum paketini üretir ve Releases'e
  yükler. Model `MODEL_URL` değişkeninden (veya elle verilen girdiden, yoksa
  depoda yayımlanmış varsayılan paketten) indirilir ve SHA-256 doğrulanır.
- SBOM bayatlarsa CI kırmızı olur: `cargo metadata --format-version 1
  --all-features > /tmp/m.json && python3 scripts/generate-sbom.py < /tmp/m.json`

## Faydalı referanslar

- UDF biçimi: `docs/udf-uyumluluk.md` (ZIP + `content.xml`, karakter ofsetleri)
- Gereksinim izlenebilirliği: `docs/gereksinim-matrisi.md`
- Kabul kriterleri: `docs/kabul-testi-matrisi.md`
- `ewf-image` API'si: `Image::open`, `Image::cursor`, `EwfWriter::create`
- printpdf: `PdfDocument::new` → `add_font` → `with_pages` → `save`
- UDF ofset kuralı: **karakter (rune)**, bayt değil; havuz sırası
  üstbilgi → gövde → altbilgi.
