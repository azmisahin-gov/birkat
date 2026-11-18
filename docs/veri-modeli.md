# Veri Modeli

Bu belge, BİRKÂT'ın kalıcı veri modelini ve sürümleme stratejisini tanımlar
(şartname 21).

## 1. Varlık listesi

Şartname 21.1'de istenen 36 varlığın tamamı uygulanmıştır.

| Şartname varlığı | Gerçek tip | Konum |
|---|---|---|
| ExpertProfile | `ExpertProfile` | `birkat-core::profile` |
| Case | `Case` | `birkat-core::case` |
| Assignment | `Assignment` | `birkat-core::case` |
| AssignmentQuestion | `AssignmentQuestion` | `birkat-core::case` |
| Party | `Party` | `birkat-core::case` |
| CourtOrAuthority | `CourtOrAuthority` | `birkat-core::case` |
| Deadline | `Deadline` | `birkat-core::case` |
| EvidenceItem | `EvidenceItem` | `birkat-core::evidence` |
| PhysicalDevice | `PhysicalDevice` | `birkat-core::evidence` |
| AcquisitionOperation | `AcquisitionOperation` | `birkat-core::evidence` |
| EvidenceImage | `EvidenceImage` | `birkat-core::evidence` |
| FileObject | `FileObject` | `birkat-core::evidence` |
| AudioAsset | `AudioAsset` | `birkat-core::audio` |
| AudioMetadata | `AudioMetadata` | `birkat-core::audio` |
| TranscriptionJob | `TranscriptionJob` | `birkat-core::audio` |
| ModelVersion | `ModelVersion` | `birkat-core::audio` |
| Transcript | `Transcript` | `birkat-core::audio` |
| TranscriptSegment | `TranscriptSegment` | `birkat-core::audio` |
| WordTimestamp | `WordTimestamp` | `birkat-core::audio` |
| SpeakerLabel | `SpeakerLabel` | `birkat-core::audio` |
| ReviewMark | `ReviewMark` | `birkat-core::audio` |
| ExpertNote | `ExpertNote` | `birkat-core::audio` |
| Finding | `Finding` | `birkat-core::findings` |
| EvidenceReference | `EvidenceReference` | `birkat-core::findings` |
| Answer | `Answer` | `birkat-core::findings` |
| ReportDocument | `ReportDocument` | `birkat-core::report` |
| ReportVersion | `TranscriptVersion` / `ReportDocument::version` | `birkat-core` |
| UdfArtifact | `UdfDocument` | `birkat-udf::model` |
| Attachment | `Attachment` | `birkat-core::report` |
| SignatureRecord | `SignatureRecord` | `birkat-core::report` |
| DeliveryRecord | `DeliveryRecord` | `birkat-core::report` |
| Expense | `Expense` | `birkat-core::admin` |
| FeeRecord | `FeeRecord` | `birkat-core::admin` |
| AuditEvent | `AuditEvent` | `birkat-audit` |
| Backup | `BackupManifest` | `birkat-backup` |
| UpdateRecord | `UpdateRecord` | `birkat-updater` |

Ek tipler: `CaseVault` / `VaultObject` (kasa), `AcquisitionManifest`,
`CustodyRecord` (fiziksel teslim), `PanelMemberAssignment` (heyet),
`FeeTariff` (ücret tarifesi), `CalendarEvent`, `QualityReport` /
`QualityFinding` (kalite kontrolü), `ProcessingStep` (türetilmiş ses).

## 2. İçerik durumu (şartname A.2)

Üç durum kesin olarak ayrılır ve birbirine karıştırılmaz:

```rust
pub enum ContentStatus {
    MachineOutput,      // Modelin veya otomatik işlemin ürettiği içerik
    ReadyForReview,     // Teknik kontrolleri yapılmış, onaylanmamış içerik
    ExpertApproved,     // Mesleki inceleme ve onay kaydedilmiş içerik
}
```

Geçiş kuralları (`apply_transition`):

| Geçiş | İzin |
|---|---|
| `MachineOutput` + `QualityControlPassed` | `ReadyForReview` |
| `ReadyForReview` + `ExpertApproved` | `ExpertApproved` |
| `ExpertApproved` + `ApprovalWithdrawn` | `ReadyForReview` |
| Herhangi + `Regenerated` | `MachineOutput` |
| `MachineOutput` + `ExpertApproved` | **REDDEDİLİR** |

Makine çıktısı, teknik kalite kontrolü tamamlanmadan bilirkişi onaylı
sayılamaz. Bu kural `content_status::tests::machine_output_cannot_be_approved_directly`
testiyle korunur.

## 3. Transkript sürümleme (şartname 12.3)

Beş aşama ayrı tutulur; ilk makine çıktısı asla insan düzeltmesiyle üzerine
yazılarak kaybedilmez:

| Sürüm | Değiştirilemez |
|---|---|
| `MachineRaw` | Evet |
| `QualityCheckedDraft` | Hayır |
| `ExpertReviewed` | Hayır |
| `ExpertCorrected` | Hayır |
| `ExpertApprovedFinal` | Evet |

Her değişiklik `TranscriptChange` ile kaydedilir: önceki değer, yeni değer,
zaman, kullanıcı, gerekçe.

## 4. İş akışı durumları (şartname 21.3)

22 durum:

```text
NEW → ASSIGNMENT_IMPORTED → ASSIGNMENT_REVIEW → MATERIALS_PENDING →
MATERIALS_RECEIVED → ACQUISITION → INVENTORY → TRANSCRIPTION_QUEUED →
TRANSCRIPTION_RUNNING → QUALITY_CONTROL → EXPERT_REVIEW →
QUESTIONS_AND_FINDINGS → REPORT_DRAFT → UDF_GENERATION → UDF_VALIDATION →
FINAL_REVIEW → APPROVED → SIGNATURE_PENDING → SIGNED → DELIVERY_PENDING →
SUBMITTED → ARCHIVED
```

Geri dönüş yolları tanımlıdır (kalite kontrolü başarısız, bilirkişi düzeltme
istedi gibi durumlar). `workflow::path_to` BFS ile hedef duruma giden adım
yolunu bulur; otomatik iş akışı her ara durumu kaydeder.

Her görev: kalıcı, tekrarlanabilir, iptal edilebilir, duraklatılabilir, devam
edebilir, hata sonrası güvenli yeniden denenebilir, çift işlem oluşturmaz ve
uygulama kapanınca kaybolmaz.

## 5. Rapor bölümleri (şartname 14.2)

27 kanonik bölüm (`SectionId`). 23'ü zorunludur ve gizlenemez; 4'ü
(ön kapak, kullanılan yazılımlar, yazım yöntemi, zaman kodları) isteğe
bağlıdır. Zorunlu bölümü gizleme girişimi reddedilir ve bildirilir.

## 6. Kalıcılık

| Veri | Depolama | Şifreleme |
|---|---|---|
| Vaka metaverisi | SQLite `cases` tablosu (JSON payload) | Veritabanı dosyası |
| Profil | SQLite `profiles` tablosu | — |
| İşlem geçmişi | SQLite `audit` tablosu | — |
| Ayarlar | SQLite `settings` tablosu | — |
| Kasa nesneleri (ses, imaj, transkript, rapor) | Kasa `data/` klasörü | AES-256-GCM |
| Kasa anahtarı | Kasa `key.enc` | Parola ile sarılı |
| Yedek | Şifreli blob dosyası | AES-256-GCM |
| Model paketi | Kurulum klasörü | — (hash doğrulamalı) |

## 7. Sürümleme stratejisi

| Alan | Sürüm | Nerede |
|---|---|---|
| Uygulama | `APP_VERSION` | `birkat-core` |
| Veri modeli şeması | `MODEL_VERSION` | `birkat-core`, SQLite `meta` tablosu |
| UDF biçimi | `format_id="1.8"` | `birkat-udf` |
| Yedek biçimi | `BACKUP_FORMAT_VERSION` | `birkat-backup` |
| Güncelleme manifestosu | `UPDATE_MANIFEST_VERSION` | `birkat-updater` |
| Model paketi | Model sürümü + dosya hash'i | `birkat-transcription` |

**Göç kuralı:** Yeni alan eklendiğinde `MODEL_VERSION` artırılır ve göç
yordamı yazılır. Eski vakalar okunmaya devam eder. Bilinmeyen alanlar
`serde` varsayılanlarıyla atlanır.

**Geriye dönük hesaplama:** Ücret tarifeleri sürüm ve yürürlük tarihiyle
tutulur. Yeni tarife geldiğinde eski dosyaların hesapları geriye dönük
değişmez (`FeeTariff::is_effective_at`).

## 8. Veri ilişkileri

```text
ExpertProfile ──┐
                ├──► Case ──► Assignment ──► AssignmentQuestion ──┐
CourtOrAuthority┘        │                       │                │
                         │                       ▼                │
                         ├──► Deadline       EvidenceReference ◄──┤
                         │                       ▲                │
                         ├──► EvidenceItem ─► FileObject          │
                         │        │              ▲                │
                         │        ├─► PhysicalDevice              │
                         │        ├─► AcquisitionOperation        │
                         │        └─► EvidenceImage               │
                         │                                       │
                         ├──► AudioAsset ──► Transcript ──────────┤
                         │        │            │                  │
                         │        │            └─► TranscriptSegment
                         │        │                     │         │
                         │        └─► AudioMetadata     ▼         │
                         │                        WordTimestamp   │
                         │                                       ▼
                         ├──► ReportDocument ◄──────────── Finding / Answer
                         │        │
                         │        ├─► Attachment
                         │        ├─► SignatureRecord
                         │        └─► DeliveryRecord
                         │
                         └──► AuditEvent (hash zinciri)
```

## 9. Bilinmeyen veri politikası

- Kaynak dosyada olmayan metadata **üretilmez**; `unknown_fields()` ile
  listelenir.
- İşletim sisteminin sunmadığı aygıt bilgisi **tahmin edilmez**;
  `RawDeviceInfo::unknown_fields()` ile listelenir.
- Model kelime düzeyi zaman kodu üretmiyorsa **sahte zaman üretilmez**;
  `WordTimestamps::SegmentOnly` bildirilir.
- Bilinmeyen XML öğeleri okuma sırasında **atlanır ve bildirilir**; hata
  üretilmez.
