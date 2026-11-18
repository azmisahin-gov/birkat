# Kullanıcı Kılavuzu

Bu kılavuz, 56.12 bilirkişisinin BİRKÂT'ı günlük çalışmasında nasıl
kullanacağını anlatır.

## 1. BİRKÂT ne yapar, ne yapmaz

**BİRKÂT yapar:**

- Görevlendirme belgesini okur, dosya bilgilerini ve soruları çıkarır.
- Materyalleri (USB, SD, harici disk, klasör, imaj) tanır ve güvenli edinir.
- Ses dosyalarını bulur, teknik envanterini çıkarır.
- Sesleri **yerel olarak** metne dönüştürür.
- Transkriptte kontrol edilmesi gereken yerleri işaretler.
- Soruları dayanaklarla ilişkilendirir.
- Raporu hazırlar ve **doğrudan UDF** üretir.
- UDF'yi doğrular, ekleri hazırlar, teslim paketini düzenler.
- Dosyayı arşivler, şifreli yedek alır, güncellenir.

**BİRKÂT yapmaz** (bu işler sizin mesleki değerlendirmenize aittir):

- Konuşmacının kim olduğunu söylemez.
- Kaydın montajlı/sahte olduğuna karar vermez.
- Kişinin niyetini veya yalan söyleyip söylemediğini belirlemez.
- Hukuki nitelendirme yapmaz.
- Silinmiş verileri kurtarmaz.
- Sizin yerinize mesleki kanaat oluşturmaz.

Sistem üç içerik durumunu kesin ayırır:

| Durum | Ne demek |
|---|---|
| **Makine çıktısı** | Modelin ürettiği içerik; henüz kontrol edilmedi |
| **İncelemeye hazır içerik** | Teknik kontroller yapıldı; siz onaylamadınız |
| **Bilirkişi onaylı içerik** | Siz incelediniz ve onayladınız |

Makine çıktısı asla kendiliğinden "onaylı" sayılmaz.

## 2. İlk kullanım

1. Uygulamayı açın.
2. **Bilirkişi profili** ekleyin (ad, unvan, sicil no). Bu bilgiler her raporda
   otomatik kullanılır; tekrar yazmanız gerekmez.
3. Veri kök dizinini ve yedek dizinini seçin.
4. Kasa parolasını belirleyin (en az 12 karakter) ve kurtarma anahtarını
   güvenli bir yere yazın.

## 3. Yeni görev

1. **Görevlendirme ve dosya kabul** ekranında vaka başlığı, görevlendiren
   merci, dosya numarası ve inceleme konusunu girin.
2. "Vaka oluştur ve aç" düğmesine basın.
3. Uygulama **dosya çalışma alanını** açar. Dosyanın tüm işlevleri bu
   alandadır; başka program açmanız gerekmez.

## 4. Materyal teslimi ve otomatik işleme

1. **Dosya çalışma alanında** "Materyal yolu" alanına materyal klasörünü veya
   dosyasını yazın (USB belleğin sürücü harfi, bir klasör yolu veya imaj
   dosyası).
2. "Materyalleri al ve işle" düğmesine basın.
3. Uygulama **otomatik olarak** şunları yapar:
   - Materyalleri kasaya alır (SHA-256 ile).
   - Ses dosyalarını tespit eder.
   - Teknik envanteri çıkarır.
   - Tüm uygun sesleri transkripsiyon kuyruğuna alır.
   - Transkripsiyonları üretir.
   - Otomatik kalite kontrolünü çalıştırır.
   - İncelenmesi gereken bölümleri işaretler.
   - Rapor taslağını ve UDF'yi hazırlar.

**Her ses dosyası için ayrı "çalıştır" düğmesine basmanız gerekmez. Her
klasörü tek tek seçmeniz gerekmez. Model, sıcaklık, GPU gibi teknik
parametreler sorulmaz.**

İşlem sırasında kesinti olursa (elektrik, çökme), uygulama kaldığı yerden
devam eder.

## 5. İstisnalar

Uygulama rutin işlerde kendisi ilerler. Yalnızca şu durumlarda kararınızı
ister (alt paneldeki **Bildirimler ve istisnalar** alanında gösterilir):

- Görevlendirme bilgisi belirsiz.
- Materyal eksik.
- Yazma koruması doğrulanamıyor.
- Dosya bozuk veya şifreli.
- Codec çözülemiyor.
- Konuşma anlaşılmıyor.
- Örtüşen konuşma var.
- Özel isim veya kritik kelime belirsiz.
- Görevlendirme sorusu kapsam dışında.
- Raporun mesleki değerlendirme gerektiren alanı boş.
- E-imza veya UYAP sunum işlemi yetkinizi gerektiriyor.

Her istisna kısa ve çözüm odaklı anlatılır.

## 6. Transkript incelemesi

**Ses/transkript inceleme** ekranında:

1. Ses ve transkript yan yana gelir.
2. Bir metin satırına tıklayınca ses o bölüme gider.
3. Ses çalarken ilgili satır vurgulanır.
4. Sesle karşılaştırarak düzeltmeleri yapın.
5. Şu işaretlere dikkat edin:
   - `[anlaşılamayan bölüm]`
   - `[belirsiz: ...]`
   - `[üst üste konuşma]`
   - `[konuşma kesildi]`
   - `[ses anlaşılmıyor]`
   - `[konuşmacı etiketi belirsiz]`
6. Düzeltmeleri kaydedin. Her değişiklik (önceki değer, yeni değer, zaman,
   gerekçe) kayıt altına alınır.

**Önemli:** Konuşmacı etiketleri (Konuşmacı 1, Konuşmacı 2) gerçek kişi
kimliği değildir. Biyometrik tanımlama yapılmaz.

**Önemli:** Kalite kontrolü işaretleri (ör. "olası özel isim", "olumsuzluk
sözcüğü", "sessizlikte metin") yalnızca inceleme önceliğidir. Bunlar
doğruluk yüzdesi değildir ve onayınızın yerine geçmez.

**Önemli:** İlk makine transkripti asla üzerine yazılarak kaybolmaz. Beş ayrı
sürüm korunur.

## 7. Sorular ve yanıtlar

**Sorular ve dayanaklar** ekranında:

1. Her soru orijinal sırasıyla ve **orijinal metniyle** listelenir.
2. Bir yanıtı hazırlarken dayanağı olan ses bölümünü tek işlemle açabilirsiniz.
3. Her yanıt için bir veya birden fazla dayanak bağlayabilirsiniz.
4. Yanıtı doğrudan siz yazarsınız. Sistem taslak önerebilir; ancak taslak
   sizin kanaatiniz değildir ve onaylanmadan rapora girmez.

Yanıtlanmamış soru varsa son kontrolde açık uyarı verilir.

## 8. Bulgular

**Bulgular** ekranında teknik bulgularınızı ve mesleki notlarınızı yazın.
Her bulgu, makine çıktısı / incelemeye hazır / onaylı durumlarından birini
taşır.

## 9. Rapor ve UDF

**Rapor editörü** ekranında "Raporu ve UDF'yi üret" düğmesine basın.
Uygulama şunları üretir:

| Dosya | Rol |
|---|---|
| `BILIRKISI_RAPORU.udf` | **Birincil rapor** (UYAP teslimi için) |
| `BILIRKISI_RAPORU.pdf` | Yardımcı çıktı |
| `BILIRKISI_RAPORU.docx` | Yardımcı çıktı |
| `BILIRKISI_RAPORU.txt` | Yardımcı çıktı |
| `BILIRKISI_RAPORU.json` | Yardımcı çıktı (kanonik veri) |

Raporu önce DOCX olarak oluşturup başka programda UDF'ye dönüştürmeniz
**gerekmez.** UDF doğrudan üretilir.

**Zorunlu alanlar kaldırılamaz.** Bir zorunlu bölümü gizlemeye çalışırsanız
sistem uyarır.

Rapor üretilirken UDF otomatik doğrulanır. Doğrulama başarısızsa dosya
"hazır" olarak sunulmaz ve hata listesi gösterilir.

## 10. UDF önizleme

**UDF önizleme ve kontrol** ekranında ürettiğiniz UDF'yi sayfa sayfa,
yazdırma düzeninde görürsünüz. Metin içinde arama yapabilirsiniz. Sayfa
taşmaları ve tablo sorunları işaretlenir.

**Önemli:** Bu önizleme gerçek UYAP Editör görünümünü doğrulamanın yerine
geçmez; bir ön kontrol katmanıdır. Nihai kontrol için dosyayı resmî UYAP
Editör'de açın.

## 11. İmza ve teslim

**İmza ve teslim** ekranında:

1. Teslim kontrol listesini gözden geçirin.
2. Raporu onaylayın.
3. Uygulama imza öncesi dosya hash'ini kaydeder.
4. UDF'yi resmî UYAP Editör'de açıp imzalayın (resmî süreç).
5. İmzalı dosyayı vaka arşivine alın; uygulama dosyanın değişip
   değişmediğini kontrol eder.
6. UYAP Portal üzerinden dosyayı **kendi yetkinizle** teslim edin.
7. Portal teslim bilgisini uygulamaya kaydedin.

**Önemli:** Uygulama kendi ekranındaki bir onay kutusunu gerçek e-imza gibi
sunmaz. PIN, kart parolası veya gizli anahtar saklamaz. UYAP Portal'a otomatik
giriş yapmaz.

## 12. Yardımcı belgeler

**Ek raporlar ve yazışmalar** ekranında şu belgeleri doğrudan UDF olarak
üretebilirsiniz:

- Ek rapor (önceki raporu değiştirmez; yeni sürüm oluşturur)
- Süre uzatım talebi
- Eksik materyal talebi
- Ek kayıt/belge talebi
- Görev kapsamının açıklığa kavuşturulması talebi
- Görevden çekilme/mazeret bildirimi
- Materyal teslim tutanağı
- Materyal iade tutanağı
- Dizi pusulası
- Rapor ek listesi
- Teslim kontrol listesi

## 13. Takvim ve ücretler

**Takvim ve ücretler** ekranında süreleri, ücretleri ve giderleri takip
edersiniz. Portalda gördüğünüz tutarlar resmî veri gibi gösterilmez; sizin
girdiğiniz tutar ile portal bilgisi ayrı kaynak türleri olarak saklanır.

Ücret tarifeleri sürüm ve yürürlük tarihiyle tutulur. Yeni tarife geldiğinde
eski dosyaların hesapları geriye dönük değişmez.

## 14. Yedekleme

**Yedekleme** ekranında "Yedek al ve geri yükleme testi yap" düğmesine basın.
Uygulama:

1. Şifreli yedek oluşturur (hash manifesti dahil).
2. Yedeği geçici bir konuma **gerçekten geri yükler.**
3. Bütünlüğü doğrular ve sonucu bildirir.

Yalnızca yedek almak yeterli değildir; gerçek geri yükleme testi yapılır.

## 15. Arşiv

**Arşiv** ekranında tamamlanan dosyaları arşivlersiniz. Arşivlenen dosyanın
tüm belgeleri, sesleri, transkriptleri, raporları ve işlem geçmişi korunur.

## 16. İşlem geçmişi

Dosya çalışma alanında "İşlem geçmişini doğrula" düğmesine basın. Uygulama
tüm işlemlerin hash zincirini doğrular.

**Önemli:** Yerel hash zinciri, sonradan yapılan değişiklikleri tespit etmeye
yardım eder. Ancak **harici güvenilir zaman damgası veya nitelikli elektronik
imza olmadan bağımsız ve değiştirilemez zaman damgası olarak sunulamaz.**
Raporda bu sınır açıkça belirtilir.

## 17. Kısayollar

| Kısayol | İşlev |
|---|---|
| `Ctrl+S` | Kaydet (otomatik kaydetme açıktır) |
| `Ctrl+D` | Ana kontrol paneli |
| `Ctrl+R` | Rapor ve UDF üret |
| `Ctrl+B` | Yedek al |
| `F5` | Verileri yenile |

## 18. Sık sorulan sorular

**Ses dosyası bulunamadı diyor, ne yapmalıyım?**
Materyal yolunun doğru olduğunu kontrol edin. Desteklenmeyen bir codec varsa
hata nedeni bildirilir.

**Transkript çok yavaş üretiliyor.**
CPU üzerinde çalışmak normaldir. GPU varsa otomatik kullanılır. Uzun kayıtlar
parçalara ayrılarak işlenir; kesinti olursa kaldığı yerden devam eder.

**Bir bölümü anlamadım.**
Sistem kelime uydurmaz; `[anlaşılamayan bölüm]` gibi işaretler koyar. Bu
işaretleri düzenleyebilirsiniz.

**Model güven puanı %90 diyor, bu doğruluk mu?**
Hayır. Bu gösterge yalnızca inceleme önceliği belirlemek içindir; gerçek
doğruluk yüzdesi değildir ve onayınızın yerine geçmez.

**UDF'yi Word'de açabilir miyim?**
Hayır. UDF, UYAP Editör ile açılır. Word için DOCX yardımcı çıktısı vardır;
ancak bu dosya UDF'nin yerine geçmez.

**İnternet bağlantısı gerekli mi?**
Hayır. Tüm ana işlevler çevrimdışı çalışır. İnternet yalnızca güncelleme için
kullanılır.
