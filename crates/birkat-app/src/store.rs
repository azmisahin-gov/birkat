//! Vaka deposu (SQLite).
//!
//! Vaka verisi, işlem geçmişi ve diğer kayıtlar yerel, dayanıklı bir SQLite
//! veritabanında tutulur. Büyük dosyalar (ses, imaj, rapor) kasadaki şifreli
//! blob'larda saklanır; veritabanında yalnızca metaveri ve hash tutulur.
//!
//! ## Dayanıklılık
//!
//! WAL günlükleme ve `synchronous = FULL` ile elektrik kesintisi/çökme
//! senaryolarına karşı dayanıklılık sağlanır.

use birkat_core::case::Case;
use birkat_core::error::{CoreError, Result};
use birkat_core::ids::CaseId;
use birkat_core::profile::ExpertProfile;
use birkat_core::time_util::Timestamp;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// Veritabanı şema sürümü.
pub const SCHEMA_VERSION: u32 = 1;

/// Vaka deposu.
pub struct CaseStore {
    conn: Connection,
    path: PathBuf,
}

impl CaseStore {
    /// Depoyu açar (yoksa oluşturur).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)
            .map_err(|e| CoreError::Data(format!("Veritabanı açılamadı: {e}")))?;
        let mut store = Self { conn, path };
        store.configure()?;
        store.migrate()?;
        Ok(store)
    }

    /// Bellek içi depo (testler için).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| CoreError::Data(format!("Bellek içi veritabanı açılamadı: {e}")))?;
        let mut store = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        store.configure()?;
        store.migrate()?;
        Ok(store)
    }

    /// Veritabanı yolunu döndürür.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn configure(&mut self) -> Result<()> {
        // Elektrik kesintisi/çökmeye dayanıklılık.
        self.conn
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(db_err)?;
        self.conn
            .pragma_update(None, "synchronous", "FULL")
            .map_err(db_err)?;
        self.conn
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_err)?;
        Ok(())
    }

    fn migrate(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS cases (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    state TEXT NOT NULL,
                    schema_version INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    archived INTEGER NOT NULL DEFAULT 0,
                    payload TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS profiles (
                    id TEXT PRIMARY KEY,
                    full_name TEXT NOT NULL,
                    payload TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS audit (
                    sequence INTEGER PRIMARY KEY,
                    id TEXT NOT NULL,
                    case_id TEXT,
                    action TEXT NOT NULL,
                    actor TEXT NOT NULL,
                    description TEXT NOT NULL,
                    at TEXT NOT NULL,
                    previous_hash TEXT NOT NULL,
                    hash TEXT NOT NULL,
                    payload TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                "#,
            )
            .map_err(db_err)?;

        self.set_meta("schema_version", &SCHEMA_VERSION.to_string())?;
        Ok(())
    }

    fn set_meta(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO meta(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Meta değeri okur.
    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(db_err)
    }

    /// Vakayı kaydeder (yeni veya güncelleme).
    pub fn save_case(&mut self, case: &Case) -> Result<()> {
        let payload = serde_json::to_string(case)
            .map_err(|e| CoreError::Data(format!("Vaka serileştirilemedi: {e}")))?;
        let state = format!("{:?}", case.workflow.state);
        self.conn
            .execute(
                "INSERT INTO cases(id, title, state, schema_version, created_at, updated_at, archived, payload)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   state = excluded.state,
                   schema_version = excluded.schema_version,
                   updated_at = excluded.updated_at,
                   archived = excluded.archived,
                   payload = excluded.payload",
                params![
                    case.id.as_str(),
                    case.title,
                    state,
                    case.schema_version as i64,
                    case.created_at.to_rfc3339(),
                    case.updated_at.to_rfc3339(),
                    case.archived as i64,
                    payload,
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Vakayı yükler.
    pub fn load_case(&self, id: &CaseId) -> Result<Case> {
        let payload: Option<String> = self
            .conn
            .query_row(
                "SELECT payload FROM cases WHERE id = ?1",
                params![id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)?;
        let payload = payload.ok_or_else(|| CoreError::NotFound {
            kind: "vaka",
            id: id.to_string(),
        })?;
        serde_json::from_str(&payload)
            .map_err(|e| CoreError::Data(format!("Vaka çözülemedi: {e}")))
    }

    /// Tüm vakaları yükler.
    pub fn list_cases(&self) -> Result<Vec<Case>> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload FROM cases ORDER BY updated_at DESC")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?;
        let mut out = Vec::new();
        for r in rows {
            let payload = r.map_err(db_err)?;
            if let Ok(c) = serde_json::from_str::<Case>(&payload) {
                out.push(c);
            }
        }
        Ok(out)
    }

    /// Vakayı siler.
    pub fn delete_case(&mut self, id: &CaseId) -> Result<()> {
        self.conn
            .execute("DELETE FROM cases WHERE id = ?1", params![id.as_str()])
            .map_err(db_err)?;
        Ok(())
    }

    /// Vaka sayısı.
    pub fn case_count(&self) -> Result<u32> {
        self.conn
            .query_row("SELECT COUNT(*) FROM cases", [], |r| r.get::<_, i64>(0))
            .map(|n| n as u32)
            .map_err(db_err)
    }

    /// Bilirkişi profilini kaydeder.
    pub fn save_profile(&mut self, profile: &ExpertProfile) -> Result<()> {
        let payload = serde_json::to_string(profile)
            .map_err(|e| CoreError::Data(format!("Profil serileştirilemedi: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO profiles(id, full_name, payload) VALUES(?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET full_name = excluded.full_name, payload = excluded.payload",
                params![profile.id.as_str(), profile.full_name, payload],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Profilleri listeler.
    pub fn list_profiles(&self) -> Result<Vec<ExpertProfile>> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload FROM profiles ORDER BY full_name")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?;
        let mut out = Vec::new();
        for r in rows {
            let payload = r.map_err(db_err)?;
            if let Ok(p) = serde_json::from_str::<ExpertProfile>(&payload) {
                out.push(p);
            }
        }
        Ok(out)
    }

    /// Profili siler.
    pub fn delete_profile(&mut self, id: &birkat_core::ids::ExpertProfileId) -> Result<()> {
        self.conn
            .execute("DELETE FROM profiles WHERE id = ?1", params![id.as_str()])
            .map_err(db_err)?;
        Ok(())
    }

    /// Denetim olayını kaydeder.
    pub fn append_audit(&mut self, event: &birkat_audit::AuditEvent) -> Result<()> {
        let payload = serde_json::to_string(event)
            .map_err(|e| CoreError::Data(format!("Olay serileştirilemedi: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO audit(sequence, id, case_id, action, actor, description, at, previous_hash, hash, payload)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.sequence as i64,
                    event.id.as_str(),
                    event.case.as_ref().map(|c| c.as_str()),
                    format!("{:?}", event.action),
                    event.actor,
                    event.description,
                    event.at.to_rfc3339(),
                    event.previous_hash,
                    event.hash,
                    payload,
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Tüm denetim olaylarını yükler.
    pub fn load_audit_chain(&self) -> Result<birkat_audit::AuditChain> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload FROM audit ORDER BY sequence ASC")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?;
        let mut chain = birkat_audit::AuditChain::new();
        for r in rows {
            let payload = r.map_err(db_err)?;
            if let Ok(e) = serde_json::from_str::<birkat_audit::AuditEvent>(&payload) {
                chain.events.push(e);
            }
        }
        Ok(chain)
    }

    /// Ayar değeri kaydeder.
    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO settings(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Ayar değeri okur.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(db_err)
    }

    /// Veritabanı bütünlüğünü denetler.
    pub fn integrity_check(&self) -> Result<bool> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(db_err)?;
        Ok(result == "ok")
    }

    /// Yedekleme için veritabanını bir dosyaya kopyalar.
    pub fn backup_to(&self, target: impl AsRef<Path>) -> Result<()> {
        let target = target.as_ref();
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let sql = format!("VACUUM INTO '{}'", target.display());
        self.conn.execute_batch(&sql).map_err(db_err)?;
        Ok(())
    }

    /// Şema sürümü.
    pub fn schema_version(&self) -> Result<u32> {
        Ok(self
            .get_meta("schema_version")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(SCHEMA_VERSION))
    }

    /// Son güncelleme zamanı en yeni olan vakalar (panel için).
    pub fn recent_cases(&self, limit: usize) -> Result<Vec<Case>> {
        let mut all = self.list_cases()?;
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        all.truncate(limit);
        Ok(all)
    }

    /// Vakaları iş akışı durumuna göre sayar.
    pub fn count_by_state(&self) -> Result<Vec<(String, u32)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT state, COUNT(*) FROM cases GROUP BY state ORDER BY COUNT(*) DESC")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u32)))
            .map_err(db_err)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(db_err)?);
        }
        Ok(out)
    }

    /// Vakanın güncelleme zamanını şimdiye ayarlar.
    pub fn touch_case(&mut self, case: &mut Case) -> Result<()> {
        case.updated_at = Timestamp::now();
        self.save_case(case)
    }
}

fn db_err(e: rusqlite::Error) -> CoreError {
    CoreError::Data(format!("Veritabanı hatası: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use birkat_core::case::CourtOrAuthority;
    use birkat_core::ids::CourtId;

    fn court() -> CourtOrAuthority {
        CourtOrAuthority {
            id: CourtId::new(),
            name: "Ankara 5. Asliye Ceza Mahkemesi".into(),
            unit_kind: "Mahkeme".into(),
            contact: None,
            source_ref: None,
        }
    }

    #[test]
    fn open_and_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let s = CaseStore::open(dir.path().join("birkat.db")).unwrap();
        assert_eq!(s.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(s.integrity_check().unwrap());
    }

    #[test]
    fn save_and_load_case() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let c = Case::new("2025/123", court());
        s.save_case(&c).unwrap();
        let loaded = s.load_case(&c.id).unwrap();
        assert_eq!(loaded.title, "2025/123");
        assert_eq!(loaded.id, c.id);
    }

    #[test]
    fn update_case_persists_changes() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut c = Case::new("Test", court());
        s.save_case(&c).unwrap();
        c.assignment.file_number = Some("2025/123 Esas".into());
        c.workflow.advance().unwrap();
        s.save_case(&c).unwrap();
        let loaded = s.load_case(&c.id).unwrap();
        assert_eq!(loaded.assignment.file_number.as_deref(), Some("2025/123 Esas"));
        assert_eq!(
            loaded.workflow.state,
            birkat_core::workflow::WorkflowState::AssignmentImported
        );
    }

    #[test]
    fn list_and_delete_cases() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let a = Case::new("A", court());
        let b = Case::new("B", court());
        s.save_case(&a).unwrap();
        s.save_case(&b).unwrap();
        assert_eq!(s.case_count().unwrap(), 2);
        assert_eq!(s.list_cases().unwrap().len(), 2);
        s.delete_case(&a.id).unwrap();
        assert_eq!(s.case_count().unwrap(), 1);
    }

    #[test]
    fn missing_case_reports_not_found() {
        let s = CaseStore::open_in_memory().unwrap();
        let err = s.load_case(&CaseId::new()).unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn save_and_list_profiles() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let p = ExpertProfile::new("Ayşe Yılmaz");
        s.save_profile(&p).unwrap();
        let list = s.list_profiles().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].full_name, "Ayşe Yılmaz");
        s.delete_profile(&p.id).unwrap();
        assert!(s.list_profiles().unwrap().is_empty());
    }

    #[test]
    fn audit_chain_persists_and_verifies() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut chain = birkat_audit::AuditChain::new();
        chain.append(
            birkat_audit::AuditAction::CaseCreated,
            "bilirkişi",
            "vaka oluşturuldu",
            None,
            None,
            serde_json::json!({}),
        );
        chain.append(
            birkat_audit::AuditAction::HashComputed,
            "bilirkişi",
            "hash hesaplandı",
            None,
            None,
            serde_json::json!({}),
        );
        for e in &chain.events {
            s.append_audit(e).unwrap();
        }
        let loaded = s.load_audit_chain().unwrap();
        assert_eq!(loaded.events.len(), 2);
        assert!(loaded.verify().valid);
    }

    #[test]
    fn settings_round_trip() {
        let mut s = CaseStore::open_in_memory().unwrap();
        s.set_setting("tema", "koyu").unwrap();
        assert_eq!(s.get_setting("tema").unwrap().as_deref(), Some("koyu"));
        assert!(s.get_setting("yok").unwrap().is_none());
    }

    #[test]
    fn count_by_state() {
        let mut s = CaseStore::open_in_memory().unwrap();
        s.save_case(&Case::new("A", court())).unwrap();
        s.save_case(&Case::new("B", court())).unwrap();
        let counts = s.count_by_state().unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].1, 2);
    }

    #[test]
    fn backup_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = CaseStore::open(dir.path().join("b.db")).unwrap();
        s.save_case(&Case::new("A", court())).unwrap();
        let target = dir.path().join("yedek/b.db");
        s.backup_to(&target).unwrap();
        assert!(target.exists());
        // Yedek açılabilmeli.
        let s2 = CaseStore::open(&target).unwrap();
        assert_eq!(s2.case_count().unwrap(), 1);
    }

    #[test]
    fn recent_cases_sorted_by_update() {
        let mut s = CaseStore::open_in_memory().unwrap();
        let mut a = Case::new("A", court());
        a.updated_at = Timestamp::from(Timestamp::now().inner() - time::Duration::hours(1));
        let b = Case::new("B", court());
        s.save_case(&a).unwrap();
        s.save_case(&b).unwrap();
        let recent = s.recent_cases(1).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title, "B");
    }
}
