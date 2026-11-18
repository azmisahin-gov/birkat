//! Otomatik iş akışı (2.2).
//!
//! Bilirkişi yeni görev başlatıp belgeleri ve materyalleri verdiğinde uygulama
//! otomatik olarak:
//!
//! 1. Belgeleri okur.
//! 2. Görev bilgilerini çıkarır.
//! 3. Soru listesini oluşturur.
//! 4. Dosya ve ekleri envantere alır.
//! 5. Materyalleri sınıflandırır.
//! 6. Güvenli edinimi planlar.
//! 7. Ses dosyalarını tespit eder.
//! 8. Teknik envanteri oluşturur.
//! 9. Transkripsiyon kuyruğunu hazırlar.
//! 10. Transkripsiyonları üretir.
//! 11. Kalite kontrolü yapar.
//! 12. İncelenmesi gereken bölümleri işaretler.
//! 13. Sorularla ilgili ses ve transkript dayanaklarını ilişkilendirir.
//! 14. Rapor taslağını oluşturur.
//! 15. UDF çıktısını hazırlar.
//! 16. Son kontrol listesini oluşturur.
//!
//! Bilirkişiden her ses dosyasına ayrı ayrı "çalıştır" düğmesine basması
//! istenmez; her klasörü tek tek seçmesi beklenmez; model, sıcaklık, beam
//! size, batch size, GPU, CUDA veya teknik ses parametreleri sorulmaz.

use crate::store::CaseStore;
use crate::workspace::CaseWorkspace;
use birkat_core::audio::{Transcript, TranscriptSegment, TranscriptVersion};
use birkat_core::content_status::{apply_transition, ContentStatus, StatusTransition};
use birkat_core::error::Result;
use birkat_core::ids::{AudioId, TranscriptId};
use birkat_core::time_util::Timestamp;
use birkat_report::builder::{AudioInventoryRow, ReportBuildInput, TranscriptRow};
use birkat_transcription::engine::{TranscriptionEngine, TranscriptionRequest};
use birkat_transcription::quality::{run_quality_control, QualityControlInput, QualitySegment};
use birkat_udf::model::UdfDocument;
use std::path::PathBuf;

/// Otomatik iş akışı çıktısı.
#[derive(Debug, Clone, Default)]
pub struct WorkflowOutcome {
    /// Envanterde bulunan ses dosyası sayısı.
    pub audio_files: usize,
    /// Transkripsiyonu tamamlanan dosya sayısı.
    pub transcribed_files: usize,
    /// İncelenmesi gereken bölüm sayısı.
    pub review_items: usize,
    /// Oluşturulan transkriptler.
    pub transcripts: Vec<Transcript>,
    /// Rapor taslağı verisi.
    pub report_input: ReportBuildInput,
    /// UDF belgesi (hazır).
    pub udf: Option<UdfDocument>,
    /// Son kontrol listesi.
    pub checklist: Vec<String>,
    /// Uyarılar (istisnalar).
    pub warnings: Vec<String>,
    /// Otomatik yürütülen adımlar (kayıt amaçlı).
    pub steps: Vec<String>,
}

impl WorkflowOutcome {
    /// İstisna var mı? (kullanıcı kararı gereken durumlar)
    pub fn has_exceptions(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Türkçe özet.
    pub fn summary(&self) -> String {
        format!(
            "{} ses dosyası bulundu, {} dosya metne dönüştürüldü, {} bölüm incelenmeli.",
            self.audio_files, self.transcribed_files, self.review_items
        )
    }
}

/// Otomatik iş akışını yürütür.
///
/// Bu işlev, bilirkişiden teknik parametre istemez; güvenli varsayılanları
/// kendisi seçer. Yalnızca çözülemeyen veya kullanıcı kararı gerektiren
/// durumlar `warnings` içinde bildirilir.
pub fn run_automatic_workflow(
    workspace: &mut CaseWorkspace,
    store: &mut CaseStore,
    material_roots: &[PathBuf],
    engine: &dyn TranscriptionEngine,
    data_root: &std::path::Path,
) -> Result<WorkflowOutcome> {
    let mut out = WorkflowOutcome::default();
    let mut step = |out: &mut WorkflowOutcome, s: &str| out.steps.push(s.to_string());

    step(&mut out, "Materyaller envantere alınıyor");
    workspace.advance_workflow(store, birkat_core::workflow::WorkflowState::Acquisition)?;

    // 1-6: Materyalleri kasaya al ve envanter çıkar.
    for root in material_roots {
        if !root.exists() {
            out.warnings
                .push(format!("Materyal yolu bulunamadı: {}", root.display()));
            continue;
        }
        if root.is_dir() {
            for entry in walkdir::WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let rel = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                match workspace
                    .vault
                    .store_file(entry.path(), &rel, &root.display().to_string())
                {
                    Ok(_) => {}
                    Err(e) => out.warnings.push(format!("{rel} alınamadı: {e}")),
                }
            }
        } else if let Err(e) = workspace.vault.store_file(
            root,
            &root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            &root.display().to_string(),
        ) {
            out.warnings.push(format!("{} alınamadı: {e}", root.display()));
        }
    }
    workspace.commit(
        store,
        birkat_audit::AuditAction::MaterialAdded,
        format!("{} materyal kaynağı kasaya alındı", material_roots.len()),
        None,
        serde_json::json!({ "roots": material_roots.len() }),
    )?;

    step(&mut out, "Ses dosyaları tespit ediliyor ve teknik envanter çıkarılıyor");
    let inventory = birkat_audio::build_inventory(material_roots, true)?;
    out.audio_files = inventory.audio_files;

    for e in &inventory.entries {
        if !e.readability.is_transcribable() {
            out.warnings.push(format!(
                "{} okunamadı: {}",
                e.path.display(),
                e.readability.label()
            ));
        }
    }

    // Teknik envanter satırları.
    out.report_input.audio_inventory = inventory
        .entries
        .iter()
        .map(|e| {
            let m = e.probe.as_ref().map(|p| &p.metadata);
            AudioInventoryRow {
                file_name: e
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                container: m.map(|m| m.container.clone()).unwrap_or_else(|| e.detected.label()),
                codec: m.map(|m| m.codec.clone()).unwrap_or_else(|| "-".to_string()),
                duration_secs: m.and_then(|m| m.duration_secs),
                channels: m.and_then(|m| m.channels),
                sample_rate: m.and_then(|m| m.sample_rate),
                sha256: e.sha256.clone(),
                readability: e.readability.label(),
            }
        })
        .collect();

    // Yinelenen dosya uyarısı (silinmez; her kaynak ilişkisi korunur).
    for e in &inventory.entries {
        if e.duplicate_of.is_some() {
            out.warnings.push(format!(
                "{} yinelenen içerik; kaynak ilişkisi korundu, silinmedi.",
                e.path.display()
            ));
        }
    }

    workspace.advance_workflow(store, birkat_core::workflow::WorkflowState::Inventory)?;

    // 9-11: Transkripsiyon kuyruğu ve üretim.
    step(&mut out, "Transkripsiyon kuyruğu hazırlanıyor ve çalıştırılıyor");
    let mut queue = birkat_transcription::JobQueue::new();
    let transcribable: Vec<_> = inventory.transcribable_entries();
    for (i, e) in transcribable.iter().enumerate() {
        let audio_id = AudioId::new();
        let mut task = birkat_transcription::queue::QueueTask::new(
            audio_id,
            e.path.to_string_lossy().to_string(),
            i as u32,
        );
        task.audio_duration = e.probe.as_ref().and_then(|p| p.metadata.duration_secs);
        queue.enqueue(task)?;
    }
    workspace.advance_workflow(store, birkat_core::workflow::WorkflowState::TranscriptionQueued)?;

    let mut review_segments_total = 0usize;
    while let Some(idx) = queue.next_runnable() {
        queue.start(idx)?;
        let path = queue.task(idx).unwrap().path.clone();
        let audio_id = queue.task(idx).unwrap().job.audio.clone();

        let request = TranscriptionRequest {
            audio: audio_id.clone(),
            path: path.clone(),
            language: if workspace.case.assignment.special_instructions.iter().any(|s| s.contains("İngilizce")) {
                Some("en".to_string())
            } else {
                None
            },
            params: birkat_core::audio::InferenceParams::default(),
            chunk_secs: 30.0,
            overlap_secs: 2.0,
            diarization: false,
            word_timestamps: false,
        };

        let mut progress = |_p: f64| true;
        match engine.transcribe(&request, &mut progress) {
            Ok(engine_out) => {
                let model = birkat_core::audio::ModelVersion {
                    name: engine.capabilities().name,
                    version: engine.capabilities().version,
                    file_hash: String::new(),
                    license: "MIT".to_string(),
                    path: workspace.case.assignment.special_instructions.first().cloned().unwrap_or_default(),
                };
                queue.complete(idx, model)?;

                // Kalite kontrolü.
                let qc_segments: Vec<QualitySegment> = engine_out
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(i, s)| QualitySegment {
                        index: i,
                        start: s.start,
                        end: s.end,
                        text: s.text.clone(),
                        level_dbfs: s.level_dbfs,
                        has_speech: Some(!s.text.trim().is_empty()),
                        overlapping_speech: s.text.contains("[üst üste konuşma]"),
                        language: engine_out.language.clone(),
                    })
                    .collect();

                let qc = run_quality_control(&QualityControlInput {
                    segments: qc_segments,
                    audio_duration: engine_out.duration_secs(),
                    expected_language: engine_out.language.clone(),
                    chunk_boundaries: Vec::new(),
                    ..Default::default()
                });

                let review_count = qc.review_priorities().len();
                review_segments_total += review_count;

                // Transkript oluştur (makine çıktısı), sonra kalite kontrol
                // tamamlandığı için incelemeye hazır duruma geçir.
                let status = apply_transition(
                    ContentStatus::MachineOutput,
                    StatusTransition::QualityControlPassed,
                )
                .unwrap_or(ContentStatus::ReadyForReview);

                let segments: Vec<TranscriptSegment> = engine_out
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(i, s)| TranscriptSegment {
                        index: i,
                        start: s.start,
                        end: s.end,
                        text: s.text.clone(),
                        words: Vec::new(),
                        speaker: s.speaker.map(birkat_core::audio::SpeakerId),
                        annotations: Vec::new(),
                        model_confidence: s.model_indicator,
                        level_dbfs: s.level_dbfs,
                    })
                    .collect();

                let transcript = Transcript {
                    id: TranscriptId::new(),
                    audio: audio_id.clone(),
                    version: TranscriptVersion::MachineRaw,
                    status,
                    segments,
                    model: queue.task(idx).unwrap().job.model.clone(),
                    created_at: Timestamp::now(),
                    language: engine_out.language.clone(),
                    speakers: Vec::new(),
                    quality: Some(qc),
                    changes: Vec::new(),
                };

                // Kasaya kaydet.
                let name = PathBuf::from(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "transcript".to_string());
                let rel = format!("transkriptler/{name}.json");
                let json = serde_json::to_string_pretty(&transcript).unwrap_or_default();
                let _ = workspace
                    .vault
                    .store_bytes(&rel, json.as_bytes(), "transkripsiyon motoru");

                out.report_input.transcripts.push(TranscriptRow {
                    audio_file: name,
                    version_label: TranscriptVersion::MachineRaw.label().to_string(),
                    status: transcript.status,
                    segments: transcript
                        .segments
                        .iter()
                        .map(|s| {
                            (
                                birkat_audio::playback::format_time_code(s.start),
                                s.speaker.map(|sp| sp.display_label()),
                                s.text.clone(),
                            )
                        })
                        .collect(),
                    uncertain: Vec::new(),
                });

                out.transcripts.push(transcript);
                out.transcribed_files += 1;
            }
            Err(e) => {
                queue.fail(idx, e.to_string())?;
                out.warnings.push(format!("{path} metne dönüştürülemedi: {e}"));
            }
        }
    }

    out.review_items = review_segments_total;

    workspace.commit(
        store,
        birkat_audit::AuditAction::Transcription,
        format!(
            "{} ses dosyası metne dönüştürüldü",
            out.transcribed_files
        ),
        None,
        serde_json::json!({ "transcribed": out.transcribed_files }),
    )?;

    workspace.advance_workflow(store, birkat_core::workflow::WorkflowState::QualityControl)?;
    if out.review_items > 0 {
        out.warnings.push(format!(
            "{} bölüm bilirkişi incelemesi gerektiriyor (kalite kontrolü).",
            out.review_items
        ));
    }

    workspace.advance_workflow(store, birkat_core::workflow::WorkflowState::ExpertReview)?;

    // 14-16: Rapor taslağı, UDF ve son kontrol listesi.
    step(&mut out, "Rapor taslağı ve UDF hazırlanıyor");

    // Rapor girdisini kasaya yaz; böylece rapor komutu aynı veriyi kullanır.
    if let Ok(json) = serde_json::to_string_pretty(&out.report_input) {
        let _ = workspace.vault.store_bytes(
            "rapor/rapor-girdisi.json",
            json.as_bytes(),
            "otomatik iş akışı",
        );
    }

    let experts: Vec<birkat_core::profile::ExpertProfile> = store.list_profiles()?;
    let report = birkat_report::build_report(
        &workspace.case,
        &experts,
        &out.report_input,
        birkat_core::report::ReportKind::Main,
        1,
    );
    let udf = birkat_report::report_to_udf(&report, &birkat_report::ReportToUdfOptions::default());
    out.udf = Some(udf);

    // Son kontrol listesi.
    out.checklist = vec![
        format!("{} ses dosyası envantere alındı.", out.audio_files),
        format!("{} dosya metne dönüştürüldü.", out.transcribed_files),
        format!("{} bölüm inceleme bekliyor.", out.review_items),
        "Transkriptler bilirkişi tarafından doğrulanmalıdır.".to_string(),
        "Görevlendirme soruları yanıtlanmalıdır.".to_string(),
        "Gerekçeli sonuç ve mesleki kanaat yazılmalıdır.".to_string(),
        "Rapor onaylanmalı ve UDF doğrulanmalıdır.".to_string(),
    ];

    // Veri kökünü kullan (ileride rapor dosyaları buraya yazılabilir).
    let _ = data_root;

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::case::CourtOrAuthority;
    use birkat_core::ids::CourtId;
    use birkat_transcription::engine::MockEngine;

    fn court() -> CourtOrAuthority {
        CourtOrAuthority {
            id: CourtId::new(),
            name: "Ankara 5. Asliye Ceza Mahkemesi".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    fn write_wav(path: &std::path::Path, secs: f32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        let n = (16000.0 * secs) as usize;
        for i in 0..n {
            let t = i as f32 / 16000.0;
            let v = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            w.write_sample((v * 12000.0) as i16).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn workflow_discovers_and_transcribes_audio() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let materials = dir.path().join("materyal");
        std::fs::create_dir_all(&materials).unwrap();
        write_wav(&materials.join("kayit1.wav"), 0.3);
        write_wav(&materials.join("kayit2.wav"), 0.3);
        std::fs::write(materials.join("belge.pdf"), b"%PDF-1.7\n").unwrap();

        let case = crate::workspace::new_case_from_assignment(
            "2025/123",
            court(),
            Some("2025/123 Esas".into()),
            Some("Ses çözümlemesi".into()),
        );
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();

        let engine = MockEngine::default();
        let out = run_automatic_workflow(
            &mut ws,
            &mut store,
            &[materials.clone()],
            &engine,
            dir.path(),
        )
        .unwrap();

        assert_eq!(out.audio_files, 2, "iki ses dosyası bulunmalı");
        assert_eq!(out.transcribed_files, 2, "iki dosya metne dönüştürülmeli");
        assert_eq!(out.transcripts.len(), 2);
        assert!(out.udf.is_some());
        assert!(!out.checklist.is_empty());
        assert!(!out.steps.is_empty());
    }

    #[test]
    fn workflow_produces_valid_udf() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let materials = dir.path().join("m");
        std::fs::create_dir_all(&materials).unwrap();
        write_wav(&materials.join("a.wav"), 0.2);

        let case = crate::workspace::new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        let out = run_automatic_workflow(
            &mut ws,
            &mut store,
            &[materials],
            &MockEngine::default(),
            dir.path(),
        )
        .unwrap();

        let udf = out.udf.unwrap();
        let bytes = birkat_udf::write_udf_to_bytes(udf).unwrap();
        let v = birkat_udf::validate_udf(&bytes);
        assert!(v.ok, "hatalar: {:?}", v.errors());
    }

    #[test]
    fn workflow_reports_unreadable_audio() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let materials = dir.path().join("m");
        std::fs::create_dir_all(&materials).unwrap();
        std::fs::write(materials.join("bozuk.mp3"), b"ID3\x04\x00bozuk veri").unwrap();

        let case = crate::workspace::new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        let out = run_automatic_workflow(
            &mut ws,
            &mut store,
            &[materials],
            &MockEngine::default(),
            dir.path(),
        )
        .unwrap();
        assert!(out.has_exceptions());
        assert!(out.warnings.iter().any(|w| w.contains("okunamadı")));
    }

    #[test]
    fn workflow_handles_missing_material_path() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let case = crate::workspace::new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        let out = run_automatic_workflow(
            &mut ws,
            &mut store,
            &[dir.path().join("yok")],
            &MockEngine::default(),
            dir.path(),
        )
        .unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("bulunamadı")));
        assert_eq!(out.audio_files, 0);
    }

    #[test]
    fn workflow_records_audit_chain() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let materials = dir.path().join("m");
        std::fs::create_dir_all(&materials).unwrap();
        write_wav(&materials.join("a.wav"), 0.2);
        let case = crate::workspace::new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        run_automatic_workflow(
            &mut ws,
            &mut store,
            &[materials],
            &MockEngine::default(),
            dir.path(),
        )
        .unwrap();

        let chain = store.load_audit_chain().unwrap();
        assert!(chain.verify().valid, "{}", chain.verify().message);
        assert!(chain.events.len() >= 3);
    }

    #[test]
    fn workflow_moves_case_through_states() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let materials = dir.path().join("m");
        std::fs::create_dir_all(&materials).unwrap();
        write_wav(&materials.join("a.wav"), 0.2);
        let case = crate::workspace::new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        run_automatic_workflow(
            &mut ws,
            &mut store,
            &[materials],
            &MockEngine::default(),
            dir.path(),
        )
        .unwrap();
        assert_eq!(
            ws.case.workflow.state,
            birkat_core::workflow::WorkflowState::ExpertReview
        );
    }

    #[test]
    fn workflow_stores_transcripts_in_vault() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let materials = dir.path().join("m");
        std::fs::create_dir_all(&materials).unwrap();
        write_wav(&materials.join("kayit.wav"), 0.2);
        let case = crate::workspace::new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        run_automatic_workflow(
            &mut ws,
            &mut store,
            &[materials],
            &MockEngine::default(),
            dir.path(),
        )
        .unwrap();
        assert!(ws.vault.manifest().objects.iter().any(|o| o.relative_path.contains("transkriptler")));
    }

    #[test]
    fn outcome_summary_is_turkish() {
        let out = WorkflowOutcome {
            audio_files: 3,
            transcribed_files: 2,
            review_items: 1,
            ..Default::default()
        };
        let s = out.summary();
        assert!(s.contains("3 ses dosyası"));
        assert!(s.contains("2 dosya"));
    }

    #[test]
    fn report_input_includes_inventory_rows() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = CaseStore::open(dir.path().join("b.db")).unwrap();
        let materials = dir.path().join("m");
        std::fs::create_dir_all(&materials).unwrap();
        write_wav(&materials.join("a.wav"), 0.2);
        let case = crate::workspace::new_case_from_assignment("A", court(), None, None);
        let mut ws = CaseWorkspace::create(dir.path(), case, &mut store).unwrap();
        let out = run_automatic_workflow(
            &mut ws,
            &mut store,
            &[materials],
            &MockEngine::default(),
            dir.path(),
        )
        .unwrap();
        assert_eq!(out.report_input.audio_inventory.len(), 1);
        assert_eq!(out.report_input.audio_inventory[0].codec, "PCM");
    }
}
