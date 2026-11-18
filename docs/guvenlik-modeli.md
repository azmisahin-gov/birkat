# Güvenlik Modeli

Bu belge, BİRKÂT'ın tehdit modelini, şifreleme mimarisini, anahtar yönetimini
ve güvenlik sınırlarını tanımlar (şartname 19).

---

## 1. Temel ilkeler

1. **Yerel ve çevrimdışı.** Ana işlevlerin tamamı internet olmadan çalışır.
2. **Veri dışarı çıkmaz.** Görevlendirme belgeleri, dosya numaraları, taraf
   bilgileri, ses kayıtları, transkriptler, bilirkişi notları, raporlar, delil
   hash kayıtları ve imaj dosyaları hiçbir sunucuya gönderilmez.
3. **En az yetki.** Uygulama yönetici yetkisi olmadan çalışır; yalnızca yazma
   koruması uygulaması gibi belirli işlemler için yönetici yetkisi ister ve bu
   gereksinimi kullanıcıya açıkça bildirir.
4. **Şüphede reddet.** Doğrulanamayan koruma "korumalı" sayılmaz;
   imzalanmamış güncelleme uygulanmaz; doğrulanmayan UDF "hazır" gösterilmez.
5. **Dürüst sınırlar.** Güvenlik mekanizmalarının ne **kanıtlamadığı** açıkça
   belgelenir.

---

## 2. Tehdit modeli

| Tehdit | Varlık | Karşı önlem |
|---|---|---|
| Vaka verisine yetkisiz erişim (disk çalınması) | Kasa, veritabanı | AES-256-GCM şifreli blob'lar; parola ile korunan anahtar |
| Zayıf parola tahmini | Kasa anahtarı | Argon2id (64 MiB, 3 yineleme); parola politikası (≥12 karakter) |
| Kaba kuvvet denemesi | Uygulama kilidi | Başarısız deneme politikası (5 deneme → 5 dakika kilit) |
| Bellek dökümünden anahtar sızdırma | Oturum anahtarı | `zeroize` ile anahtar bellekten silinir; `Debug` çıktısı gizler |
| Veri bütünlüğünün bozulması | Kasa, yedek, işlem geçmişi | AEAD etiketi; SHA-256 manifesti; hash zinciri |
| İşlem geçmişinin değiştirilmesi | Audit log | Hash zinciri; her kayıt öncekine bağlı |
| Zararlı arşiv (zip slip, arşiv bombası) | Arşiv açma | Yol kaçışı denetimi; sıkıştırma oranı ve boyut sınırı |
| Zararlı güncelleme paketi | Güncelleme | İmza kapısı (imzasız paket reddedilir); paket SHA-256 |
| Yetkisiz portal erişimi | UYAP Portal | Uygulama otomatik giriş yapmaz; parola saklamaz |
| Sahte e-imza | Rapor | Uygulama kendi onay kutusunu imza gibi sunmaz; PIN/anahtar saklamaz |
| Gizli dışa veri aktarımı | Tümü | Hiçbir ağ çağrısı yok; güncelleme dışında bağlantı kurulmaz |

---

## 3. Şifreleme mimarisi

### 3.1 Şifreli blob biçimi

```text
[ "BKAT" (4) ][ sürüm (1) ][ tuz (16) ][ nonce (12) ][ şifreli metin + GCM etiketi ]
```

- **AEAD:** AES-256-GCM. Her blob için rastgele 96-bit nonce üretilir.
- **Anahtar türetme:** Argon2id, 64 MiB bellek, 3 yineleme, 1 paralellik.
- **Tuz:** Her anahtar türetmede 16 bayt rastgele tuz.
- **Sürüm numarası başlıkta taşınır;** KDF parametreleri başlığa
  **yazılmaz.** Bu, başlıktan gelen parametrelerle zayıflatma saldırılarını
  önler.
- **Etiket doğrulaması:** Yanlış anahtar veya bozulmuş veri çözme sırasında
  reddedilir (sabit zamanlı karşılaştırma).

Testler: `crypto::tests::encrypt_decrypt_round_trip`,
`tampered_blob_fails_authentication`, `wrong_key_fails_to_decrypt`.

### 3.2 Anahtar hiyerarşisi

```text
Parola (kullanıcı, saklanmaz)
    │
    │ Argon2id + tuz
    ▼
Sarma anahtarı
    │
    │ AES-256-GCM (anahtar sarma)
    ▼
Kasa anahtarı (rastgele 32 bayt, key.enc içinde sarılı)
    │
    │ AES-256-GCM
    ▼
Kasadaki nesneler (ses, imaj, transkript, rapor)
```

- Kasa anahtarı rastgele üretilir ve kullanıcı parolasıyla **sarılır**
  (`wrap_vault_key`). Parola değiştirildiğinde yalnızca sarma katmanı yeniden
  üretilir; nesneler yeniden şifrelenmez.
- Kasa anahtarı yalnızca oturum süresince bellekte tutulur; kasa
  kilitlendiğinde (`lock()`) düşürülür.

### 3.3 Parola politikası

| Ölçüt | Değer |
|---|---|
| Asgari uzunluk | 12 karakter |
| Rakam | En az bir |
| Harf | En az bir |
| Zayıf ifade | `123456`, `password`, `parola`, `qwerty`, `birkat` yasak |

Testler: `crypto::tests::password_policy_rejects_weak`.

### 3.4 Anahtar bellekten silme

`SecretKey`, `Zeroize` ve `ZeroizeOnDrop` türetir. Düşürüldüğünde bellek
sıfırlanır. `Debug` çıktısı anahtar malzemesini göstermez
(`SecretKey(<gizli>)`).

---

## 4. Bütünlük ve denetim

### 4.1 Hash

| Algoritma | Kullanım |
|---|---|
| SHA-256 | **Zorunlu**; tüm dosya, imaj, rapor ve manifestolarda |
| SHA-1 | Eski imaj formatlarıyla uyumluluk ve karşılaştırma |
| MD5 | Eski imaj formatlarıyla uyumluluk ve karşılaştırma |

Her hash kaydı şunları taşır: dosya kimliği, algoritma, değer, hesaplama
zamanı, hesaplama aşaması, kaynak/hedef türü ve doğrulama sonucu.

### 4.2 İşlem geçmişi hash zinciri

Her olay önceki olayın hash'ini taşır:

```text
hash(n) = SHA-256( sıra ‖ kimlik ‖ işlem ‖ aktör ‖ açıklama ‖ konu ‖
                   detay ‖ zaman ‖ hash(n-1) )
```

İlk kayıt 64 sıfırdan oluşan `GENESIS_HASH` değerine bağlanır.

**Dürüstlük sınırı (önemli):** Yerel hash zinciri, sonradan yapılabilecek
değişiklikleri tespit etmeye yardım eder. Ancak **harici güvenilir zaman
damgası veya nitelikli elektronik imza olmadan bağımsız ve değiştirilemez
zaman damgası olarak sunulamaz.** Bu sınır `ChainVerification::is_independent_timestamp`
alanıyla (`false`) taşınır ve `birkat denetim-dogrula` çıktısında açıkça
yazdırılır.

Testler: `audit::tests::tampering_with_description_is_detected`,
`removing_an_event_is_detected`, `reordering_events_is_detected`,
`verification_is_not_an_independent_timestamp`.

---

## 5. Yedekleme güvenliği

Yedek, iç ZIP paketinin **tamamının** AES-256-GCM ile şifrelenmesiyle
oluşturulur. Böylece hem dosya içerikleri hem de **dosya adları** korunur.

Yedek manifestosu:

- Biçim sürümü, uygulama sürümü, veri modeli sürümü.
- Vaka kimlikleri.
- Her dosyanın göreli yolu, boyutu ve SHA-256'sı.
- Model sürümü ve işlem metaverisi özeti.
- Manifestonun kendi SHA-256'sı.

**Geri yükleme sırasında her dosyanın hash'i yeniden hesaplanır ve manifestoyla
karşılaştırılır.** Uyuşmayan dosya bildirilir ve geri yükleme başarılı
sayılmaz.

Testler: `backup::tests::backup_is_encrypted`, `wrong_password_fails`,
`tampered_backup_is_rejected`, `verify_restore_tests_integrity`.

---

## 6. Güncelleme güvenliği

Güncelleme akışı ve kapıları:

| Aşama | Kapı | Başarısızlık davranışı |
|---|---|---|
| Meşguliyet | Aktif transkripsiyon/edinim yok | Güncelleme uygulanmaz |
| Bütünlük | Paket boyutu ve SHA-256 eşleşir | Reddedilir |
| İmza | İmza dolu; anahtar tanınır; imza geçerli | Reddedilir |
| Ön yedek | Uygulama kökü kopyalanır | Kurulum yapılmaz |
| Kurulum | Yol kaçışı yok; aşamalı klasöre açılır | Otomatik geri dönüş |
| Migrasyon | Başarılı olmalı | Otomatik geri dönüş |

**İmza kapısı katıdır:** İmza doğrulaması etkin değilse güncelleme
**reddedilir**. Bu, sessizce imzasız paket uygulanmasını önler.

Ağ kullanımı yalnızca güncelleme metadata ve paketi içindir; vaka verileri ve
kullanıcı belgeleri gönderilmez.

Testler: `updater::tests::unsigned_package_is_rejected`,
`signature_verification_blocks_unsigned_update`,
`tampered_package_fails_integrity`, `install_and_rollback`,
`busy_updater_refuses_to_update`.

---

## 7. Arşiv güvenliği

| Risk | Önlem |
|---|---|
| Yol kaçışı (zip slip) | `..`, mutlak yol, sürücü harfi, UNC, NUL reddedilir |
| Arşiv bombası | Sıkıştırma oranı (1000x) ve toplam boyut (64 GiB) sınırı |
| Aşırı girdi | 100 000 girdi sınırı |
| Sembolik bağlantı/aygıt | Atlanır ve bildirilir |
| Çıktı hedef dışına çıkma | Çıkarma sonrası yol öneki denetimi |

Testler: `archive::tests::traversal_names_are_rejected`,
`rejects_compression_bomb_ratio`, `rejects_too_many_entries`,
`safe_join_blocks_escape`.

---

## 8. Delil edinimi güvenliği

### 8.1 Yazma koruması

Koruma kaynağı **açıkça sınıflandırılır** ve yazılımsal koruma donanımsal
korumaya eşdeğer **gösterilmez**.

**Dürüstlük sınırı:** İşletim sistemi aygıtı uygulama algılamadan önce
bağlayabileceği için, sonradan yapılan koruma doğrulaması ilk bağlantıdan
itibaren hiç yazma yapılmadığının kanıtı olarak sunulamaz. Bu nedenle yüksek
güvence gerektiren işlemlerde donanımsal yazma engelleyici kullanımı
desteklenir ve kullanıcıya bildirilir.

Testler: `write_protection::tests::*` (10 test).

### 8.2 Dosya bazlı edinim

- Kaynak hash'i **kopyalamadan önce** alınır.
- Hedef kopya **tekrar okunur** ve hash karşılaştırılır.
- Uyuşmazlık hata olarak raporlanır.
- Hedefte var olan dosyanın üzerine **yazılmaz** (varsayılan).
- Dosya bazlı kopya, fiziksel imaj veya L01 imajı olarak
  **adlandırılmaz**.

### 8.3 EWF imajı

**Dürüstlük sınırı:** Bir imajın hash doğrulamasının başarılı olması, kaynak
aygıtın tarihsel olarak orijinal olduğunu veya okunamayan sektörlerin doğru
içerikle elde edildiğini kendiliğinden kanıtlamaz.

Okunamayan sektörler kaydedilir ve raporda "doğru içerikle elde edilmiş
sayılmaz" notuyla sunulur.

---

## 9. Kullanıcı kilidi

| Özellik | Davranış |
|---|---|
| Uygulama kilidi | Kasa parolasıyla açılır |
| Otomatik kilitleme | Ayarlanabilir süre (varsayılan 15 dakika) |
| Oturum kapatma | Kasa anahtarı bellekten düşürülür |
| Vaka kasası kapatma | `CaseWorkspace::lock()` |
| Başarısız parola denemesi | 5 deneme → 5 dakika kilit |
| Güvenli anahtar kullanımı | Anahtar hiçbir yere yazılmaz; parola günlüklenmez |

---

## 10. Gizlilik ve veri saklama

- Uygulama dosyaları kendiliğinden ve kullanıcının bilgisi dışında **silmez.**
- Kullanıcı saklama politikası tanımlayabilir (`RetentionPolicy`).
- İmzalı rapor, orijinal materyal ve işlem geçmişi **ayrı saklama
  sınıflarında** tutulur.
- İmzalı rapor değiştirilemez bir artefakt olarak saklanır; değiştirme
  girişimi hata döndürür (`ReportDocument::ensure_editable`).

---

## 11. Günlükleme

- Günlükler `tracing` ile tutulur; varsayılan düzey `info`.
- **Parola, PIN, anahtar veya kişisel veri günlüğe yazılmaz.**
- Hata mesajları hassas bilgi sızdırmaz (dosya yolları ve hata nedenleri
  gösterilir; anahtar malzemesi gösterilmez).

---

## 12. Doğrulama listesi

| Kontrol | Durum | Kanıt |
|---|---|---|
| Çevrimdışı çalışma | ✓ | Hiçbir ağ çağrısı yok |
| Buluta veri gönderimi yok | ✓ | `updater::network_note`, kod incelemesi |
| AES-256-GCM AEAD | ✓ | `crypto::tests::encrypt_decrypt_round_trip` |
| Argon2id anahtar türetme | ✓ | `crypto::tests::password_round_trip` |
| Anahtar bellekten silme | ✓ | `SecretKey: ZeroizeOnDrop` |
| Parola politikası | ✓ | `crypto::tests::password_policy_rejects_weak` |
| Deneme kilidi | ✓ | `crypto::tests::lockout_policy_locks_after_max_attempts` |
| Şifreli kasa | ✓ | `vault::tests::encrypted_vault_round_trip` |
| Şifreli yedek + gerçek geri yükleme | ✓ | `backup::tests::verify_restore_tests_integrity` |
| Hash zinciri bütünlüğü | ✓ | `audit::tests::*` |
| Arşiv güvenliği | ✓ | `archive::tests::*` |
| İmza kapısı | ✓ | `updater::tests::unsigned_package_is_rejected` |
| Otomatik geri dönüş | ✓ | `updater::tests::install_and_rollback` |
| Yazma koruması dürüst sınıflandırma | ✓ | `write_protection::tests::*` |
| İmzalı rapor kilitli | ✓ | `report::tests::signed_report_is_locked` |
| Parola günlüğe yazılmaz | ✓ | Kod incelemesi |
