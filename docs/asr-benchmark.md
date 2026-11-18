# Türkçe ASR Benchmark Raporu

Bu belge, BİRKÂT'ın transkripsiyon doğruluğunu ölçmek için kullanılan test
veri setini, ölçüm yöntemini ve sonuçları tanımlar (şartname 23.1).

> **Önemli:** Bu raporda bir WER yüzdesi, transkriptin bilirkişi tarafından
> doğrulandığı anlamına **gelmez.** WER, yalnızca model davranışını
> karşılaştırmak için kullanılan teknik bir ölçüdür.

---

## 1. Ölçüm durumu

| Aşama | Durum |
|---|---|
| Ölçüm çerçevesi ve metrikler | ✓ Hazır (bu belge) |
| Kategori tanımları | ✓ Hazır |
| Nicel ölçüm (gerçek veri) | ✗ Yapılmadı — telifli veri seti gerekir |
| Model karşılaştırma altyapısı | ✓ `ModelPackage` + `ModelVersion` ile hazır |

Nicel ölçüm, kurumun kendi doğrulanmış Türkçe veri setiyle yapılmalıdır.
Bu belge, ölçümün nasıl yapılacağını tam olarak tanımlar; böylece sonuçlar
tekrarlanabilir ve karşılaştırılabilir olur.

---

## 2. Test veri seti gereksinimleri

Veri seti, Türkçe bilirkişi kullanımına uygun ve **insan tarafından
doğrulanmış** olmalıdır. Aşağıdaki ses koşullarının her biri için en az
10 örnek bulunmalıdır (toplam ≥ 150 kayıt, ≥ 5 saat):

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

## 3. Ölçülen metrikler

### 3.1 Genel metrikler

| Metrik | Tanım |
|---|---|
| **WER** | Kelime hata oranı: `(S + D + I) / N` |
| **CER** | Karakter hata oranı: `(S + D + I) / N` (karakter bazında) |

`S` = yerine koyma, `D` = silme, `I` = ekleme, `N` = referans kelime/karakter
sayısı.

### 3.2 Kategori bazında ayrı ölçülen hatalar (şartname 23.1)

| # | Hata türü | Ölçüm |
|---|---|---|
| H1 | Özel isim hatası | K12 kayıtlarında isim bazında doğruluk |
| H2 | Sayı hatası | K14 kayıtlarında sayı bazında doğruluk |
| H3 | Tarih hatası | K14 kayıtlarında tarih bazında doğruluk |
| H4 | Olumsuzluk hatası | K15 kayıtlarında olumsuzluk sözcüğünün doğruluğu (kritik) |
| H5 | Cümle atlama | Referansta olup çıktıda hiç bulunmayan cümle sayısı |
| H6 | Konuşma dışı metin üretme | Sessizlik/müzik bölümlerinde üretilen metin |
| H7 | Sessizlikte metin üretme | Tam sessizlikte üretilen segment sayısı |
| H8 | Zaman kodu hatası | Segment sınırlarının referanstan sapması (±0,5 sn eşik) |
| H9 | Örtüşen konuşmayı yanlış birleştirme | K6 kayıtlarında konuşmacı karışması |
| H10 | Parça sınırında tekrar/kayıp | Parça sınırı ±2 sn penceresinde tekrar/eksik kelime |

### 3.3 Ölçüm aracı

BİRKÂT, ölçüm için gerekli altyapıyı sağlar:

- Transkripsiyon çıktısı `Transcript` (JSON) olarak kasaya yazılır.
- Segment zaman kodları `TranscriptSegment` içinde tutulur.
- Kalite kontrolü, H5–H10 hatalarını otomatik olarak işaretler
  (`QualityFindingKind`).
- Model karşılaştırması için `ModelVersion` (ad, sürüm, dosya hash'i) kaydedilir.

---

## 4. Ölçüm prosedürü

```bash
# 1. Veri setini hazırla
#    kayitlar/  altında WAV/MP3 dosyaları
#    referanslar/ altında aynı adlı TXT dosyaları (insan doğrulamalı)

# 2. Her kaydı transkript et
for f in kayitlar/*; do
  birkat --veri ./bench-veri vaka-olustur --baslik "$(basename $f)" \
         --mahkeme "Benchmark" > /dev/null
  # vaka kimliğini al ve transkript et
done

# 3. WER/CER hesapla (harici betik)
python3 scripts/benchmark-wer.py --referans referanslar/ --cikti cikti/

# 4. Kategori bazında raporla
```

`scripts/benchmark-wer.py` betiği, WER/CER ve H1–H10 metriklerini hesaplar
ve kategori bazında Markdown tablosu üretir.

---

## 5. Sonuç tablosu (doldurulacak)

Ölçüm yapıldığında aşağıdaki tablo doldurulur:

| Model | Sürüm | Veri seti | WER (%) | CER (%) | Not |
|---|---|---|---|---|---|
| Whisper large-v3-turbo | — | K1–K15 | — | — | Ölçüm bekliyor |
| Whisper large-v3 | — | K1–K15 | — | — | Karşılaştırma |
| Whisper medium | — | K1–K15 | — | — | Karşılaştırma |

Kategori bazında:

| Kategori | WER (%) | H1 | H2 | H3 | H4 | H5 | H6 | H7 | H8 | H9 | H10 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| K1 Temiz konuşma | — | — | — | — | — | — | — | — | — | — | — |
| K2 Telefon | — | — | — | — | — | — | — | — | — | — | — |
| K3 Gürültülü | — | — | — | — | — | — | — | — | — | — | — |
| K4 Düşük ses | — | — | — | — | — | — | — | — | — | — | — |
| K5 Çok konuşmacı | — | — | — | — | — | — | — | — | — | — | — |
| K6 Örtüşen | — | — | — | — | — | — | — | — | — | — | — |
| K7 Farklı hız | — | — | — | — | — | — | — | — | — | — | — |
| K8 Aksan | — | — | — | — | — | — | — | — | — | — | — |
| K9 Uzak mikrofon | — | — | — | — | — | — | — | — | — | — | — |
| K10 Kesintili | — | — | — | — | — | — | — | — | — | — | — |
| K11 Uzun kayıt | — | — | — | — | — | — | — | — | — | — | — |
| K12 Özel isim | — | — | — | — | — | — | — | — | — | — | — |
| K13 Kurum/yer | — | — | — | — | — | — | — | — | — | — | — |
| K14 Sayı/tarih | — | — | — | — | — | — | — | — | — | — | — |
| K15 Olumsuzluk | — | — | — | — | — | — | — | — | — | — | — |

---

## 6. Model güncelleme karşılaştırması

Model güncellemeleri **aynı test setiyle** karşılaştırılır. Karşılaştırma
kaydı `ModelVersion` içinde tutulur:

- Model adı ve sürümü
- Model dosyası SHA-256'sı
- Lisans
- Ölçüm tarihi
- WER/CER sonuçları

**Kabul ölçütü:** Yeni model, önceki modelden kötü değilse (WER artmıyorsa)
kabul edilir. H4 (olumsuzluk) ve H2/H3 (sayı/tarih) metrikleri kritiktir;
bunlarda bozulma kabul edilmez.

---

## 7. Model güven göstergeleri hakkında

Whisper, her segment için log olasılık üretir. BİRKÂT bu değeri
`TranscriptSegment::model_confidence` içinde **gösterge** olarak saklar ve
şu uyarıyla sunar:

> Model log olasılık veya güven göstergeleri, gerçek doğruluk yüzdesi olarak
> sunulmaz. Bu göstergeler yalnızca inceleme önceliği belirlemek için
> kullanılır. Hiçbir kalite göstergesi bilirkişi onayının yerine geçmez.

`quality::confidence_note()` bu metni döndürür ve
`quality::tests::confidence_note_is_honest` testiyle korunur.

---

## 8. Bilinen davranışlar

| Davranış | Açıklama | Karşı önlem |
|---|---|---|
| Sessizlikte metin üretme | Whisper ailesinin bilinen davranışı | Sessiz parçalar çözülmez; halüsinasyon taraması çalışır |
| Tekrar döngüsü | Uzun sessizlikte tekrar | `detect_repetitions` işaretler |
| Altyazı kalıntıları | Eğitim verisinden kalma kalıplar | `KNOWN_HALLUCINATION_PATTERNS` listesi taranır |
| Özel isim hataları | Eğitim verisinde geçmeyen adlar | Kalite kontrolü "olası özel isim" işaretler |
| Olumsuzluk atlama | Kritik hata | `contains_negation` ile yüksek önemde işaretlenir |
| Parça sınırı kaybı | Parça tabanlı işleme | 2 saniye örtüşme + sınır artefaktı denetimi |

---

## 9. Sonuç

- Ölçüm çerçevesi, kategori tanımları ve metrikler **tamamlanmıştır.**
- Nicel ölçüm, telifli ve kişisel veri içeren veri seti gerektirdiği için
  **kurum tarafından yapılmalıdır.**
- Ölçüm tamamlandığında bölüm 5'teki tablolar doldurulur ve bu belge
  güncellenir.
- **WER/CER sonucu, transkriptin bilirkişi tarafından doğrulandığı anlamına
  gelmez.**
