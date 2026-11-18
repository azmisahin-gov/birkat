//! İlk çalıştırma kurulum sihirbazı (şartname 8).
//!
//! Uygulama ilk açıldığında kullanıcıyı teknik geliştirme ortamıyla
//! karşılaştırmamalıdır. Bu sihirbaz, bilirkişiden yalnızca **mesleki
//! bilgileri** ve **güvenlik tercihini** ister; model, codec, Python, dosya
//! yolu veya terminal gibi teknik ayrıntılar sorulmaz.
//!
//! ## Adımlar
//!
//! 1. Hoş geldiniz ve ürün tanıtımı.
//! 2. Bilirkişi profili (ad, unvan, sicil, iletişim).
//! 3. Veri ve yedek klasörü.
//! 4. Kasa parolası + kurtarma anahtarı.
//! 5. Sistem denetimi (model, veritabanı, kasa) ve bitiş.

use crate::app::{BirkatApp, Notice, Screen};
use birkat_core::profile::ExpertProfile;
use eframe::egui;

/// Sihirbaz adımı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    /// Hoş geldiniz.
    Welcome,
    /// Bilirkişi profili.
    Profile,
    /// Klasörler.
    Folders,
    /// Kasa parolası.
    Vault,
    /// Sistem denetimi.
    SystemCheck,
    /// Tamamlandı.
    Done,
}

impl WizardStep {
    /// Adımın başlığı.
    pub fn title(&self) -> &'static str {
        match self {
            WizardStep::Welcome => "BİRKÂT'a hoş geldiniz",
            WizardStep::Profile => "Bilirkişi profili",
            WizardStep::Folders => "Veri ve yedek klasörü",
            WizardStep::Vault => "Kasa parolası",
            WizardStep::SystemCheck => "Sistem denetimi",
            WizardStep::Done => "Kurulum tamamlandı",
        }
    }

    /// Adım numarası (1 tabanlı); bitiş adımı 0 döner.
    pub fn ordinal(&self) -> usize {
        match self {
            WizardStep::Welcome => 1,
            WizardStep::Profile => 2,
            WizardStep::Folders => 3,
            WizardStep::Vault => 4,
            WizardStep::SystemCheck => 5,
            WizardStep::Done => 0,
        }
    }

    /// Sonraki adım.
    pub fn next(&self) -> WizardStep {
        match self {
            WizardStep::Welcome => WizardStep::Profile,
            WizardStep::Profile => WizardStep::Folders,
            WizardStep::Folders => WizardStep::Vault,
            WizardStep::Vault => WizardStep::SystemCheck,
            WizardStep::SystemCheck => WizardStep::Done,
            WizardStep::Done => WizardStep::Done,
        }
    }

    /// Önceki adım.
    pub fn prev(&self) -> WizardStep {
        match self {
            WizardStep::Welcome => WizardStep::Welcome,
            WizardStep::Profile => WizardStep::Welcome,
            WizardStep::Folders => WizardStep::Profile,
            WizardStep::Vault => WizardStep::Folders,
            WizardStep::SystemCheck => WizardStep::Vault,
            WizardStep::Done => WizardStep::Done,
        }
    }

    /// Toplam adım sayısı (bitiş hariç).
    pub const TOTAL: usize = 5;
}

/// Sihirbaz durumu.
#[derive(Debug, Clone)]
pub struct SetupWizard {
    /// Geçerli adım.
    pub step: WizardStep,
    /// Adım doğrulama hatası.
    pub error: Option<String>,
    /// Kurtarma anahtarı üretildi mi?
    pub recovery_key: Option<String>,
    /// Kullanıcı kurtarma anahtarını kaydettiğini onayladı mı?
    pub recovery_confirmed: bool,
    /// Sistem denetimi sonucu.
    pub system_ok: bool,
    /// Sistem denetimi uyarıları.
    pub system_notes: Vec<String>,
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self {
            step: WizardStep::Welcome,
            error: None,
            recovery_key: None,
            recovery_confirmed: false,
            system_ok: false,
            system_notes: Vec::new(),
        }
    }
}

impl SetupWizard {
    /// Sihirbazı çizer.
    pub fn ui(&mut self, app: &mut BirkatApp, ui: &mut egui::Ui) {
        ui.heading(self.step.title());
        if self.step.ordinal() > 0 {
            ui.label(format!(
                "Adım {} / {}",
                self.step.ordinal(),
                WizardStep::TOTAL
            ));
            let ilerleme = self.step.ordinal() as f32 / WizardStep::TOTAL as f32;
            ui.add(egui::ProgressBar::new(ilerleme).desired_width(320.0));
        }
        ui.separator();

        if let Some(e) = &self.error {
            ui.colored_label(egui::Color32::from_rgb(200, 60, 60), format!("! {e}"));
            ui.separator();
        }

        egui::ScrollArea::vertical()
            .max_height(420.0)
            .show(ui, |ui| match self.step {
                WizardStep::Welcome => self.welcome(ui),
                WizardStep::Profile => self.profile(app, ui),
                WizardStep::Folders => self.folders(app, ui),
                WizardStep::Vault => self.vault(app, ui),
                WizardStep::SystemCheck => self.system_check(app, ui),
                WizardStep::Done => self.done(ui),
            });

        ui.separator();
        ui.horizontal(|ui| {
            if self.step != WizardStep::Welcome
                && self.step != WizardStep::Done
                && ui.button("← Geri").clicked()
            {
                self.step = self.step.prev();
                self.error = None;
            }
            match self.step {
                WizardStep::Done => {
                    if ui.button("BİRKÂT'ı kullanmaya başla").clicked() {
                        app.wizard = None;
                        app.screen = Screen::Dashboard;
                        app.notice(Notice::Success(
                            "Kurulum tamamlandı. BİRKÂT kullanıma hazır.".to_string(),
                        ));
                    }
                }
                WizardStep::SystemCheck => {
                    if ui.button("Yeniden denetle").clicked() {
                        self.run_system_check(app);
                    }
                    if ui.button("Bitir").clicked() {
                        if !self.system_ok {
                            self.error = Some(
                                "Sistem denetiminde giderilmesi gereken sorunlar var. \
                                 Devam etmeden önce uyarıları okuyun."
                                    .to_string(),
                            );
                        } else {
                            self.step = WizardStep::Done;
                            self.error = None;
                        }
                    }
                }
                _ => {
                    if ui.button("İleri →").clicked() {
                        self.advance(app);
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Sihirbazı atla").clicked() {
                    app.wizard = None;
                    app.notice(Notice::Warning(
                        "Kurulum sihirbazı atlandı. Profil ve kasa ayarlarını \
                         Sistem sağlığı ekranından tamamlayabilirsiniz."
                            .to_string(),
                    ));
                }
            });
        });
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "BİRKÂT, 56.12 (Ses Kayıtlarının Metin Hâline Dönüştürülmesi) \
             bilirkişileri için geliştirilmiş yerel ve çevrimdışı bir çalışma \
             uygulamasıdır.",
        );
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Bu sihirbaz ne yapar?").strong());
        for s in [
            "Bilirkişi profilinizi oluşturur (raporlarda otomatik kullanılır).",
            "Veri ve yedek klasörlerini belirler.",
            "Vaka kasası için parola belirlemenizi sağlar.",
            "Sistemin hazır olup olmadığını denetler.",
        ] {
            ui.label(format!("• {s}"));
        }
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Neler sorulmaz?").strong());
        ui.label(
            "Model, codec, GPU, Python, FFmpeg veya dosya yolu gibi teknik \
             ayrıntılar sorulmaz. Whisper modeli kurulum paketinde gelir ve \
             internet bağlantısı gerekmez.",
        );
    }

    fn profile(&mut self, app: &mut BirkatApp, ui: &mut egui::Ui) {
        ui.label("Bu bilgiler her raporda otomatik kullanılır; tekrar yazmanız gerekmez.");
        ui.separator();
        egui::Grid::new("sihirbaz_profil").show(ui, |ui| {
            ui.label("Ad ve soyad *");
            ui.text_edit_singleline(&mut app.profile_name);
            ui.end_row();
            ui.label("Unvan");
            ui.text_edit_singleline(&mut app.profile_title);
            ui.end_row();
            ui.label("Bilirkişi sicil numarası");
            ui.text_edit_singleline(&mut app.profile_registry);
            ui.end_row();
            ui.label("Kurum (varsa)");
            ui.text_edit_singleline(&mut app.profile_institution);
            ui.end_row();
            ui.label("Telefon");
            ui.text_edit_singleline(&mut app.profile_phone);
            ui.end_row();
            ui.label("E-posta");
            ui.text_edit_singleline(&mut app.profile_email);
            ui.end_row();
        });
        ui.add_space(6.0);
        ui.label("Uzmanlık alanı: 56.12 — Ses Kayıtlarının Metin Hâline Dönüştürülmesi");
    }

    fn folders(&mut self, app: &mut BirkatApp, ui: &mut egui::Ui) {
        ui.label("Vaka verileri ve yedekler bu klasörlerde saklanır.");
        ui.separator();
        egui::Grid::new("sihirbaz_klasor").show(ui, |ui| {
            ui.label("Veri kök dizini *");
            ui.text_edit_singleline(&mut app.data_root_text);
            ui.end_row();
            ui.label("Yedek dizini");
            ui.text_edit_singleline(&mut app.backup_dir_text);
            ui.end_row();
        });
        ui.add_space(6.0);
        ui.label(
            "Yedek dizinini farklı bir disk veya ağ konumu seçmeniz önerilir. \
             Yedekler şifreli oluşturulur.",
        );
    }

    fn vault(&mut self, app: &mut BirkatApp, ui: &mut egui::Ui) {
        ui.label(
            "Vaka kasası AES-256-GCM ile şifrelenir. Parola hiçbir yere \
             kaydedilmez.",
        );
        ui.separator();
        egui::Grid::new("sihirbaz_kasa").show(ui, |ui| {
            ui.label("Parola *");
            ui.add(
                egui::TextEdit::singleline(&mut app.vault_password)
                    .password(true)
                    .desired_width(240.0),
            );
            ui.end_row();
            ui.label("Parola (tekrar) *");
            ui.add(
                egui::TextEdit::singleline(&mut app.vault_password_confirm)
                    .password(true)
                    .desired_width(240.0),
            );
            ui.end_row();
        });

        // Parola gücü geri bildirimi
        if !app.vault_password.is_empty() {
            let politika = birkat_crypto::check_password_policy(&app.vault_password);
            if politika.ok {
                ui.colored_label(egui::Color32::from_rgb(40, 140, 60), "✔ Parola güçlü.");
            } else {
                for i in &politika.issues {
                    ui.colored_label(egui::Color32::from_rgb(200, 140, 40), format!("• {i}"));
                }
            }
        }

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Kurtarma anahtarı").strong());
        if let Some(k) = &self.recovery_key {
            ui.label(
                "Aşağıdaki kurtarma anahtarını yazdırın veya güvenli bir yere kaydedin. \
                 Parolayı unutursanız kasayı yalnızca bu anahtarla açabilirsiniz.",
            );
            let mut text = k.clone();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(2),
            );
            ui.checkbox(
                &mut self.recovery_confirmed,
                "Kurtarma anahtarını güvenli bir yere kaydettim.",
            );
        } else {
            ui.label("Parolayı girdikten sonra kurtarma anahtarı üretilecektir.");
        }
    }

    fn system_check(&mut self, app: &mut BirkatApp, ui: &mut egui::Ui) {
        if !self.system_ok && self.system_notes.is_empty() {
            self.run_system_check(app);
        }
        if self.system_ok {
            ui.colored_label(
                egui::Color32::from_rgb(40, 140, 60),
                "✔ Sistem BİRKÂT kullanımına hazır.",
            );
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(200, 60, 60),
                "! Giderilmesi gereken sorunlar var:",
            );
        }
        ui.separator();
        for n in &self.system_notes {
            ui.label(format!("• {n}"));
        }
        ui.add_space(6.0);
        ui.label(
            "Model, veritabanı ve kasa denetimi otomatik yapılır. Teknik \
             ayrıntılarla uğraşmanız gerekmez.",
        );
    }

    fn done(&mut self, ui: &mut egui::Ui) {
        ui.colored_label(
            egui::Color32::from_rgb(40, 140, 60),
            "✔ Kurulum tamamlandı.",
        );
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Sıradaki adımlar").strong());
        for s in [
            "Görevlendirme ve dosya kabul ekranından yeni vaka oluşturun.",
            "Görevlendirme belgesini ve ses dosyalarını verin.",
            "Uygulama envanteri, transkripsiyonu ve kalite kontrolünü yürütür.",
            "Transkriptleri Ses/transkript inceleme ekranında doğrulayın.",
            "Rapor editöründen UDF raporunu üretin ve doğrulayın.",
        ] {
            ui.label(format!("• {s}"));
        }
        ui.add_space(8.0);
        ui.label(
            "Bilirkişilik Yönetmeliği gereği mesleki değerlendirme ve nihai \
             onay her zaman size aittir. Uygulama makine çıktısını asla \
             doğrulanmış içerik olarak sunmaz.",
        );
    }

    /// Adımı doğrular ve ilerletir.
    fn advance(&mut self, app: &mut BirkatApp) {
        self.error = None;
        match self.step {
            WizardStep::Welcome => {
                self.step = WizardStep::Profile;
            }
            WizardStep::Profile => {
                if app.profile_name.trim().is_empty() {
                    self.error = Some("Ad ve soyad zorunludur.".to_string());
                    return;
                }
                app.save_wizard_profile();
                self.step = WizardStep::Folders;
            }
            WizardStep::Folders => {
                if app.data_root_text.trim().is_empty() {
                    self.error = Some("Veri kök dizini zorunludur.".to_string());
                    return;
                }
                app.apply_wizard_folders();
                self.step = WizardStep::Vault;
            }
            WizardStep::Vault => {
                if app.vault_password != app.vault_password_confirm {
                    self.error = Some("Parolalar eşleşmiyor.".to_string());
                    return;
                }
                let politika = birkat_crypto::check_password_policy(&app.vault_password);
                if !politika.ok {
                    self.error = Some(format!(
                        "Parola yeterince güçlü değil: {}",
                        politika.issues.join(" ")
                    ));
                    return;
                }
                if self.recovery_key.is_none() {
                    self.recovery_key = Some(app.generate_recovery_key());
                    return; // Kullanıcı anahtarı kaydetsin
                }
                if !self.recovery_confirmed {
                    self.error = Some(
                        "Devam etmeden önce kurtarma anahtarını kaydettiğinizi \
                         onaylamanız gerekir."
                            .to_string(),
                    );
                    return;
                }
                app.save_wizard_vault_password();
                self.step = WizardStep::SystemCheck;
            }
            _ => {}
        }
    }

    /// Sistem denetimini çalıştırır.
    pub fn run_system_check(&mut self, app: &mut BirkatApp) {
        app.refresh_health();
        let mut notes = Vec::new();
        let mut ok = true;

        // Veritabanı
        if app.store.is_some() {
            notes.push("Veritabanı: hazır".to_string());
        } else {
            notes.push("Veritabanı: AÇILAMADI".to_string());
            ok = false;
        }

        // Kasa
        let kasa = app.data_root.join("kasalar");
        match std::fs::create_dir_all(&kasa) {
            Ok(()) => notes.push("Vaka kasası: hazır".to_string()),
            Err(e) => {
                notes.push(format!("Vaka kasası: OLUŞTURULAMADI ({e})"));
                ok = false;
            }
        }

        // Whisper modeli
        match birkat_transcription::discover_model_dir(&app.data_root) {
            Some(p) => notes.push(format!("Whisper modeli: hazır ({})", p.root.display())),
            None => {
                notes.push(
                    "Whisper modeli: BULUNAMADI — transkripsiyon çalışmayacak. \
                     Kurulum paketini kullanın veya model klasörünü \
                     BIRKAT_MODEL_DIR ile belirtin."
                        .to_string(),
                );
                ok = false;
            }
        }

        // Whisper derleme desteği
        if birkat_transcription::runtime::whisper_compiled() {
            notes.push("Whisper desteği: derlemede etkin".to_string());
        } else {
            notes.push(
                "Whisper desteği: derlemede KAPALI — bu derlemede gerçek \
                 transkripsiyon yapılamaz."
                    .to_string(),
            );
            ok = false;
        }

        // Yedek dizini
        if !app.backup_dir_text.trim().is_empty() {
            match std::fs::create_dir_all(app.backup_dir_text.trim()) {
                Ok(()) => notes.push("Yedek dizini: hazır".to_string()),
                Err(e) => {
                    notes.push(format!("Yedek dizini: HAZIRLANAMADI ({e})"));
                    ok = false;
                }
            }
        }

        // Çevrimdışı çalışma hatırlatması
        notes.push(
            "Çevrimdışı çalışma: uygulama vaka verisini hiçbir sunucuya \
             göndermez."
                .to_string(),
        );

        self.system_ok = ok;
        self.system_notes = notes;
    }
}

/// Sihirbazın gösterilip gösterilmeyeceğine karar verir.
///
/// Profil yoksa veya kasa anahtarı oluşturulmamışsa sihirbaz gösterilir.
pub fn needs_setup(app: &BirkatApp) -> bool {
    let profil_yok = app.profiles().is_empty();
    let kasa_yok = !app.data_root.join("kasalar").exists();
    profil_yok || kasa_yok
}

/// Profil bilgisini sihirbazdan kaydeder (test edilebilir çekirdek).
pub fn build_profile_from_wizard(
    name: &str,
    title: &str,
    registry: &str,
    institution: &str,
    phone: &str,
    email: &str,
) -> ExpertProfile {
    let mut p = ExpertProfile::new(name.trim());
    p.title = non_empty(title);
    p.registry_number = non_empty(registry);
    p.institution = non_empty(institution);
    p.contact.phone = non_empty(phone);
    p.contact.email = non_empty(email);
    p
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wizard_steps_progress_and_return() {
        let mut s = WizardStep::Welcome;
        assert_eq!(s.ordinal(), 1);
        for beklenen in 2..=WizardStep::TOTAL {
            s = s.next();
            assert_eq!(s.ordinal(), beklenen);
        }
        assert_eq!(s, WizardStep::SystemCheck);
        assert_eq!(s.next(), WizardStep::Done);
        assert_eq!(s.prev(), WizardStep::Vault);
    }

    #[test]
    fn all_steps_have_titles() {
        for s in [
            WizardStep::Welcome,
            WizardStep::Profile,
            WizardStep::Folders,
            WizardStep::Vault,
            WizardStep::SystemCheck,
            WizardStep::Done,
        ] {
            assert!(!s.title().is_empty());
        }
    }

    #[test]
    fn profile_built_from_wizard_fields() {
        let p = build_profile_from_wizard(
            "Ayşe Yılmaz",
            "Dr.",
            "12345",
            "Adli Tıp Kurumu",
            "0555 000 00 00",
            "a@b.c",
        );
        assert_eq!(p.full_name, "Ayşe Yılmaz");
        assert_eq!(p.title.as_deref(), Some("Dr."));
        assert_eq!(p.registry_number.as_deref(), Some("12345"));
        assert_eq!(p.institution.as_deref(), Some("Adli Tıp Kurumu"));
        assert_eq!(p.contact.phone.as_deref(), Some("0555 000 00 00"));
        assert_eq!(p.specialty, "56.12");
    }

    #[test]
    fn empty_optional_fields_become_none() {
        let p = build_profile_from_wizard("A", "", "  ", "", "", "");
        assert!(p.title.is_none());
        assert!(p.registry_number.is_none());
        assert!(p.institution.is_none());
        assert!(p.contact.phone.is_none());
    }

    #[test]
    fn default_wizard_starts_at_welcome() {
        let w = SetupWizard::default();
        assert_eq!(w.step, WizardStep::Welcome);
        assert!(w.recovery_key.is_none());
        assert!(!w.recovery_confirmed);
    }

    #[test]
    fn system_check_reports_missing_model() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = BirkatApp::new(dir.path().to_path_buf());
        std::mem::forget(dir);
        let mut w = SetupWizard::default();
        w.run_system_check(&mut app);
        // Model yoksa denetim başarısız olmalı ve neden açıkça yazılmalı.
        assert!(w.system_notes.iter().any(|n| n.contains("Whisper modeli")));
        assert!(w
            .system_notes
            .iter()
            .any(|n| n.to_lowercase().contains("çevrimdışı")));
    }
}
