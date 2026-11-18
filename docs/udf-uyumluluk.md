# UDF Uyumluluk ve Doğrulama

Bu belge, BİRKÂT'ın UYAP UDF (Doküman Formatı) desteğini, doğrulama
yöntemini ve gerçek UYAP Editör ile yapılması gereken nihai test prosedürünü
tanımlar (şartname 15, 23.2).

---

## 1. UDF biçimi

UDF dosyası, Deflate sıkıştırmalı bir **ZIP arşividir**. Arşiv içinde tek bir
dosya bulunur: `content.xml`.

`content.xml`, UTF-8 kodlamalı bir XML'dir:

```xml
<?xml version="1.0" encoding="UTF-8" ?>
<template format_id="1.8">
  <content><![CDATA[ ... tüm düz metin ... ]]></content>
  <properties>
    <pageFormat mediaSizeName="1" leftMargin="42.52" rightMargin="28.35"
                topMargin="14.17" bottomMargin="14.17" paperOrientation="1"
                headerFOffset="20.0" footerFOffset="20.0" />
  </properties>
  <elements resolver="hvl-default">
    <header>...</header>
    <paragraph Alignment="0" LeftIndent="0.0" RightIndent="0.0">
      <content startOffset="0" length="12" family="Times New Roman" size="11.0" />
    </paragraph>
    <table columnCount="2" columnSpans="150.0,200.0" border="borderCell">
      <row rowType="headerRow" border="borderTable" height="20.0">
        <cell colspan="1" align="top" border="borderCell"
              borderStyle="borderStyle-solid" borderWidth="1.0"
              borderColor="-16777216" borderSpec="15">
          <paragraph>...</paragraph>
        </cell>
      </row>
    </table>
    <page-break>...</page-break>
    <footer pageNumber-spec="BSP32_40" pageNumber-fontFace="Arial"
            pageNumber-fontSize="11" pageNumber-color="-16777216">...</footer>
  </elements>
  <styles>
    <style name="default" ... />
    <style name="hvl-default" family="Times New Roman" size="12" />
  </styles>
</template>
```

### 1.1 Kritik kural: karakter (rune) ofsetleri

Belgenin tüm düz metni tek bir CDATA havuzunda tutulur. Havuz sırası:

**üstbilgi → gövde → altbilgi**

`<elements>` içindeki `<content>`, `<image>`, `<tab>` ve `<space>` elemanları
havuzdaki konumlarını `startOffset` ve `length` ile bildirir. Bu değerler
**karakter (rune) sayısıdır**, bayt değil. Türkçe karakterler ve emoji dahil
her karakter 1 sayılır.

Örnek: havuz `"Merhaba\nDünya"` ise

- `"Merhaba"` → `startOffset="0" length="7"`
- `"\n"` → `startOffset="7" length="1"`
- `"Dünya"` → `startOffset="8" length="5"`

BİRKÂT bu kuralı `char_len` / `chars().count()` ile uygular ve
`writer::tests::turkish_characters_count_as_one` testiyle doğrular.

### 1.2 Özel karakterler

| Karakter | Unicode | Kullanım |
|---|---|---|
| Resim yer tutucusu | U+FFFC | CDATA'da resim konumu |
| Sekme | U+0009 | Sekme karakteri |
| Satır sonu | U+000A | Paragraf ayırıcı (isteğe bağlı) |
| Sıfır genişlikli boşluk | U+200B | Boş paragraf işaretçisi |

### 1.3 Renk kodlaması

Renkler işaretli 32-bit ARGB tam sayısıdır:

```text
Hex #RRGGBB → ARGB = 0xFF000000 | (R << 16) | (G << 8) | B → signed int32
```

Yaygın değerler: siyah `-16777216`, beyaz `-1`, kırmızı `-65536`.

### 1.4 Paragraf ayırıcı tercihi

Biçim belgeleri havuzda paragrafların `\n` ile ayrıldığını yazar. Ancak
üretimde kullanılan ve UYAP Editör'de açıldığı bildirilen açık kaynak
dönüştürücüler ayırıcı **eklemez**; paragraf yapısı yalnızca `<elements>`
ağacından kurulur. BİRKÂT varsayılan olarak bu davranışı izler;
`UdfWriter::with_paragraph_separators(true)` ile `\n` ayırıcı eklenebilir.

### 1.5 UYAP uyumluluğu için korunan yazım hatası

Arka plan resmi elemanındaki sağ kenar boşluğu alanı, UYAP uyumluluğu için
`bgImageRigtMargin` (doğru yazım `Right` değil) olarak yazılır. Okuyucu da bu
adı bekler.

---

## 2. Desteklenen UDF öğeleri

| Öğe | Yazma | Okuma | Doğrulama | Test |
|---|---|---|---|---|
| `<template format_id>` | ✓ | ✓ | ✓ | `format_id_is_read` |
| `<content>` CDATA havuzu | ✓ | ✓ | ✓ | `simple_paragraph_round_trips` |
| `<properties>/<pageFormat>` | ✓ | ✓ | ✓ | `page_format_round_trips` |
| `<bgImage>` | ✓ | ✓ | — | `background_image_round_trips` |
| `<header>` | ✓ | ✓ | — | `header_is_separate_from_body` |
| `<footer>` + sayfa numarası | ✓ | ✓ | — | `footer_page_number_round_trips` |
| `<paragraph>` (7 öznitelik) | ✓ | ✓ | ✓ | `offsets_are_character_based_and_contiguous` |
| `<content>` (7 öznitelik) | ✓ | ✓ | ✓ | `styles_survive_round_trip` |
| `<tab>` | ✓ | ✓ | ✓ | `tab_and_space_runs_round_trip` |
| `<space>` | ✓ | ✓ | ✓ | `tab_and_space_runs_round_trip` |
| `<image>` (base64, boyut) | ✓ | ✓ | ✓ | `image_round_trips` |
| `<table>` (sütun genişlikleri) | ✓ | ✓ | ✓ | `table_round_trips` |
| `<row>` (tür, yükseklik) | ✓ | ✓ | ✓ | `table_round_trips` |
| `<cell>` (colspan, kenarlık, dolgu) | ✓ | ✓ | ✓ | `table_renders_rows_and_cells` |
| `<page-break>` | ✓ | ✓ | — | `page_break_round_trips` |
| Numaralı liste (10 tür) | ✓ | ✓ | — | `list_attributes_are_written` |
| Madde işaretli liste (7 tür) | ✓ | ✓ | — | `list_attributes_are_written` |
| `<styles>` | ✓ | ✓ | ✓ | `styles_section_round_trips` |
| Boş paragraf (U+200B) | ✓ | ✓ | ✓ | `empty_paragraph_gets_zero_width_space` |
| `]]>` kaçışı (CDATA bölme) | ✓ | ✓ | — | `cdata_split_terminator_is_rejoined` |

---

## 3. Doğrulama katmanları

BİRKÂT, ürettiği UDF'yi **üç katmanda** doğrular:

### 3.1 Yazıcı sonrası yapısal doğrulama (otomatik, her üretimde)

`birkat_udf::validate_udf` şu kontrolleri yapar:

| Kontrol | Ne denetler |
|---|---|
| `zip_integrity` | Paket açılabiliyor mu |
| `content_xml` | `content.xml` pakette var mı |
| `xml_parse` | XML çözümlenebiliyor mu |
| `format_id` | `format_id="1.8"` mi |
| `offset_bounds` | Tüm `startOffset`+`length` havuz sınırları içinde mi |
| `pool_empty` | Metin havuzu boş mu (uyarı) |
| `turkish_characters` | Türkçe karakterler korunmuş mu |
| `page_margins`, `page_usable_width`, `page_orientation` | Sayfa yapısı geçerli mi |
| `table_columns`, `table_width`, `table_row_cells` | Tablo yapısı tutarlı mı |
| `image_data`, `image_base64`, `image_size`, `image_overflow` | Görsel referansları geçerli mi |
| `styles`, `empty_document` | Stil ve içerik var mı |
| `font_size`, `placeholder_in_text` | Biçim aralıkları makul mü |

**Kural:** Doğrulama başarısızsa dosya "hazır" olarak gösterilmez
(`birkat rapor` hata döndürür ve hata listesini yazdırır).

### 3.2 Kendi okuyucumuzla geri okuma (otomatik)

Üretilen UDF, BİRKÂT'ın okuyucusuyla açılır ve metin havuzu ile ofset
çözümlemesi doğrulanır. Bu, kendi kendini doğrulayan bir kapalı döngüdür ve
**tek başına yeterli değildir** (şartname 15.4).

### 3.3 Uygulama içi sayfa önizlemesi (otomatik, ön kontrol)

`birkat_report::preview::build_preview`, UDF'yi yazdırma düzeninde sayfalara
böler, metni sarar, tablo taşmalarını ve boş sayfaları işaretler. Bu
**ön kontrol katmanıdır**; gerçek UYAP Editör görünümünü doğrulamanın yerine
geçmez.

### 3.4 Gerçek UYAP Editör testi (manuel — teslim öncesi yapılmalıdır)

Bkz. bölüm 5.

---

## 4. Gerçek UYAP Editör test seti (şartname 23.2)

Aşağıdaki test seti, UYAP Editör ile yapılacak nihai uyumluluk testinin
kapsamını tanımlar. Her madde için hem **BİRKÂT üretimi** hem de **Editör
tarafından üretilmiş gerçek UDF** ile karşılaştırma yapılmalıdır.

### 4.1 BİRKÂT üretimi test dosyaları

| # | Test | İçerik | Beklenen |
|---|---|---|---|
| T1 | Paragraflar | Düz metin, Türkçe karakterler | Metin eksiksiz, karakterler doğru |
| T2 | Başlıklar | 3 seviye başlık | Kalın ve büyük punto |
| T3 | Tablolar | 2 ve 7 sütunlu tablolar | Sütun genişlikleri doğru, taşma yok |
| T4 | Birleşik hücreler | `colspan="2"` | Birleşme korunur |
| T5 | Görseller | JPEG ve PNG, alt yazılı | Görsel yerinde, boyut doğru |
| T6 | Üstbilgi/altbilgi | Üstbilgi metni + sayfa numarası | Her sayfada görünür |
| T7 | Sayfa numaraları | `BSP32_40` biçimi | Numaralar doğru artar |
| T8 | Listeler | Numaralı ve madde işaretli, 2 seviye | İşaretler doğru |
| T9 | Sayfa sonları | Zorunlu sayfa sonu | Yeni sayfada başlar |
| T10 | Uzun metin | 200+ paragraf | Çok sayfalı, boş sayfa yok |
| T11 | Türkçe karakterler | ç Ç ğ Ğ ı İ ö Ö ş Ş ü Ü â î û | Hiçbiri bozulmaz |
| T12 | Uzun transkript tablosu | 500 satır | Satır kaybı yok |
| T13 | Ek belge | Ayrı UDF ek | Ana raporda numaralandırılır |
| T14 | Özel karakterler | `< > & " ' ]]>` | Kaçış doğru, metin korunur |
| T15 | Boş paragraf | Arka arkaya boş paragraflar | Yükseklik korunur |

Bu dosyaların bir kısmı otomatik testlerde üretilir
(`birkat-udf`, `birkat-report` testleri). Tam set, `birkat rapor` komutuyla
gerçek vaka verisinden üretilebilir.

### 4.2 Editör tarafından üretilmiş gerçek UDF ile karşılaştırma

Gerçek UDF örnekleri (UYAP Editör ile oluşturulmuş) şu özellikleri
içermelidir: paragraflar, başlıklar, tablolar, birleşik hücreler, görseller,
üstbilgi/altbilgi, sayfa numaraları, listeler, sayfa sonları, dipnotlar,
Türkçe karakterler, uzun metinler ve ek belgeler.

BİRKÂT'ın okuyucusu bu dosyaları **uyarı üretmeden** açabilmelidir. Okuyucu
bilinmeyen öğeleri sessizce atlar ve `warnings` listesinde bildirir.

---

## 5. Nihai uyumluluk test prosedürü (teslim öncesi)

Bu prosedür Windows ortamı ve resmî UYAP Editör gerektirir; bu geliştirme
ortamında yapılamamıştır.

1. **Editör sürümünü belirle.** Adalet Bakanlığı'nın resmî indirme
   sayfasından (`https://www.uyap.gov.tr/UYAP-Editor`) proje kabul tarihindeki
   en güncel Windows sürümünü indir. Sürüm numarasını ve indirme tarihini
   kaydet. Sürüm numarasını kaynak kodda sabitleme.

2. **Test dosyalarını üret.** `birkat rapor` ile T1–T15 test dosyalarını
   üret.

3. **Editör'de aç.** Her dosyayı çift tıklayarak aç. Açılmıyorsa hatayı ve
   dosyayı kaydet.

4. **Şu kontrolleri yap ve kaydet:**

   | Kontrol | Beklenen | Sonuç |
   |---|---|---|
   | UDF açılıyor mu? | Evet | |
   | Türkçe karakterler doğru mu? | Evet | |
   | Metin eksiksiz mi? | Evet | |
   | Tablolar doğru mu? | Evet | |
   | Sayfa numaraları doğru mu? | Evet | |
   | Üstbilgi/altbilgi doğru mu? | Evet | |
   | Görseller doğru mu? | Evet | |
   | Sayfa taşması var mı? | Hayır | |
   | Boş veya eksik sayfa oluşuyor mu? | Hayır | |
   | Kaydetme çalışıyor mu? | Evet | |
   | Kaydedilmiş UDF tekrar açılıyor mu? | Evet | |
   | Yazdırma/önizleme görünümü uygun mu? | Evet | |
   | Resmî imza akışında dosya kullanılabiliyor mu? | Evet | |

5. **Kaydetme turu.** Editör'de aç, küçük bir değişiklik yap, kaydet. Kaydedilen
   dosyayı BİRKÂT'ın okuyucusuyla aç ve metnin korunduğunu doğrula. Editör'ün
   kaydetme sırasında hangi öğeleri değiştirdiğini not et.

6. **Fark raporu.** Editör'ün farklı yorumladığı veya desteklemediği bir özellik
   bulunursa:
   - İçerik kaybı olup olmadığını belirle.
   - BİRKÂT'ın `validate_udf` çıktısıyla karşılaştır.
   - `docs/udf-uyumluluk.md` içindeki tabloyu güncelle.
   - Gerekiyorsa rapor şablonunu Editör'ün gerçek desteğine göre düzelt.

7. **Sonuç.** Bu prosedür tamamlanmadan UDF uyumluluğu **doğrulanmış sayılmaz.**

---

## 6. Bilinen UDF sınırlamaları

| Sınırlama | Açıklama |
|---|---|
| Dipnot/sonnot | Yazıcı dipnot üretmez; rapor şablonu dipnot gerektirmez. Editör'ün dipnot içeren dosyaları okunabilir. |
| Alan (`<field>`) elemanı | Şablon alanları için belgelenmiş yapı yoktur; BİRKÂT şablon alanı üretmez. |
| `<data>` bölümü | Şablon verisi bölümü belgelenmemiştir; BİRKÂT üretmez, okurken yok sayar. |
| Sayfa numarası biçimi | `BSP32_40` varsayılanı kullanılır; Editör'ün kabul ettiği biçimler doğrulanmalıdır. |
| 7z/RAR arşivleri | UDF içeren arşivler ZIP ise açılır; 7z/RAR açık hata verir. |

---

## 7. Kaynaklar

- Adalet Bakanlığı UYAP Editör indirme: <https://www.uyap.gov.tr/UYAP-Editor>
- UYAP Editör yardım: <https://uyap.gov.tr/Uyap-Editor-Yardim>
- Bilirkişi Portal: <https://kurum.uyap.gov.tr/main/bilirkisi.jsp>
- Açık kaynak UDF dönüştürücü (biçim referansı):
  <https://github.com/saidsurucu/UDF-Toolkit>
- Açık kaynak UDF biçim notu:
  <https://github.com/talhaorak/uyap-dilekce-kit>

> Bu kaynaklar **biçim referansı** olarak kullanılmıştır. Nihai uyumluluk
> kontrolü yalnızca resmî UYAP Editör ile yapılabilir.
