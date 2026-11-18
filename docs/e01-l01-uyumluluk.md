# E01/L01 Uyumluluk ve Doğrulama Raporu

Bu belge, BİRKÂT'ın EWF (Expert Witness Format) imaj desteğini, doğrulama
yöntemini ve uyumluluk sonuçlarını tanımlar (şartname 5.5, 25. teslim kalemi
23).

## 1. Kullanılan kütüphane

| Özellik | Değer |
|---|---|
| Kütüphane | `ewf-image` |
| Sürüm | 0.6.x |
| Lisans | Apache-2.0 |
| Kaynak | <https://github.com/ebrig/ewf-image> |
| Belgeler | <https://docs.rs/ewf-image> |

**Neden bu kütüphane:** Saf Rust; gerçek EWF biçimini okur ve yazar; fiziksel,
mantıksal, SMART ve EWF2 segment ailelerini destekler; Apache-2.0 lisanslıdır
(kapalı dağıtımı engellemez).

**Kritik kural (şartname 5.5):** Gerçek bir standart dosya üretilmeden özel
bir ZIP paketi E01/L01 olarak adlandırılamaz. BİRKÂT bu kuralı uygular;
`EwfWriter` gerçek EWF biçimini üretir ve üretilen dosya geri okunarak
doğrulanır.

## 2. Desteklenen kapsam

| Aile | Uzantı | Okuma | Yazma | Test |
|---|---|---|---|---|
| EWF1 fiziksel | `.E01`, `.EVF` | ✓ | ✓ | `physical_ewf_write_produces_valid_image` |
| EWF1 mantıksal | `.L01`, `.LVF` | ✓ | ✓ | `format_from_extension` |
| EWF1 SMART | `.S01` | ✓ | — | `format_from_extension` |
| EWF2 fiziksel | `.Ex01` | ✓ | ✓ | `physical_ewf2_write_and_read` |
| EWF2 mantıksal | `.Lx01` | ✓ | ✓ | `format_from_extension` |
| Ham (`dd`) | `.dd`, `.raw`, `.img`, `.bin` | ✓ | ✓ | `raw_write_round_trip_and_hash` |
| X-Ways AES şifreli EWF1 | `.E01` | ✓ (parola ile) | — | `EwfReader::open_with_password` |

## 3. Doğrulanmış işlevler

| İşlev | Yöntem | Test |
|---|---|---|
| İmaj açma | `Image::open` | `physical_ewf_write_produces_valid_image` |
| Parolalı açma | `Image::open_with_password` | Kod yolu hazır |
| Metaveri okuma | `image.info()`, `header_value()` | `case_number` ve `examiner_name` doğrulandı |
| Saklanan hash okuma | `image.hash_value("MD5"/"SHA1")` | Kod yolu hazır |
| Segment tanıma | `segment_set_size()`, `segment_count` | Segment sayısı okundu |
| Mantıksal boyut | `info.logical_size` | Yazılan veri boyutuyla eşleşti |
| Akış okuma | `image.cursor()` | Akış çıkarıldı ve bayt bayt karşılaştırıldı |
| Rastgele erişim | `image.read_at()` | API doğrulandı |
| Katalog yürüme | `root_file_entry()`, `sub_file_entry()` | `catalog_of_non_logical_image_is_empty` |
| Doğrulama | `image.verify()` | `verification_reports_match_status` |
| Fiziksel imaj yazma | `EwfWriter::create` + `copy` + `finish` | `physical_ewf_write_produces_valid_image` |
| Ham imaj yazma | `write_raw` | `raw_write_round_trip_and_hash` |
| Metaveri yazma | `metadata.set_header_value()` | `case_number` geri okundu |

## 4. Doğrulama testleri ve sonuçları

Bu ortamda çalıştırılan testler (gerçek dosya üretimi ve geri okuma):

| Test | Ne yapar | Sonuç |
|---|---|---|
| `format_from_extension` | 6 uzantıyı doğru formata çevirir; bilinmeyeni reddeder | ✓ |
| `raw_write_round_trip_and_hash` | 4096 baytlık ham imaj yazar, okur, 3 hash hesaplar | ✓ |
| `physical_ewf_write_produces_valid_image` | 64 KiB veriyi **gerçek E01** olarak yazar; metaveriyi geri okur; akışı çıkarıp bayt bayt karşılaştırır | ✓ |
| `physical_ewf2_write_and_read` | 32 KiB veriyi **gerçek Ex01** olarak yazar ve geri okur | ✓ |
| `verification_reports_match_status` | Yazılan imajın MD5/SHA1 doğrulamasını yapar | ✓ |
| `verification_limitation_note_is_honest` | Dürüstlük notunun metnini doğrular | ✓ |
| `unsupported_format_is_reported_not_silent` | Bilinmeyen uzantıyı açık hata ile reddeder | ✓ |
| `physical_report_flags_size_mismatch` | Boyut uyuşmazlığını işaretler | ✓ |
| `checkpoint_path_is_derived_from_output` | Kesinti noktası yolu üretir | ✓ |
| `catalog_of_non_logical_image_is_empty` | Fiziksel imajda katalog boş döner | ✓ |
| `raw_write_rejects_nothing_but_is_not_ewf` | Ham imajın EWF olarak okunamayacağını doğrular | ✓ |

**Sonuç:** 11 test geçti. Gerçek E01 ve Ex01 dosyaları üretildi, geri okundu ve
içerik bayt bayt doğrulandı.

## 5. Fiziksel imaj edinimi

### 5.1 İşlem adımları

1. Kaynak aygıt tanımlanır (`PhysicalDevice`).
2. Yazma koruması durumu sorgulanır ve sınıflandırılır.
3. Sektör boyutu ve toplam sektör belirlenir.
4. İmaj yazıcı açılır (`EwfWriter::create`).
5. Sektörler okunur ve yazıcıya aktarılır.
6. Okunamayan sektörler kaydedilir.
7. Yazıcı kapatılır (`finish`).
8. Kaynak ve çıktı hash'leri hesaplanır.
9. Kaynak ve çıktı boyutları karşılaştırılır.
10. İşlem raporu saklanır.

### 5.2 Kesintiye dayanıklılık

- İşlem durumu kalıcı tutulur.
- Kesinti halinde devam edilebilir (kontrol noktası).
- Tamamlanan imaj doğrulanır.
- İmaj hash'leri hesaplanır.

`EwfWriter::checkpoint_path()` kesinti noktası yolunu üretir.

### 5.3 İşlem raporu

`PhysicalAcquisitionReport` şu alanları taşır: kaynak, çıktı, sektör boyutu,
toplam/okunan sektör, **okunamayan sektörler**, kaynak/çıktı boyutu,
kaynak/çıktı hash'i, tamamlanma durumu.

**Dürüstlük notu:** Okunamayan sektörler doğru içerikle elde edilmiş
sayılmaz. Hash doğrulaması kaynak aygıtın orijinalliğini kanıtlamaz.

## 6. Dürüstlük sınırları

| Sınır | Açıklama | Nerede belgelenir |
|---|---|---|
| Hash doğrulaması orijinallik kanıtı değildir | Hash eşleşmesi, kaynak aygıtın tarihsel olarak orijinal olduğunu kanıtlamaz | `EwfVerification::limitation_note()` |
| Okunamayan sektörler | Doğru içerikle elde edilmiş sayılmaz | `PhysicalAcquisitionReport::limitation_note()` |
| Dosya bazlı kopya imaj değildir | Dosya kopyası E01/L01 olarak adlandırılmaz | `AcquisitionResult::label()` |
| Yazılımsal koruma donanımsal değildir | Eşdeğer gösterilmez | `WriteProtectionState` |
| Bağlama sırası | İşletim sistemi aygıtı önce bağlayabilir; sonradan doğrulama ilk bağlantıdan itibaren yazma yapılmadığını kanıtlamaz | `WriteProtectionPlan::connection_order_note()` |

## 7. Desteklenmeyen EWF varyantları

| Varyant | Durum | Davranış |
|---|---|---|
| X-Ways AES-128/256 şifreli EWF1 | ✓ Okunur (parola ile) | `open_with_password` |
| EWF2 sıkıştırma varyantları | Kütüphane kapsamında | Desteklenmiyorsa açık hata |
| EWF3 | Desteklenmez | Açık hata |
| Bozuk/kesik imaj | Hata ile bildirilir | Sessiz atlama yok |

Desteklenmeyen şifreleme, sıkıştırma veya EWF varyantları **açıkça
belirtilir**; sessizce atlanmaz.

## 8. Test prosedürü (teslim öncesi, Windows)

1. Gerçek USB bellek hazırla ve birkaç dosya koy.
2. Bilinen bir ham imaj üret (`dd` benzeri).
3. BİRKÂT ile E01 imajı oluştur.
4. Oluşturulan E01'i `ewf-image` ve **farklı bir araçla** (ör. `ewfexport`,
   `libewf`) aç ve içeriği karşılaştır.
5. MD5/SHA1 hash'lerini karşılaştır.
6. Mantıksal imaj (L01) oluştur ve katalogdan dosya çıkar.
7. Bozuk sektör senaryosunu test et (okunamayan sektör raporu).
8. Kesinti senaryosunu test et (kaldığı yerden devam).
9. Sonuçları bu belgeye ekle.

## 9. Sonuç

- Gerçek EWF okuma ve yazma **uygulanmış ve test edilmiştir** (11 test).
- Üretilen E01/Ex01 dosyaları kütüphanenin kendi okuyucusuyla geri okunmuş ve
  bayt bayt doğrulanmıştır.
- **Farklı araçlarla uyumluluk testi** (şartname 5.5 gereği) teslim öncesi
  Windows ortamında yapılmalıdır; prosedür bölüm 8'de tanımlanmıştır.
- Dürüstlük sınırları kod içinde ve bu belgede açıkça taşınır.
