//! Otomatik transkripsiyon kuyruğu (9.4).
//!
//! Kuyruk özellikleri:
//!
//! * Dosya bazlı görev.
//! * İşlem önceliği.
//! * Duraklatma, devam, iptal.
//! * Hata sonrası yeniden deneme.
//! * Kesinti sonrası devam (kontrol noktası).
//! * İlerleme yüzdesi.
//! * Tahmini kalan süre; **yalnızca ölçümlere dayalıysa**.
//!
//! Aynı işlem tekrarlandığında mükerrer sonuçlar oluşturulmaz: tamamlanmış bir
//! ses için yeniden görev açılmaz.

use birkat_core::audio::{JobStatus, ModelVersion, TranscriptionJob};
use birkat_core::error::{CoreError, Result};
use birkat_core::ids::{AudioId, JobId};
use birkat_core::time_util::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kuyruk görevi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTask {
    /// İş kaydı.
    pub job: TranscriptionJob,
    /// Kaynak dosya yolu.
    pub path: String,
    /// Ölçülen işlem hızı (ses saniyesi / gerçek saniye); ETA için kullanılır.
    pub measured_speed: Option<f64>,
    /// Ölçülen işlem süresi (saniye).
    pub measured_elapsed: Option<f64>,
    /// Ses süresi (saniye).
    pub audio_duration: Option<f64>,
}

impl QueueTask {
    /// Yeni görev.
    pub fn new(audio: AudioId, path: impl Into<String>, priority: u32) -> Self {
        let mut job = TranscriptionJob {
            id: JobId::new(),
            audio,
            priority,
            status: JobStatus::Queued,
            progress: 0.0,
            chunks_done: 0,
            chunks_total: None,
            eta_secs: None,
            attempts: 0,
            max_attempts: 3,
            started_at: None,
            finished_at: None,
            model: None,
            params: Default::default(),
            error: None,
            checkpoint: None,
        };
        job.params = birkat_core::audio::InferenceParams::default();
        Self {
            job,
            path: path.into(),
            measured_speed: None,
            measured_elapsed: None,
            audio_duration: None,
        }
    }

    /// Tahmini kalan süre (saniye).
    ///
    /// **Yalnızca ölçümlere dayalıysa** döndürülür; aksi hâlde `None`.
    /// Böylece uydurma bir tahmin sunulmaz.
    pub fn eta_secs(&self) -> Option<u64> {
        let speed = self.measured_speed?;
        let total = self.audio_duration?;
        if speed <= 0.0 {
            return None;
        }
        let remaining_audio = total * (1.0 - self.job.progress);
        let remaining = remaining_audio / speed;
        Some(remaining.max(0.0).round() as u64)
    }

    /// İlerleme yüzdesi (0 - 100).
    pub fn progress_percent(&self) -> f64 {
        self.job.progress * 100.0
    }

    /// Yeniden denenebilir mi?
    pub fn can_retry(&self) -> bool {
        self.job.status == JobStatus::Failed && self.job.attempts < self.job.max_attempts
    }

    /// Tamamlandı mı?
    pub fn is_completed(&self) -> bool {
        self.job.status == JobStatus::Completed
    }
}

/// Kuyruk anlık görüntüsü.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    /// Kuyruktaki görev sayısı.
    pub queued: usize,
    /// Çalışan görev sayısı.
    pub running: usize,
    /// Duraklatılmış.
    pub paused: usize,
    /// Tamamlanmış.
    pub completed: usize,
    /// Hatalı.
    pub failed: usize,
    /// İptal edilmiş.
    pub cancelled: usize,
    /// Genel ilerleme (0.0 - 1.0).
    pub overall_progress: f64,
}

/// Transkripsiyon iş kuyruğu.
#[derive(Debug, Clone, Default)]
pub struct JobQueue {
    tasks: Vec<QueueTask>,
    /// Aynı ses için tamamlanmış görev indeksi (mükerrer işlemi önler).
    completed_index: HashMap<AudioId, usize>,
    /// Kuyruk duraklatıldı mı?
    pub paused: bool,
}

impl JobQueue {
    /// Yeni kuyruk.
    pub fn new() -> Self {
        Self::default()
    }

    /// Kuyruğa görev ekler.
    ///
    /// Aynı ses için zaten tamamlanmış bir görev varsa yeni görev
    /// oluşturulmaz; var olan görev indeksi döndürülür.
    pub fn enqueue(&mut self, task: QueueTask) -> Result<usize> {
        if let Some(&idx) = self.completed_index.get(&task.job.audio) {
            return Ok(idx);
        }
        self.tasks.push(task);
        Ok(self.tasks.len() - 1)
    }

    /// Tüm görevler.
    pub fn tasks(&self) -> &[QueueTask] {
        &self.tasks
    }

    /// Değiştirilebilir görev.
    pub fn task_mut(&mut self, index: usize) -> Option<&mut QueueTask> {
        self.tasks.get_mut(index)
    }

    /// Görev.
    pub fn task(&self, index: usize) -> Option<&QueueTask> {
        self.tasks.get(index)
    }

    /// Sıradaki çalıştırılacak görevi seçer (öncelik küçük olan önce).
    ///
    /// Duraklatılmışsa `None` döner.
    pub fn next_runnable(&self) -> Option<usize> {
        if self.paused {
            return None;
        }
        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.job.status == JobStatus::Queued)
            .min_by_key(|(_, t)| t.job.priority)
            .map(|(i, _)| i)
    }

    /// Görevi çalıştırmaya başlar.
    pub fn start(&mut self, index: usize) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        if task.job.status != JobStatus::Queued && task.job.status != JobStatus::Paused {
            return Err(CoreError::Validation(format!(
                "Görev {:?} durumunda başlatılamaz.",
                task.job.status
            )));
        }
        task.job.status = JobStatus::Running;
        task.job.started_at = Some(Timestamp::now());
        task.job.attempts += 1;
        Ok(())
    }

    /// İlerleme günceller.
    pub fn update_progress(&mut self, index: usize, progress: f64, elapsed_secs: f64) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        task.job.progress = progress.clamp(0.0, 1.0);
        // Ölçüme dayalı hız ve ETA güncelle.
        if let Some(total) = task.audio_duration {
            if elapsed_secs > 0.0 && progress > 0.0 {
                let processed_audio = total * progress;
                let speed = processed_audio / elapsed_secs;
                task.measured_speed = Some(speed);
                task.measured_elapsed = Some(elapsed_secs);
                task.job.eta_secs = task.eta_secs();
            }
        }
        Ok(())
    }

    /// Görevi tamamlar.
    pub fn complete(&mut self, index: usize, model: ModelVersion) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        task.job.status = JobStatus::Completed;
        task.job.progress = 1.0;
        task.job.finished_at = Some(Timestamp::now());
        task.job.model = Some(model);
        task.job.eta_secs = Some(0);
        let audio = task.job.audio.clone();
        self.completed_index.insert(audio, index);
        Ok(())
    }

    /// Görevi hata ile bitirir.
    pub fn fail(&mut self, index: usize, error: impl Into<String>) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        task.job.status = JobStatus::Failed;
        task.job.error = Some(error.into());
        task.job.finished_at = Some(Timestamp::now());
        Ok(())
    }

    /// Görevi yeniden denemek üzere kuyruğa alır.
    pub fn retry(&mut self, index: usize) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        if task.job.attempts >= task.job.max_attempts {
            return Err(CoreError::Validation(
                "Azami deneme sayısına ulaşıldı.".to_string(),
            ));
        }
        task.job.status = JobStatus::Queued;
        task.job.error = None;
        Ok(())
    }

    /// Görevi duraklatır.
    pub fn pause_task(&mut self, index: usize) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        if task.job.status == JobStatus::Running {
            task.job.status = JobStatus::Paused;
        }
        Ok(())
    }

    /// Görevi devam ettirir.
    pub fn resume_task(&mut self, index: usize) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        if task.job.status == JobStatus::Paused {
            task.job.status = JobStatus::Queued;
        }
        Ok(())
    }

    /// Görevi iptal eder.
    pub fn cancel(&mut self, index: usize) -> Result<()> {
        let task = self
            .tasks
            .get_mut(index)
            .ok_or_else(|| CoreError::NotFound {
                kind: "kuyruk görevi",
                id: index.to_string(),
            })?;
        task.job.status = JobStatus::Cancelled;
        Ok(())
    }

    /// Tüm kuyruğu duraklatır.
    pub fn pause_all(&mut self) {
        self.paused = true;
    }

    /// Kuyruğu devam ettirir.
    pub fn resume_all(&mut self) {
        self.paused = false;
    }

    /// Anlık görüntü.
    pub fn snapshot(&self) -> QueueSnapshot {
        let count = |s: JobStatus| self.tasks.iter().filter(|t| t.job.status == s).count();
        let overall = if self.tasks.is_empty() {
            0.0
        } else {
            self.tasks.iter().map(|t| t.job.progress).sum::<f64>() / self.tasks.len() as f64
        };
        QueueSnapshot {
            queued: count(JobStatus::Queued),
            running: count(JobStatus::Running),
            paused: count(JobStatus::Paused),
            completed: count(JobStatus::Completed),
            failed: count(JobStatus::Failed),
            cancelled: count(JobStatus::Cancelled),
            overall_progress: overall,
        }
    }

    /// Kesinti sonrası devam için kuyruk durumunu serileştirir.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.tasks).unwrap_or_default()
    }

    /// Kesinti sonrası kuyruğu geri yükler.
    pub fn from_json(json: &str) -> Result<Self> {
        let tasks: Vec<QueueTask> = serde_json::from_str(json)
            .map_err(|e| CoreError::Data(format!("Kuyruk geri yüklenemedi: {e}")))?;
        let mut q = Self {
            tasks,
            completed_index: HashMap::new(),
            paused: false,
        };
        // Çalışırken kesilen görevler kuyruğa geri alınır.
        for (i, t) in q.tasks.iter_mut().enumerate() {
            if t.job.status == JobStatus::Running {
                t.job.status = JobStatus::Queued;
            }
            if t.job.status == JobStatus::Completed {
                let audio = t.job.audio.clone();
                q.completed_index.insert(audio, i);
            }
        }
        Ok(q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> ModelVersion {
        ModelVersion {
            name: "whisper-large-v3-turbo".into(),
            version: "1.0".into(),
            file_hash: "aa".repeat(32),
            license: "MIT".into(),
            path: "models/whisper".into(),
        }
    }

    fn queue_with(n: usize) -> JobQueue {
        let mut q = JobQueue::new();
        for i in 0..n {
            q.enqueue(QueueTask::new(AudioId::new(), format!("/a/{i}.wav"), i as u32))
                .unwrap();
        }
        q
    }

    #[test]
    fn next_runnable_respects_priority() {
        let mut q = JobQueue::new();
        q.enqueue(QueueTask::new(AudioId::new(), "a", 5)).unwrap();
        q.enqueue(QueueTask::new(AudioId::new(), "b", 1)).unwrap();
        q.enqueue(QueueTask::new(AudioId::new(), "c", 3)).unwrap();
        assert_eq!(q.next_runnable(), Some(1));
    }

    #[test]
    fn paused_queue_has_no_runnable() {
        let mut q = queue_with(2);
        q.pause_all();
        assert_eq!(q.next_runnable(), None);
        q.resume_all();
        assert!(q.next_runnable().is_some());
    }

    #[test]
    fn completed_task_is_not_duplicated() {
        let mut q = JobQueue::new();
        let audio = AudioId::new();
        let idx = q
            .enqueue(QueueTask::new(audio.clone(), "a", 1))
            .unwrap();
        q.start(idx).unwrap();
        q.complete(idx, model()).unwrap();
        let idx2 = q.enqueue(QueueTask::new(audio, "a", 1)).unwrap();
        assert_eq!(idx, idx2, "mükerrer görev oluşturulmamalı");
        assert_eq!(q.tasks().len(), 1);
    }

    #[test]
    fn retry_is_limited_by_max_attempts() {
        let mut q = JobQueue::new();
        let idx = q.enqueue(QueueTask::new(AudioId::new(), "a", 1)).unwrap();
        q.task_mut(idx).unwrap().job.max_attempts = 2;
        q.start(idx).unwrap();
        q.fail(idx, "hata").unwrap();
        assert!(q.task(idx).unwrap().can_retry());
        q.retry(idx).unwrap();
        q.start(idx).unwrap();
        q.fail(idx, "hata").unwrap();
        assert!(!q.task(idx).unwrap().can_retry());
        assert!(q.retry(idx).is_err());
    }

    #[test]
    fn eta_requires_measurement() {
        let mut t = QueueTask::new(AudioId::new(), "a", 1);
        assert!(t.eta_secs().is_none(), "ölçüm olmadan ETA verilmez");
        t.audio_duration = Some(100.0);
        t.job.progress = 0.5;
        t.measured_speed = Some(2.0); // 2x gerçek zaman
        let eta = t.eta_secs().unwrap();
        assert_eq!(eta, 25); // 50 sn ses / 2x = 25 sn
    }

    #[test]
    fn update_progress_measures_speed() {
        let mut q = JobQueue::new();
        let idx = q.enqueue(QueueTask::new(AudioId::new(), "a", 1)).unwrap();
        q.task_mut(idx).unwrap().audio_duration = Some(100.0);
        q.start(idx).unwrap();
        q.update_progress(idx, 0.5, 25.0).unwrap();
        let t = q.task(idx).unwrap();
        assert!(t.measured_speed.is_some());
        assert_eq!(t.job.eta_secs, Some(25));
    }

    #[test]
    fn pause_and_resume_task() {
        let mut q = JobQueue::new();
        let idx = q.enqueue(QueueTask::new(AudioId::new(), "a", 1)).unwrap();
        q.start(idx).unwrap();
        q.pause_task(idx).unwrap();
        assert_eq!(q.task(idx).unwrap().job.status, JobStatus::Paused);
        q.resume_task(idx).unwrap();
        assert_eq!(q.task(idx).unwrap().job.status, JobStatus::Queued);
    }

    #[test]
    fn cancel_marks_task() {
        let mut q = JobQueue::new();
        let idx = q.enqueue(QueueTask::new(AudioId::new(), "a", 1)).unwrap();
        q.cancel(idx).unwrap();
        assert_eq!(q.task(idx).unwrap().job.status, JobStatus::Cancelled);
    }

    #[test]
    fn snapshot_counts_states() {
        let mut q = queue_with(3);
        q.start(0).unwrap();
        q.complete(0, model()).unwrap();
        q.start(1).unwrap();
        let s = q.snapshot();
        assert_eq!(s.completed, 1);
        assert_eq!(s.running, 1);
        assert_eq!(s.queued, 1);
    }

    #[test]
    fn crash_recovery_requeues_running_tasks() {
        let mut q = queue_with(2);
        q.start(0).unwrap();
        let json = q.to_json();
        let restored = JobQueue::from_json(&json).unwrap();
        assert_eq!(
            restored.task(0).unwrap().job.status,
            JobStatus::Queued,
            "çalışırken kesilen görev kuyruğa dönmeli"
        );
    }

    #[test]
    fn crash_recovery_preserves_completed_index() {
        let mut q = JobQueue::new();
        let audio = AudioId::new();
        let idx = q
            .enqueue(QueueTask::new(audio.clone(), "a", 1))
            .unwrap();
        q.start(idx).unwrap();
        q.complete(idx, model()).unwrap();
        let restored = JobQueue::from_json(&q.to_json()).unwrap();
        // Aynı ses yeniden kuyruğa alınmamalı.
        let mut r = restored;
        let before = r.tasks().len();
        r.enqueue(QueueTask::new(audio, "a", 1)).unwrap();
        assert_eq!(r.tasks().len(), before);
    }

    #[test]
    fn cannot_start_completed_task() {
        let mut q = JobQueue::new();
        let idx = q.enqueue(QueueTask::new(AudioId::new(), "a", 1)).unwrap();
        q.start(idx).unwrap();
        q.complete(idx, model()).unwrap();
        assert!(q.start(idx).is_err());
    }
}
