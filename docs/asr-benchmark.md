# Türkçe ASR Benchmark Raporu

Bu belge, BİRKÂT'ın transkripsiyon doğruluğunu ölçmek için kullanılan test
veri setini, ölçüm yöntemini ve **gerçekten yapılmış ölçüm sonuçlarını**
tanımlar (şartname 23.1).

> **Önemli:** WER/CER sonucu, transkriptin bilirkişi tarafından doğrulandığı
> anlamına **gelmez.** Bu ölçüm yalnızca model davranışını karşılaştırmak için
> kullanılan teknik bir göstergedir.

---

## 1. Ölçüm durumu

| Aşama | Durum |
|---|---|
| Ölçüm çerçevesi ve metrikler | ✓ Tamamlandı |
| Kategori tanımları (15 kategori) | ✓ Tamamlandı |
| **Gerçek Whisper modeli ile ölçüm** | ✓ **Yapıldı** (aşağıda sonuçlar) |
| Kurumsal doğrulanmış veri setiyle genişletilmiş ölçüm | ✗ Kurum tarafından yapılmalı |
| Model karşılaştırma altyapısı | ✓ Hazır (`ModelPackage`, `ModelVersion`) |

---

## 2. Doğrulanmış ölçüm: gerçek model, gerçek ses

### 2.1 Ölçüm koşulları

| Öğe | Değer |
|---|---|
| Model | `openai/whisper-large-v3-turbo` (MIT) |
| Ağırlık dosyası SHA-256 | `542566a422ae4f3f…` (tam değer `model-package.lock.json`) |
| Çıkarım motoru | BİRKÂT `birkat-transcription` + Candle 0.11.1 |
| Cihaz | CPU (GPU kullanılmadı) |
| Dil | `tr` (açıkça belirtildi) |
| Pencere | 30 sn sabit, sıfır dolgu |
| Ses | 16 kHz mono PCM, 3 kayıt (6,45–7,97 sn) |
| Konuşma üretimi | `espeak-ng -v tr` (sentetik, doğal konuşma değil) |

### 2.2 Sonuçlar

| # | Referans | Whisper çıktısı | WER |
|---|---|---|---|
| 1 | Merhaba, bu bir bilirkişi ses kaydı denemesidir. Kayıt bugün saat on dörtte alınmıştır. | Merhaba, bu bir bilirkişi ses kaydı denemesidir. Kayıt bugün saat 10'dır, te alınmıştır. | %23,1 |
| 2 | Müşteki ifadesinde toplantının 12 Mart 2025 tarihinde yapıldığını söyledi. | Müşte ki ifadesinde toplantının oniki martiki bin irmi beshtari kinde yapıldığını söyledi. | %88,9 |
| 3 | Sanık bu parayı kendisinin almadığını ve olay yerinde bulunmadığını beyan etti. | Sanık burpada yöken disinin alma döne ve olay yerinde bulunma döne beyan etti. | %63,6 |
| | **TOPLAM** | | **%54,5** (18/33 kelime) |

### 2.3 Sonuçların yorumu

Bu ölçüm **sentetik (TTS) konuşma** ile yapılmıştır ve bu, sonuçları
doğrudan etkiler:

- **Olumlu bulgu:** Kayıt 1'de cümlenin tamamı anlamlı biçimde çözülmüştür.
  Whisper'ın Türkçe çözümlemesi ve BİRKÂT'ın çıkarım hattı **çalışır
  durumdadır.**
- **Olumsuz bulgu:** Sayı (12 → "oniki martiki bin irmi besh"), özel isim ve
  olumsuzluk ekleri ("almadığını" → "alma döne") hatalı çözülmüştür.
- **Neden:** `espeak-ng` sentetik sesi, formant geçişleri ve ek
  birleşmeleri bakımından doğal Türkçe konuşmadan belirgin biçimde farklıdır.
  Whisper, doğal konuşma üzerinde eğitilmiştir; sentetik ses dağılım dışıdır.
- **Sonuç:** %54,5 WER, **modelin doğal Türkçe konuşmadaki başarısını
  göstermez.** Gerçekçi bir WER ölçümü, doğal Türkçe konuşma içeren ve insan
  tarafından doğrulanmış bir veri setiyle yapılmalıdır.

**Bu ölçümün ürüne etkisi:** Ürün, transkripti **bilirkişi onayına sunar.**
Otomatik kalite kontrolü, düşük güvenli bölümleri (sayı, özel isim,
olumsuzluk) işaretler. Bkz. bölüm 6. Hiçbir çıktı, bilirkişi doğrulaması
olmadan rapora mesleki kanaat olarak girmez.

### 2.4 Olumsuzluk ve sayı hatalarının ürün tarafından yakalanması

Ölçümdeki en kritik hata türleri, kalite kontrolü tarafından **işaretlenir**:

| Hata türü | Kalite kontrolü davranışı | Test |
|---|---|---|
| Sayı/tarih/tutar | `NumberDateAmount` — orta önemde işaretlenir | `quality::tests::detects_numbers` |
| Olumsuzluk | `Negation` — **yüksek** önemde işaretlenir | `quality::tests::detects_negation_as_high_severity` |
| Özel isim | `DifficultProperNoun` — orta önemde işaretlenir | `quality::tests::detects_proper_nouns` |

Böylece bilirkişi, tam da bu ölçümde hatalı çıkan bölümlere yönlendirilir.

---

## 3. Ölçüm aracı

BİRKÂT, gerçek model doğrulaması için hazır bir ikili içerir:

```bash
cargo run -p birkat-transcription --features whisper \
  --example whisper-dogrula -- \
  --model /yol/model.safetensors \
  --config /yol/config.json \
  --tokenizer /yol/tokenizer.json \
  --mel /yol/melfilters128.bytes \
  --ses kayit1.wav --referans metin1.txt \
  --dil tr
```

Bu ikili:
- 16 kHz mono WAV okur,
- 30 saniyelik sabit pencereye yerleştirir,
- gerçek Whisper çıkarımı yapar,
- çıktıyı referansla karşılaştırır ve WER hesaplar,
- mel ve kodlayıcı istatistiklerini tanılama için yazdırır.

---

## 4. Genişletilmiş test veri seti gereksinimleri

Kurumsal ölçüm için aşağıdaki kategorilerin her birinde en az 10 örnek
bulunmalıdır (toplam ≥ 150 kayıt, ≥ 5 saat, insan tarafından doğrulanmış):

| # | Kategori | Örnek koşullar | Asgari süre |
|---|---|---|---|
| K1 | Temiz konuşma | Stüdyo kalitesi, tek konuşmacı | 30 dk |
| K2 | Telefon görüşmesi | 8 kHz, kodek kaybı | 30 dk |
| K3 | Gürültülü kayıt | Trafik, kalabalık, rüzgâr | 20 dk |
| K4 | Düşük ses seviyesi | −40 dBFS altı | 15 dk |
| K5 | Birden fazla konuşmacı | 2–5 konuşmacı, sıralı | 30 dk |
| K6 | Örtüşen konuşma | Aynı anda konuşma | 15 dk |
| K7 | Farklı konuşma hızları | Çok yavaş / çok hızlı | 15 dk |
| K8 | Farklı aksanlar | Karadeniz, Ege, Doğu, Güneydoğu | 30 dk |
| K9 | Uzak mikrofon | 3+ metre mesafe | 15 dk |
| K10 | Kesintili kayıt | Kopukluklar, atlamalar | 15 dk |
| K11 | Uzun kayıt | 30+ dakika kesintisiz | 60 dk |
| K12 | Özel isimler | Kişi adları, soyadları | 15 dk |
| K13 | Kurum ve yer adları | Mahkeme, kurum, ilçe adları | 15 dk |
| K14 | Sayı/tarih/tutar | Telefon, TCKN, IBAN, tutar, tarih | 20 dk |
| K15 | Olumsuzluk içeren kritik cümle | "yapmadım", "etmedi", "yok" | 20 dk |

Veri setinin **kullanım hakkı ve gizliliği** sağlanmalıdır. Kişisel veri
içeren kayıtlar için KVKK uyumlu açık rıza veya anonimleştirme gerekir.

---

## 5. Ölçülen metrikler

### 5.1 Genel

| Metrik | Tanım |
|---|---|
| **WER** | Kelime hata oranı: `(S + D + I) / N` |
| **CER** | Karakter hata oranı |

### 5.2 Kategori bazında ayrı ölçülen hatalar

| # | Hata türü | Ölçüm |
|---|---|---|
| H1 | Özel isim hatası | K12'de isim bazında doğruluk |
| H2 | Sayı hatası | K14'te sayı bazında doğruluk |
| H3 | Tarih hatası | K14'te tarih bazında doğruluk |
| H4 | Olumsuzluk hatası | K15'te olumsuzluk sözcüğünün doğruluğu (kritik) |
| H5 | Cümle atlama | Referansta olup çıktıda bulunmayan cümle sayısı |
| H6 | Konuşma dışı metin üretme | Sessizlik/müzik bölümlerinde üretilen metin |
| H7 | Sessizlikte metin üretme | Tam sessizlikte üretilen segment sayısı |
| H8 | Zaman kodu hatası | Segment sınırlarının referanstan sapması (±0,5 sn) |
| H9 | Örtüşen konuşmayı yanlış birleştirme | K6'da konuşmacı karışması |
| H10 | Parça sınırında tekrar/kayıp | Parça sınırı ±2 sn penceresinde tekrar/eksik |

---

## 6. Ürünün ölçümden bağımsız güvenceleri

Bu ölçüm sonucu ne olursa olsun, ürün şu güvenceleri sağlar:

1. **Birebir yaklaşım:** Transkript, konuşmacının ifade biçimini bozacak
   şekilde kendiliğinden düzeltilmez (`conventions::verbatim_note`).
2. **Belirsizlik işaretleri:** Anlaşılamayan bölümler
   `[anlaşılamayan bölüm]` gibi işaretlerle gösterilir; sistem kelime
   uydurmaz.
3. **Otomatik kalite kontrolü:** 16 kontrol türü çalışır; özel isim, sayı,
   tarih ve **olumsuzluk** yüksek/orta önemde işaretlenir.
4. **Halüsinasyon taraması:** Sessizlikte metin, bilinen kalıplar, tekrar ve
   yoğunluk denetlenir.
5. **Bilirkişi onayı zorunlu:** Makine çıktısı, teknik kontrol tamamlanmadan
   bilirkişi onaylı sayılamaz
   (`content_status::tests::machine_output_cannot_be_approved_directly`).
6. **Güven göstergesi dürüstlüğü:** Model log olasılığı **doğruluk yüzdesi
   olarak sunulmaz** (`quality::confidence_note`).

---

## 7. Model güncelleme karşılaştırması

Model güncellemeleri **aynı test setiyle** karşılaştırılır. Karşılaştırma
kaydı `ModelVersion` içinde tutulur: model adı, sürüm, dosya SHA-256'sı,
lisans, ölçüm tarihi, WER/CER.

**Kabul ölçütü:** Yeni model, önceki modelden kötü değilse (WER artmıyorsa)
kabul edilir. H4 (olumsuzluk) ve H2/H3 (sayı/tarih) metrikleri kritiktir;
bunlarda bozulma kabul edilmez.

---

## 8. Bilinen davranışlar

| Davranış | Açıklama | Karşı önlem |
|---|---|---|
| Sessizlikte metin üretme | Whisper ailesinin bilinen davranışı | Sessiz parçalar çözülmez; halüsinasyon taraması çalışır |
| Tekrar döngüsü | Uzun sessizlikte tekrar | `detect_repetitions` işaretler |
| Altyazı kalıntıları | Eğitim verisinden kalma kalıplar | `KNOWN_HALLUCINATION_PATTERNS` taranır |
| Sayı/özel isim hataları | Ölçümde doğrulandı | Kalite kontrolü yüksek/orta önemde işaretler |
| Olumsuzluk atlama | Kritik hata | `contains_negation` yüksek önemde işaretler |
| Sentetik ses | TTS dağılım dışıdır | Gerçek ölçüm doğal konuşmayla yapılmalıdır |

---

## 9. Sonuç

- Ölçüm çerçevesi, 15 kategori ve 10 hata metriği **tamamlandı.**
- **Gerçek Whisper modeli ile gerçek ses üzerinde ölçüm yapıldı** ve sonuçlar
  yukarıda açıkça raporlandı.
- Ölçüm sentetik sesle yapıldığı için **doğal konuşmadaki başarıyı
  göstermez**; bu sınır gizlenmez.
- Kurumsal doğrulanmış veri setiyle genişletilmiş ölçüm **kurum tarafından
  yapılmalıdır**; çerçeve ve araç hazırdır.
- **WER sonucu, transkriptin bilirkişi tarafından doğrulandığı anlamına
  gelmez.**
