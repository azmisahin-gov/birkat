# Desteklenen Ses Formatı ve Codec Matrisi

Bu belge, BİRKÂT'ın desteklediği ses kapsayıcılarını ve codec'leri listeler
(şartname 7.1, 25. teslim kalemi 22).

## 1. Destek matrisi

| Kapsayıcı | Codec | Durum | Yöntem | Not |
|---|---|---|---|---|
| WAV | PCM (u8, s16, s24, s32, f32, f64) | ✓ Destekleniyor | Symphonia | BWF (Broadcast Wave) dahil |
| MP3 | MPEG-1/2 Layer III | ✓ Destekleniyor | Symphonia | ID3 etiketleri okunur |
| M4A | AAC, ALAC | ✓ Destekleniyor | Symphonia | ISO Base Media |
| AAC | AAC (ADTS) | ✓ Destekleniyor | Symphonia | |
| FLAC | FLAC | ✓ Destekleniyor | Symphonia | Kayıpsız |
| OGG | Vorbis, Opus | ✓ Destekleniyor | Symphonia | |
| Opus | Opus | ✓ Destekleniyor | Symphonia | OGG kapsayıcısında |
| AIFF | PCM | ✓ Destekleniyor | Symphonia | |
| CAF | PCM | ✓ Destekleniyor | Symphonia | |
| MP4 | AAC, ALAC (ses akışı) | ✓ Destekleniyor | Symphonia | Video akışları yok sayılır |
| MOV | AAC, PCM (ses akışı) | ✓ Destekleniyor | Symphonia | |
| MKV | AAC, Vorbis, Opus (ses akışı) | ✓ Destekleniyor | Symphonia | |
| WebM | Vorbis, Opus (ses akışı) | ✓ Destekleniyor | Symphonia | |
| WMA | Windows Media Audio | ⚠ Adaptör gerekir | Windows Media Foundation | Lisans koşulları uygun paketlenmiş adaptör ile |
| AMR | Adaptive Multi-Rate | ⚠ Adaptör gerekir | Windows Media Foundation | Aynı |
| AVI | PCM, MP3 (ses akışı) | ⚠ Kısmi | Symphonia | Tespit edilir; çözümleme codec'e bağlı |
| S01 | SMART (EWF) | — | `ewf-image` | İmaj formatı; ses değil |

**Kural:** Desteklenmeyen bir codec varsa dosya **atlanmaz**; açık hata nedeni
gösterilir (`Readability::UndecodableCodec`). Bu davranış
`inventory::tests::unreadable_audio_is_reported_with_reason` testiyle
korunur.

## 2. Tespit yöntemi

Dosya türü **yalnızca uzantıya göre değerlendirilmez.** Gerçek dosya imzası
(magic bytes) kontrol edilir:

| Biçim | İmza |
|---|---|
| WAV | `RIFF....WAVE` |
| AVI | `RIFF....AVI ` |
| OGG | `OggS` |
| FLAC | `fLaC` |
| AIFF | `FORM....AIFF` |
| CAF | `caff` |
| MP3 | `ID3` veya MPEG çerçeve eşitleme (`0xFF 0xEx`) |
| MP4/M4A/MOV | `....ftyp` + marka |
| MKV/WebM | EBML (`1A 45 DF A3`) |
| AMR | `#!AMR` |
| WMA/ASF | ASF başlık GUID'i |
| EWF | `EVF\x09\x0d\x0a\xff\x00` |

Testler: `detect::tests::*` (12 test).

## 3. Metadata alanları

Her ses için kaydedilen alanlar (şartname 7.2):

| Alan | Kaynak | Yoksa |
|---|---|---|
| Kaynak dosya kimliği | Uygulama | — |
| Dosya adı, boyut | Dosya sistemi | — |
| SHA-256 | Hesaplanır | — |
| Kapsayıcı, codec | Symphonia | "bilinmiyor" |
| Süre | Symphonia | `null` + uyarı |
| Örnekleme frekansı | Symphonia | `null` + uyarı |
| Bit derinliği | Symphonia | `null` + uyarı |
| Bit hızı | Hesaplanır (bits × rate × kanal) | `null` + uyarı |
| Kanal sayısı | Symphonia | `null` |
| Kanal düzeni | Symphonia | `null` |
| Metadata etiketleri | Dosya etiketleri | Boş liste |
| Kodlayıcı bilgisi | Dosya etiketi (`EncodedBy`) | `null` |
| Dosya sistemi zamanları | Dosya sistemi | `null` |
| Akış başlangıç/bitiş | Symphonia | `null` |
| Okunabilirlik | Çözümleme sonucu | — |
| Hata/uyarılar | Toplanır | Boş liste |

**Kaynak dosyada olmayan metadata üretilmez.** `AudioMetadata::unknown_fields()`
eksik alanları listeler ve rapora "kaynakta yok" olarak girer.

Testler: `probe::tests::probes_wav_metadata`,
`audio::tests::unknown_metadata_fields_are_reported`.

## 4. Çoklu akış davranışı

MP4/MOV/MKV/WebM gibi kapsayıcılarda birden fazla akış bulunabilir. BİRKÂT:

- Ses codec'li ilk akışı seçer (`codec != CODEC_TYPE_NULL`).
- Video ve altyazı akışlarını yok sayar.
- Seçilen akışın indeksini `AudioAsset::stream_index` içinde kaydeder.

## 5. Yeniden örnekleme

Transkripsiyon motoru 16 kHz mono girdi bekler. Kaynak farklı frekansta ise
doğrusal interpolasyonla yeniden örneklenir. Bu işlem **yalnızca çıkarım
girdisi** içindir; orijinal dosya değiştirilmez ve raporda orijinal teknik
metadata esas alınır.

Testler: `whisper_candle::tests::resample_upsamples_length`,
`resample_preserves_ramp_shape`.

## 6. Performans notları

- Çözümleme paket paket yapılır; dosya tamamen belleğe yüklenmez.
- Envanter hash'i 1 MiB bloklarla akış hâlinde hesaplanır.
- Dalga biçimi kova tabanlıdır (hedef kova sayısı ekran genişliğine göre).
- Spektrogram STFT ile hesaplanır (FFT boyutu 2'nin kuvvetine yuvarlanır).

## 7. Sınırlamalar

| Sınırlama | Açıklama |
|---|---|
| WMA/AMR | Windows Media Foundation adaptörü gerektirir; bu ortamda adaptör bağlanmadı, açık hata verir |
| Şifreli ses | Desteklenmez; açık hata verir |
| Çok kanallı (5.1+) | Okunur; transkripsiyon için mono'ya indirgenir |
| Bozuk çerçeve | Atlanır ve uyarı toplanır; çözümleme durdurulmaz |
