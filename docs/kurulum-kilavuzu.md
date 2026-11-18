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

Kurulum paketi `BIRKAT-0.1.0-win64.exe` tek dosyadır. İçeriği:

```text
BIRKAT-0.1.0-win64.exe
├── birkat-desktop.exe       Masaüstü uygulaması
├── birkat.exe               Komut satırı aracı
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

Kurulum sırasında:

1. Yönetici yetkisi istenir (Program Files'a kurulum için).
2. Kurulum yeri seçilir (varsayılan: `C:\Program Files\BIRKAT`).
3. Kısayollar oluşturulur (Başlat menüsü ve masaüstü).
4. Model dosyaları kurulur (bu adım birkaç dakika sürebilir).
5. Paket hash manifesti doğrulanır.

## 3. İlk çalıştırma

1. Uygulamayı açın. İlk açılışta geliştirici aracı, Cargo, Rust veya internet
   **istenmez.**
2. **Bilirkişi profili** oluşturun: ad soyad, unvan, sicil numarası, iletişim.
3. **Veri kök dizinini** seçin (varsayılan: `C:\Users\<kullanıcı>\BIRKAT`).
4. **Yedek dizinini** seçin (önerilir: farklı bir disk veya ağ konumu).
5. **Kasa parolasını** belirleyin (en az 12 karakter, harf + rakam).
   Bu parola hiçbir yere kaydedilmez; kaybedilirse kurtarma anahtarı gerekir.
6. **Kurtarma anahtarını** güvenli bir yere yazın (yazdırın ve kasada saklayın).

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
