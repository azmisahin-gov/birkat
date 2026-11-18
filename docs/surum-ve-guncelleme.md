# Sürüm ve Güncelleme

## 1. Sürüm numaralandırma

Anlamsal sürümleme (semver) kullanılır: `ANA.YAN.YAMA`.

| Değişiklik | Artış |
|---|---|
| Geriye uyumsuz veri modeli değişikliği | ANA |
| Yeni özellik, geriye uyumlu | YAN |
| Hata düzeltmesi | YAMA |

Sürüm numarası tek yerde tanımlıdır: `Cargo.toml` → `workspace.package.version`.
Kod bu değeri `birkat_core::APP_VERSION` üzerinden okur.

## 2. Sürüm bileşenleri

| Bileşen | Sabit | Artış koşulu |
|---|---|---|
| Uygulama | `APP_VERSION` | Her sürümde |
| Veri modeli şeması | `MODEL_VERSION` | Veri modeli değiştiğinde |
| UDF biçimi | `format_id="1.8"` | UDF biçimi değiştiğinde |
| Yedek biçimi | `BACKUP_FORMAT_VERSION` | Yedek düzeni değiştiğinde |
| Güncelleme manifestosu | `UPDATE_MANIFEST_VERSION` | Manifesto alanları değiştiğinde |
| Model paketi | Model sürümü | Model değiştiğinde (ayrı yönetilir) |

## 3. Sürüm geçmişi

| Sürüm | Tarih | Özet |
|---|---|---|
| 0.1.0 | 2026-11-18 | İlk tam sürüm: 13 paket, 20 çalışma alanı, UDF motoru, EWF desteği, şifreli kasa, otomatik iş akışı |

## 4. Güncelleme akışı

```text
Kullanıcı: güncelleme bildirimini görür
        │
        ▼
Updater::can_update  ──► meşgulse REDDET (aktif transkripsiyon/edinim)
        │
        ▼
verify_integrity     ──► boyut + SHA-256 eşleşmezse REDDET
        │
        ▼
verify_signature     ──► imzasız/tanınmayan/geçersizse REDDET
        │
        ▼
pre_backup           ──► uygulama kökünü kopyala
        │
        ▼
install              ──► aşamalı klasöre aç (yol kaçışı denetimi) → köke taşı
        │
        ▼
migrate              ──► veritabanı migrasyonu
        │
        ├─ başarılı ──► Completed
        └─ başarısız ─► rollback (ön yedeği geri yükle) → RolledBack
```

Her aşama `UpdateRecord` içine kaydedilir: aşama, zaman, ön yedek yolu,
başarı durumu, hata.

## 5. Güncelleme paketi biçimi

Güncelleme paketi bir ZIP arşividir:

```text
guncelleme.zip
├── bin/birkat.exe
├── bin/birkat-desktop.exe
├── belgeler/...
└── ...
```

Manifesto (`UpdateManifest`):

```json
{
  "manifest_version": 1,
  "version": "0.2.0",
  "released_at": "2026-12-01T00:00:00Z",
  "min_compatible_version": "0.1.0",
  "package_file": "guncelleme.zip",
  "package_sha256": "…",
  "package_size_bytes": 12345678,
  "signature": "<base64 Ed25519 imzası>",
  "signing_key_id": "birkat-release-1",
  "release_notes": ["…"],
  "requires_migration": true,
  "model_update": null
}
```

## 6. İmzalama

1. Paket ZIP'i üret.
2. Paketin SHA-256'sını hesapla.
3. SHA-256 özetini Ed25519 özel anahtarıyla imzala.
4. İmzayı base64 olarak manifestoya yaz.
5. Manifestoyu ve paketi yayımla.

Kullanıcı tarafında genel anahtar `Updater::add_trusted_key` ile tanımlanır.
**İmza doğrulanmadan güncelleme uygulanmaz.**

> **Not:** Bu geliştirme derlemesinde Ed25519 kriptografik doğrulaması
> etkinleştirilmemiştir; `verify_signature` katı bir kapı olarak çalışır ve
> imzasız/tanınmayan paketi reddeder. Üretim derlemesinde Ed25519 doğrulaması
> etkinleştirilmelidir (bkz. `docs/kabul-testi-matrisi.md` bölüm D).

## 7. Çevrimdışı güncelleme

1. İmzalı paket ve manifestoyu ayrı bir ortamda indir.
2. Taşınabilir ortamla hedef bilgisayara aktar.
3. `Updater::apply` ile uygula (imza ve bütünlük yine doğrulanır).

## 8. Geri dönüş (rollback)

Güncelleme başarısız olursa:

1. Ön yedek otomatik olarak geri yüklenir.
2. `UpdateRecord.stage` = `RolledBack` olur.
3. Hata kullanıcıya bildirilir.
4. Önceki çalışan sürüm açılır.

Manuel geri dönüş: `Updater::rollback(&pre_backup_path)`.

## 9. Veri uyumluluğu

- Güncelleme mevcut vakaları **bozmaz.**
- Veritabanı migrasyonu `MODEL_VERSION` ile yönetilir.
- Bilinmeyen alanlar `serde` varsayılanlarıyla atlanır.
- İmzalı raporlar değiştirilemez artefaktlardır; güncelleme bunlara dokunmaz.
- Eski vakalar ve imzalı raporlar güncelleme sonrası değişmeden açılır.

## 10. Ağ kullanımı

İnternet **yalnızca** güncelleme metadata ve paketini almak için kullanılır.
Vaka verileri ve kullanıcı belgeleri güncelleme sunucusuna **gönderilmez.**

## 11. Sürüm yayın kontrol listesi

1. [ ] Tüm testler geçiyor (`cargo test --workspace`).
2. [ ] Sürüm numarası `Cargo.toml`'da güncellendi.
3. [ ] `MODEL_VERSION` gerekiyorsa artırıldı ve göç yazıldı.
4. [ ] Sürüm geçmişi bu belgeye eklendi.
5. [ ] SBOM ve lisans belgesi yeniden üretildi.
6. [ ] Windows paketi üretildi ve temiz makinede test edildi.
7. [ ] Paket imzalandı ve manifesto yayımlandı.
8. [ ] Belgeler güncellendi.
