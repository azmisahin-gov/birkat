//! Uygulama durumu ve ana döngü.
//!
//! Bu modül, servis katmanını (depo, kasa, otomatik iş akışı) arayüze
//! bağlar. Tüm durum değişiklikleri gerçek servis çağrılarıyla yapılır;
//! arayüzde gösterilen bilgi daima gerçek veriden gelir.

use birkat_app::dashboard::DashboardData;
use birkat_app::settings::{AppSettings, SystemHealth};
use birkat_app::store::CaseStore;
use birkat_app::workspace::CaseWorkspace;
use birkat_core::case::Case;
use birkat_core::profile::ExpertProfile;
use birkat_core::time_util::Timestamp;
use birkat_core::workflow::WorkflowState;
use eframe::egui;
use std::path::PathBuf;

/// Arayüzde gösterilen bildirim türü.
#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    /// Bilgi.
    Info(String),
    /// Başarı.
    Success(String),
    /// Uyarı (istisna).
    Warning(String),
    /// Hata.
    Error(String),
}

/// Açık olan çalışma alanı (ekran).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Ana kontrol paneli.
    Dashboard,
    /// Bilirkişi profili.
    Profile,
    /// Görevlendirme ve dosya kabul.
    Assignment,
    /// Dosya çalışma alanı.
    CaseWorkspace,
    /// Materyal ve delil yönetimi.
    Evidence,
    /// Aygıt ve edinim yönetimi.
    Device,
    /// Ses envanteri.
    AudioInventory,
    /// Transkripsiyon işlem merkezi.
    Transcription,
    /// Ses/transkript inceleme.
    AudioReview,
    /// Sorular ve dayanaklar.
    Questions,
    /// Bulgular.
    Findings,
    /// Rapor editörü.
    ReportEditor,
    /// UDF önizleme ve kontrol.
    UdfPreview,
    /// İmza ve teslim.
    Signature,
    /// Ek raporlar ve yazışmalar.
    AuxiliaryDocuments,
    /// Takvim ve ücretler.
    Calendar,
    /// Arşiv.
    Archive,
    /// Yedekleme.
    Backup,
    /// Güncelleme.
    Update,
    /// Sistem sağlığı.
    Health,
}

impl Screen {
    /// Ekranın Türkçe adı.
    pub fn title(&self) -> &'static str {
        match self {
            Screen::Dashboard => "Ana kontrol paneli",
            Screen::Profile => "Bilirkişi profili",
            Screen::Assignment => "Görevlendirme ve dosya kabul",
            Screen::CaseWorkspace => "Dosya çalışma alanı",
            Screen::Evidence => "Materyal ve delil yönetimi",
            Screen::Device => "Aygıt ve edinim yönetimi",
            Screen::AudioInventory => "Ses envanteri",
            Screen::Transcription => "Transkripsiyon işlem merkezi",
            Screen::AudioReview => "Ses/transkript inceleme",
            Screen::Questions => "Sorular ve dayanaklar",
            Screen::Findings => "Bulgular",
            Screen::ReportEditor => "Rapor editörü",
            Screen::UdfPreview => "UDF önizleme ve kontrol",
            Screen::Signature => "İmza ve teslim",
            Screen::AuxiliaryDocuments => "Ek raporlar ve yazışmalar",
            Screen::Calendar => "Takvim ve ücretler",
            Screen::Archive => "Arşiv",
            Screen::Backup => "Yedekleme",
            Screen::Update => "Güncelleme",
            Screen::Health => "Sistem sağlığı",
        }
    }

    /// Sol menüde gösterilen sıralı liste (24. madde).
    pub fn menu() -> Vec<Screen> {
        vec![
            Screen::Dashboard,
            Screen::Profile,
            Screen::Assignment,
            Screen::CaseWorkspace,
            Screen::Evidence,
            Screen::Device,
            Screen::AudioInventory,
            Screen::Transcription,
            Screen::AudioReview,
            Screen::Questions,
            Screen::Findings,
            Screen::ReportEditor,
            Screen::UdfPreview,
            Screen::Signature,
            Screen::AuxiliaryDocuments,
            Screen::Calendar,
            Screen::Archive,
            Screen::Backup,
            Screen::Update,
            Screen::Health,
        ]
    }
}

/// Uygulama durumu.
pub struct BirkatApp {
    /// Veri kök dizini.
    pub data_root: PathBuf,
    /// Vaka deposu.
    pub store: Option<CaseStore>,
    /// Açık vaka çalışma alanı.
    pub workspace: Option<CaseWorkspace>,
    /// Açık ekran.
    pub screen: Screen,
    /// Panel verisi.
    pub dashboard: Option<DashboardData>,
    /// Sistem sağlığı.
    pub health: Option<SystemHealth>,
    /// Ayarlar.
    pub settings: AppSettings,
    /// Bildirimler.
    pub notices: Vec<Notice>,
    /// Profil formu alanları.
    /// Profil formu: ad soyad.
    pub profile_name: String,
    /// Profil formu: unvan.
    pub profile_title: String,
    /// Profil formu: sicil numarası.
    pub profile_registry: String,
    /// Vaka formu alanları.
    /// Vaka formu: başlık.
    pub case_title: String,
    /// Vaka formu: görevlendiren merci.
    pub case_court: String,
    /// Vaka formu: dosya esas numarası.
    pub case_file_number: String,
    /// Vaka formu: inceleme konusu.
    pub case_subject: String,
    /// Materyal yolu.
    pub material_path: String,
    /// Kasa parolası.
    pub vault_password: String,
    /// Arama kutusu.
    pub search: String,
    /// Son transkripsiyon özeti.
    pub last_workflow_summary: Option<String>,
    /// Son üretilen rapor yolu.
    pub last_report_dir: Option<PathBuf>,
    /// Seçili vaka kimliği.
    pub selected_case: Option<String>,
    /// Uygulama başlangıç zamanı.
    pub started_at: Timestamp,
}

impl BirkatApp {
    /// Uygulamayı veri köküyle başlatır.
    pub fn new(data_root: PathBuf) -> Self {
        let mut app = Self {
            data_root,
            store: None,
            workspace: None,
            screen: Screen::Dashboard,
            dashboard: None,
            health: None,
            settings: AppSettings::default(),
            notices: Vec::new(),
            profile_name: String::new(),
            profile_title: String::new(),
            profile_registry: String::new(),
            case_title: String::new(),
            case_court: String::new(),
            case_file_number: String::new(),
            case_subject: String::new(),
            material_path: String::new(),
            vault_password: String::new(),
            search: String::new(),
            last_workflow_summary: None,
            last_report_dir: None,
            selected_case: None,
            started_at: Timestamp::now(),
        };
        app.initialize();
        app
    }

    /// Depoyu açar ve başlangıç verisini yükler.
    pub fn initialize(&mut self) {
        if let Err(e) = std::fs::create_dir_all(&self.data_root) {
            self.notice(Notice::Error(format!("Veri dizini oluşturulamadı: {e}")));
            return;
        }
        match CaseStore::open(self.data_root.join("birkat.db")) {
            Ok(store) => {
                self.store = Some(store);
                self.notice(Notice::Info(format!(
                    "BİRKÂT {} başlatıldı.",
                    birkat_core::APP_VERSION
                )));
                self.refresh_dashboard();
                self.refresh_health();
            }
            Err(e) => self.notice(Notice::Error(format!("Veritabanı açılamadı: {e}"))),
        }
    }

    /// Bildirim ekler (en fazla 20 tutulur).
    pub fn notice(&mut self, n: Notice) {
        self.notices.push(n);
        if self.notices.len() > 20 {
            self.notices.remove(0);
        }
    }

    /// Panel verisini yeniler.
    pub fn refresh_dashboard(&mut self) {
        if let Some(store) = &self.store {
            match birkat_app::build_dashboard(store, 30) {
                Ok(d) => self.dashboard = Some(d),
                Err(e) => self.notice(Notice::Error(format!("Panel yüklenemedi: {e}"))),
            }
        }
    }

    /// Sistem sağlığını yeniler.
    pub fn refresh_health(&mut self) {
        let model = self.settings.model_path.as_ref().map(PathBuf::from);
        self.health = Some(birkat_app::settings::check_health(
            &self.data_root,
            model.as_deref(),
            Some(&self.data_root.join("yedekler")),
        ));
    }

    /// Profil ekler.
    pub fn add_profile(&mut self) {
        let Some(store) = &mut self.store else {
            return;
        };
        if self.profile_name.trim().is_empty() {
            self.notice(Notice::Warning("Ad ve soyad boş olamaz.".to_string()));
            return;
        }
        let mut p = ExpertProfile::new(self.profile_name.trim());
        p.title = non_empty(&self.profile_title);
        p.registry_number = non_empty(&self.profile_registry);
        match store.save_profile(&p) {
            Ok(()) => {
                self.notice(Notice::Success(format!(
                    "Profil kaydedildi: {}",
                    p.full_name
                )));
                self.profile_name.clear();
                self.profile_title.clear();
                self.profile_registry.clear();
            }
            Err(e) => self.notice(Notice::Error(format!("Profil kaydedilemedi: {e}"))),
        }
    }

    /// Yeni vaka oluşturur ve çalışma alanını açar.
    pub fn create_case(&mut self) {
        let Some(store) = &mut self.store else {
            return;
        };
        if self.case_title.trim().is_empty() || self.case_court.trim().is_empty() {
            self.notice(Notice::Warning(
                "Vaka başlığı ve görevlendiren merci zorunludur.".to_string(),
            ));
            return;
        }
        let court = birkat_core::case::CourtOrAuthority {
            id: birkat_core::ids::CourtId::new(),
            name: self.case_court.trim().to_string(),
            unit_kind: "Mahkeme".to_string(),
            contact: None,
            source_ref: None,
        };
        let case = birkat_app::workspace::new_case_from_assignment(
            self.case_title.trim(),
            court,
            non_empty(&self.case_file_number),
            non_empty(&self.case_subject),
        );
        match CaseWorkspace::create(&self.data_root, case, store) {
            Ok(ws) => {
                self.selected_case = Some(ws.case.id.to_string());
                self.notice(Notice::Success(format!(
                    "Vaka oluşturuldu: {}",
                    ws.case.title
                )));
                self.workspace = Some(ws);
                self.screen = Screen::CaseWorkspace;
                self.case_title.clear();
                self.case_court.clear();
                self.case_file_number.clear();
                self.case_subject.clear();
                self.refresh_dashboard();
            }
            Err(e) => self.notice(Notice::Error(format!("Vaka oluşturulamadı: {e}"))),
        }
    }

    /// Seçili vakayı açar.
    pub fn open_case(&mut self, case_id: &str) {
        let Some(store) = &mut self.store else {
            return;
        };
        let id = case_id.parse::<birkat_core::ids::CaseId>().ok();
        let Some(id) = id else {
            self.notice(Notice::Error("Geçersiz vaka kimliği.".to_string()));
            return;
        };
        match CaseWorkspace::open(&self.data_root, &id, store) {
            Ok(ws) => {
                self.selected_case = Some(id.to_string());
                self.workspace = Some(ws);
                self.screen = Screen::CaseWorkspace;
                self.notice(Notice::Success("Vaka açıldı.".to_string()));
            }
            Err(e) => self.notice(Notice::Error(format!("Vaka açılamadı: {e}"))),
        }
    }

    /// Kasayı açar.
    pub fn unlock_vault(&mut self) {
        let Some(ws) = &mut self.workspace else {
            self.notice(Notice::Warning("Önce bir vaka açın.".to_string()));
            return;
        };
        if self.vault_password.is_empty() {
            self.notice(Notice::Warning("Parola boş olamaz.".to_string()));
            return;
        }
        match ws.unlock_vault(&self.vault_password) {
            Ok(()) => {
                self.notice(Notice::Success(
                    "Kasa açıldı (şifreleme etkin).".to_string(),
                ));
                self.vault_password.clear();
            }
            Err(e) => self.notice(Notice::Error(format!("Kasa açılamadı: {e}"))),
        }
    }

    /// Otomatik iş akışını çalıştırır.
    pub fn run_workflow(&mut self) {
        let Some(ws) = &mut self.workspace else {
            self.notice(Notice::Warning("Önce bir vaka açın.".to_string()));
            return;
        };
        let Some(store) = &mut self.store else {
            return;
        };
        if self.material_path.trim().is_empty() {
            self.notice(Notice::Warning("Materyal yolu boş olamaz.".to_string()));
            return;
        }
        let path = PathBuf::from(self.material_path.trim());
        if !path.exists() {
            self.notice(Notice::Error(format!(
                "Materyal yolu bulunamadı: {}",
                path.display()
            )));
            return;
        }
        let engine = birkat_transcription::engine::MockEngine::default();
        match birkat_app::run_automatic_workflow(ws, store, &[path], &engine, &self.data_root) {
            Ok(out) => {
                self.last_workflow_summary = Some(out.summary());
                self.notice(Notice::Success(out.summary()));
                for w in &out.warnings {
                    self.notice(Notice::Warning(w.clone()));
                }
                self.refresh_dashboard();
            }
            Err(e) => self.notice(Notice::Error(format!("İş akışı hatası: {e}"))),
        }
    }

    /// Rapor ve UDF üretir.
    pub fn generate_report(&mut self) {
        let Some(ws) = &mut self.workspace else {
            self.notice(Notice::Warning("Önce bir vaka açın.".to_string()));
            return;
        };
        let Some(store) = &mut self.store else {
            return;
        };
        let out_dir = self.data_root.join("cikti");
        if let Err(e) = std::fs::create_dir_all(&out_dir) {
            self.notice(Notice::Error(format!("Çıktı dizini oluşturulamadı: {e}")));
            return;
        }
        let experts = store.list_profiles().unwrap_or_default();
        let input = ws
            .vault
            .read_object("rapor/rapor-girdisi.json")
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_default();

        let report = birkat_report::build_report(
            &ws.case,
            &experts,
            &input,
            birkat_core::report::ReportKind::Main,
            1,
        );
        let udf =
            birkat_report::report_to_udf(&report, &birkat_report::ReportToUdfOptions::default());
        let udf_bytes = match birkat_udf::write_udf_to_bytes(udf) {
            Ok(b) => b,
            Err(e) => {
                self.notice(Notice::Error(format!("UDF üretilemedi: {e}")));
                return;
            }
        };
        let udf_path = out_dir.join("BILIRKISI_RAPORU.udf");
        if let Err(e) = std::fs::write(&udf_path, &udf_bytes) {
            self.notice(Notice::Error(format!("UDF yazılamadı: {e}")));
            return;
        }
        let v = birkat_udf::validate_udf(&udf_bytes);
        if !v.ok {
            for err in v.errors() {
                self.notice(Notice::Error(format!("UDF doğrulama: {}", err.message)));
            }
            self.notice(Notice::Error(
                "UDF doğrulaması başarısız; dosya hazır olarak sunulmaz.".to_string(),
            ));
            return;
        }

        // Yardımcı çıktılar.
        if let Ok(pdf) = birkat_report::pdf::report_to_pdf(&report) {
            let _ = std::fs::write(out_dir.join("BILIRKISI_RAPORU.pdf"), pdf);
        }
        if let Ok(docx) = birkat_report::docx::report_to_docx(&report) {
            let _ = std::fs::write(out_dir.join("BILIRKISI_RAPORU.docx"), docx);
        }
        let _ = std::fs::write(
            out_dir.join("BILIRKISI_RAPORU.txt"),
            birkat_report::text::to_text(&report),
        );

        let _ = ws.commit(
            store,
            birkat_audit::AuditAction::ReportGenerated,
            "Rapor ve UDF üretildi",
            Some(udf_path.display().to_string()),
            serde_json::json!({ "dogrulama": v.ok }),
        );

        self.last_report_dir = Some(out_dir.clone());
        self.notice(Notice::Success(format!(
            "Rapor üretildi: {} ({})",
            udf_path.display(),
            v.summary()
        )));
    }

    /// Vakayı arşivler.
    pub fn archive_case(&mut self) {
        let Some(ws) = &mut self.workspace else {
            return;
        };
        let Some(store) = &mut self.store else {
            return;
        };
        match ws.archive(store) {
            Ok(()) => {
                self.notice(Notice::Success("Vaka arşivlendi.".to_string()));
                self.refresh_dashboard();
            }
            Err(e) => self.notice(Notice::Error(format!("Arşivleme hatası: {e}"))),
        }
    }

    /// Şifreli yedek oluşturur ve gerçek geri yükleme testini yapar.
    pub fn create_backup(&mut self) {
        let Some(ws) = &self.workspace else {
            self.notice(Notice::Warning("Önce bir vaka açın.".to_string()));
            return;
        };
        if self.vault_password.chars().count() < 12 {
            self.notice(Notice::Warning(
                "Yedek parolası en az 12 karakter olmalıdır.".to_string(),
            ));
            return;
        }
        let backup_dir = self.data_root.join("yedekler");
        let _ = std::fs::create_dir_all(&backup_dir);
        let path = backup_dir.join(format!("{}.yedek", ws.case.id));
        let opts = birkat_backup::BackupOptions {
            password: self.vault_password.clone(),
            ..Default::default()
        };
        match birkat_backup::create_backup(
            ws.vault.root(),
            &path,
            &opts,
            vec![ws.case.id.to_string()],
            None,
            None,
        ) {
            Ok(r) => {
                self.notice(Notice::Success(format!(
                    "Yedek oluşturuldu: {} ({} dosya)",
                    r.path.display(),
                    r.manifest.file_count()
                )));
                // Gerçek geri yükleme testi (19.4).
                match birkat_backup::verify_restore(&path, &self.vault_password, &self.data_root) {
                    Ok(vr) => self.notice(if vr.ok() {
                        Notice::Success(format!("Geri yükleme testi: {}", vr.summary()))
                    } else {
                        Notice::Error(format!("Geri yükleme testi başarısız: {}", vr.summary()))
                    }),
                    Err(e) => self.notice(Notice::Error(format!("Geri yükleme testi hatası: {e}"))),
                }
            }
            Err(e) => self.notice(Notice::Error(format!("Yedek alınamadı: {e}"))),
        }
    }

    /// İşlem geçmişi zincirini doğrular.
    pub fn verify_audit(&mut self) {
        let Some(ws) = &self.workspace else {
            self.notice(Notice::Warning("Önce bir vaka açın.".to_string()));
            return;
        };
        let v = ws.verify_audit();
        if v.valid {
            self.notice(Notice::Success(format!(
                "{} ({} olay doğrulandı)",
                v.message, v.checked
            )));
        } else {
            self.notice(Notice::Error(v.message));
        }
    }

    /// Arama sonuçlarını filtreler.
    pub fn matches_search(&self, text: &str) -> bool {
        if self.search.trim().is_empty() {
            return true;
        }
        text.to_lowercase()
            .contains(&self.search.trim().to_lowercase())
    }

    /// Açık vakanın durumu.
    pub fn current_state(&self) -> Option<WorkflowState> {
        self.workspace.as_ref().map(|w| w.case.workflow.state)
    }

    /// Kayıtlı vakaları döndürür.
    pub fn cases(&self) -> Vec<Case> {
        self.store
            .as_ref()
            .and_then(|s| s.list_cases().ok())
            .unwrap_or_default()
    }

    /// Kayıtlı profilleri döndürür.
    pub fn profiles(&self) -> Vec<ExpertProfile> {
        self.store
            .as_ref()
            .and_then(|s| s.list_profiles().ok())
            .unwrap_or_default()
    }

    /// Ayarları uygular (tema, ölçek).
    pub fn apply_settings(&mut self, ctx: &egui::Context) {
        let theme = match self.settings.theme {
            birkat_app::settings::Theme::Light => egui::ThemePreference::Light,
            birkat_app::settings::Theme::Dark => egui::ThemePreference::Dark,
            birkat_app::settings::Theme::System => egui::ThemePreference::System,
        };
        ctx.set_theme(theme);
        ctx.set_pixels_per_point(self.settings.ui_scale);
    }

    /// Kısayolları işler.
    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let (ctrl_s, ctrl_d, ctrl_r, ctrl_b, f5) = ctx.input(|i| {
            (
                i.key_pressed(Key::S) && i.modifiers.matches_logically(Modifiers::COMMAND),
                i.key_pressed(Key::D) && i.modifiers.matches_logically(Modifiers::COMMAND),
                i.key_pressed(Key::R) && i.modifiers.matches_logically(Modifiers::COMMAND),
                i.key_pressed(Key::B) && i.modifiers.matches_logically(Modifiers::COMMAND),
                i.key_pressed(Key::F5),
            )
        });
        if ctrl_s {
            self.notice(Notice::Info(
                "Otomatik kaydetme: tüm değişiklikler kayıtlı.".to_string(),
            ));
        }
        if ctrl_d {
            self.screen = Screen::Dashboard;
            self.refresh_dashboard();
        }
        if ctrl_r {
            self.generate_report();
        }
        if ctrl_b {
            self.create_backup();
        }
        if f5 {
            self.refresh_dashboard();
            self.refresh_health();
            self.notice(Notice::Info("Veriler yenilendi.".to_string()));
        }
    }
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

impl eframe::App for BirkatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_settings(ctx);
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("ust_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("BİRKÂT");
                ui.label(format!(
                    "— {} Bilirkişi Dijital Çalışma Operatörü",
                    birkat_core::SPECIALTY_CODE
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Sürüm {}", birkat_core::APP_VERSION));
                    if let Some(state) = self.current_state() {
                        ui.separator();
                        ui.label(format!("Açık vaka durumu: {}", state.label()));
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label("Ara:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("dosya, vaka veya metin ara")
                        .desired_width(240.0),
                );
                if ui.button("Temizle").clicked() {
                    self.search.clear();
                }
                ui.separator();
                if ui.button("Paneli yenile (F5)").clicked() {
                    self.refresh_dashboard();
                    self.refresh_health();
                }
            });
        });

        egui::SidePanel::left("menu")
            .resizable(true)
            .default_width(240.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Çalışma alanları").strong());
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for s in Screen::menu() {
                        if ui.selectable_label(self.screen == s, s.title()).clicked() {
                            self.screen = s;
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("bildirimler")
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Bildirimler ve istisnalar").strong());
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.notices.is_empty() {
                        ui.weak("Bildirim yok.");
                    }
                    for n in &self.notices {
                        let (mark, text) = match n {
                            Notice::Info(t) => ("ℹ", t),
                            Notice::Success(t) => ("✔", t),
                            Notice::Warning(t) => ("!", t),
                            Notice::Error(t) => ("✖", t),
                        };
                        ui.label(format!("{mark} {text}"));
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(self.screen.title());
            ui.separator();
            crate::screens::render(self, ui);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> BirkatApp {
        let dir = tempfile::tempdir().unwrap();
        // tempdir yaşam süresi test boyunca tutulur.
        let root = dir.path().to_path_buf();
        std::mem::forget(dir);
        BirkatApp::new(root)
    }

    #[test]
    fn all_screens_have_turkish_titles() {
        for s in Screen::menu() {
            assert!(!s.title().is_empty());
        }
        assert_eq!(Screen::menu().len(), 20, "20 çalışma alanı olmalı");
    }

    #[test]
    fn initialization_opens_store() {
        let a = app();
        assert!(a.store.is_some());
        assert!(a.dashboard.is_some());
        assert!(a.health.is_some());
    }

    #[test]
    fn adding_profile_requires_name() {
        let mut a = app();
        a.profile_name = "".to_string();
        a.add_profile();
        assert!(a.notices.iter().any(|n| matches!(n, Notice::Warning(_))));
    }

    #[test]
    fn adding_profile_persists() {
        let mut a = app();
        a.profile_name = "Ayşe Yılmaz".to_string();
        a.profile_registry = "12345".to_string();
        a.add_profile();
        assert_eq!(a.profiles().len(), 1);
        assert!(a.notices.iter().any(|n| matches!(n, Notice::Success(_))));
    }

    #[test]
    fn creating_case_opens_workspace() {
        let mut a = app();
        a.case_title = "2025/123".to_string();
        a.case_court = "Ankara 5. ACCM".to_string();
        a.create_case();
        assert!(a.workspace.is_some());
        assert_eq!(a.screen, Screen::CaseWorkspace);
        assert_eq!(a.cases().len(), 1);
    }

    #[test]
    fn case_requires_title_and_court() {
        let mut a = app();
        a.case_title = "X".to_string();
        a.create_case();
        assert!(a.workspace.is_none());
        assert!(a.notices.iter().any(|n| matches!(n, Notice::Warning(_))));
    }

    #[test]
    fn workflow_requires_material_path() {
        let mut a = app();
        a.case_title = "X".to_string();
        a.case_court = "Y".to_string();
        a.create_case();
        a.material_path = String::new();
        a.run_workflow();
        assert!(a.last_workflow_summary.is_none());
    }

    #[test]
    fn workflow_rejects_missing_path() {
        let mut a = app();
        a.case_title = "X".to_string();
        a.case_court = "Y".to_string();
        a.create_case();
        a.material_path = "/boyle/bir/yol/yok".to_string();
        a.run_workflow();
        assert!(a.notices.iter().any(|n| matches!(n, Notice::Error(_))));
    }

    #[test]
    fn report_generation_requires_case() {
        let mut a = app();
        a.generate_report();
        assert!(a.notices.iter().any(|n| matches!(n, Notice::Warning(_))));
    }

    #[test]
    fn backup_requires_long_password() {
        let mut a = app();
        a.case_title = "X".to_string();
        a.case_court = "Y".to_string();
        a.create_case();
        a.vault_password = "kısa".to_string();
        a.create_backup();
        assert!(a
            .notices
            .iter()
            .any(|n| matches!(n, Notice::Warning(t) if t.contains("12"))));
    }

    #[test]
    fn audit_verification_reports_success() {
        let mut a = app();
        a.case_title = "X".to_string();
        a.case_court = "Y".to_string();
        a.create_case();
        a.verify_audit();
        assert!(a
            .notices
            .iter()
            .any(|n| matches!(n, Notice::Success(t) if t.contains("doğrulandı"))));
    }

    #[test]
    fn search_matching_is_case_insensitive() {
        let mut a = app();
        a.search = "KAYIT".to_string();
        assert!(a.matches_search("kayit1.wav"));
        a.search.clear();
        assert!(a.matches_search("herhangi"));
    }

    #[test]
    fn notices_are_bounded() {
        let mut a = app();
        for i in 0..30 {
            a.notice(Notice::Info(format!("bildirim {i}")));
        }
        assert!(a.notices.len() <= 20);
    }
}
