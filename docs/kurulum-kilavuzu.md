# Kurulum Kılavuzu

Bu kılavuz, BİRKÂT'ı Windows 11 x64 bilgisayara kurmayı anlatır.

## 1. Sistem gereksinimleri

| Bileşen | Gereksinim |
|---|---|
| İşletim sistemi | Windows 11 x64 |
| İşlemci | 4 çekirdek (8 çekirdek önerilir) |
| Bellek | 8 GB (16 GB önerilir) |
| Boş disk | 10 GB + model dosyaları (~3 GB) + vaka verisi |
| Ekran | 1280×840 (1920×1080 önerilir) |
| İnternet | **Kurulum ve kullanım için gerekmez** |
| GPU | Opsiyonel (CUDA destekli NVIDIA) |

Windows 10, 14 Ekim 2025'te standart desteğini tamamlamıştır; ana destek
platformu Windows 11'dir.

## 2. Kurulum paketi

**İndirme:** <https://github.com/azmisahin-gov/birkat/releases/latest>

| Dosya | Boyut | SHA-256 (ilk 16) |
|---|---|---|
| `BIRKAT-0.1.0-windows-x64-offline-setup.exe` | 1.505.558.800 bayt | `8cdbf9de1a5bc0a3…` |
| `BIRKAT-0.1.0-windows-x64-offline.zip` | 1.505.468.161 bayt | `7b49648de8f60cc9…` |

Tam hash'ler `SHA256SUMS.txt` içindedir. Doğrulama:

```bash
sha256sum -c SHA256SUMS.txt
```

Paket **internet bağlantısı gerektirmez**; Whisper modeli içindedir.

Kurulum dosyası tek bir NSIS yürütülebilir dosyasıdır. İçeriği:

```text
BIRKAT-0.1.0-windows-x64-offline-setup.exe (içerik)
├── BIRKAT.exe               Masaüstü uygulaması
├── birkat-cli.exe           Komut satırı aracı
├── KURULUM.txt              Kurulum ve ilk açılış notları
├── models/                  Whisper large-v3-turbo model paketi
│   ├── model.safetensors
│   ├── config.json
│   ├── tokenizer.json
│   ├── melfilters.bytes
│   └── LICENSE
├── THIRD-PARTY-LICENSES/    Bağımlılık lisansları
├── belgeler/                Türkçe kullanım ve teknik belgeler
└── manifest.json            Paket hash manifesti
```

Paket 39 dosya ve 1,65 GB'tır. Üretilen `.exe` yalnızca Windows sistem
DLL'lerine bağlıdır; **libgcc, libwinpthread veya libstdc bağımlılığı yoktur.**
Kullanıcı hiçbir çalışma zamanı kurmak zorunda değildir.

Kurulum sırasında:

1. Yönetici yetkisi istenir (Program Files'a kurulum için).
2. Kurulum yeri seçilir (varsayılan: `C:\Program Files\BIRKAT`).
3. Kısayollar oluşturulur (Başlat menüsü ve masaüstü).
4. Model dosyaları kurulur (bu adım birkaç dakika sürebilir).
5. Paket hash manifesti doğrulanır.

## 3. İlk çalıştırma

Uygulama ilk açıldığında **Türkçe bir kurulum sihirbazı** gösterilir.
Sihirbaz yalnızca mesleki bilgileri ve güvenlik tercihini sorar; model,
codec, GPU, Python, FFmpeg veya dosya yolu gibi teknik ayrıntılar
**sorulmaz.**

1. **Hoş geldiniz** — ürün tanıtımı ve ne sorulmadığı.
2. **Bilirkişi profili** — ad soyad, unvan, sicil numarası, kurum, iletişim.
   Bu bilgiler her raporda otomatik kullanılır.
3. **Veri ve yedek klasörü** — veri kök dizini ve yedek dizini.
4. **Kasa parolası** — en az 12 karakter (harf + rakam). Parola gücü anında
   gösterilir. Ardından **kurtarma anahtarı** üretilir; kaydettiğinizi
   onaylamadan ilerleyemezsiniz.
5. **Sistem denetimi** — veritabanı, kasa, Whisper modeli ve derleme desteği
   otomatik denetlenir. Eksik model açıkça bildirilir.

Sihirbaz tamamlandığında uygulama ana kontrol panelini açar.

## 4. Model paketi

Model dosyaları **kurulum paketinde bulunur**; ilk çalıştırmada internetten
indirilmez.

| Dosya | Rol |
|---|---|
| `model.safetensors` | Whisper large-v3-turbo ağırlıkları |
| `config.json` | Model yapılandırması |
| `tokenizer.json` | Belirteçleştirici |
| `melfilters.bytes` | Mel filtre bankası (resmî) |
| `LICENSE` | Model lisansı |

Model paketi `ModelPackage::verify()` ile doğrulanır: her dosyanın varlığı ve
SHA-256'sı kontrol edilir. Doğrulama başarısızsa transkripsiyon başlatılmaz ve
açık hata verilir.

Model güncellemeleri uygulama güncellemelerinden **ayrı** yönetilir.

## 5. GPU (opsiyonel)

CPU üzerinde çalışmak zorunludur ve her zaman çalışır. CUDA destekli NVIDIA
GPU varsa:

1. NVIDIA sürücüsünü güncelleyin.
2. GPU destekli derlemeyi kullanın (`--features cuda`).
3. Uygulama GPU'yu otomatik kullanır.
4. CUDA başlatılamazsa işlem kaybedilmeden CPU'ya geçilir ve bu durum işlem
   kaydına yazılır.

Kullanıcıdan cihaz veya hesaplama modu seçmesi **istenmez.**

## 6. Yükseltme

1. Uygulama güncelleme olduğunu bildirir.
2. Güncelleme **güvenli bir işlem noktasında** uygulanır (aktif transkripsiyon
   veya edilim sürmezken).
3. Ön yedek otomatik alınır.
4. Paket imzası ve bütünlüğü doğrulanır.
5. Kurulum yapılır, veritabanı migrasyonu çalışır.
6. Başarısızlıkta önceki sürüme otomatik geri dönülür.

Çevrimdışı ortamlar için imzalı güncelleme paketi elle indirilip
`birkat-updater` ile uygulanabilir.

## 7. Kaldırma

1. Ayarlar → Uygulamalar → BİRKÂT → Kaldır.
2. Kaldırma sırasında vaka verisi **silinmez.** Veri kök dizini korunur.
3. Vaka verisini silmek istiyorsanız veri kök dizinini elle silin (önce yedek
   alın).

## 8. Sorun giderme

| Belirti | Neden | Çözüm |
|---|---|---|
| Uygulama açılmıyor | Grafik sürücüsü | Grafik sürücüsünü güncelleyin |
| "Model paketi bulunamadı" | Model dosyaları eksik | Kurulumu onarın; `birkat saglik` çıktısını kontrol edin |
| Transkripsiyon çok yavaş | GPU kullanılmıyor | GPU sürücüsünü kontrol edin; CPU normaldir |
| Kasa açılmıyor | Parola hatalı | Kurtarma anahtarıyla açın |
| UDF açılmıyor | Biçim sorunu | `birkat udf-dogrula --dosya <dosya>` çalıştırın |
| Yedek geri yüklenmiyor | Parola hatalı veya yedek bozuk | Parolayı kontrol edin; yedek hash'ini doğrulayın |

Tanılama için:

```cmd
birkat saglik
birkat denetim-dogrula
```

## 9. Destek

- Kullanım kılavuzu: `belgeler/kullanici-kilavuzu.md`
- Teknik mimari: `belgeler/teknik-mimari.md`
- Güvenlik modeli: `belgeler/guvenlik-modeli.md`
