//! Yirmi çalışma alanı (24. madde).
//!
//! Her ekran gerçek servis katmanına bağlıdır; sahte düğme veya maket ekran
//! yoktur. Ekran sayısı fazla olsa bile kullanıcı gereksiz menü dolaşmaya
//! zorlanmaz; aktif dosya içindeyken dosyanın tüm işlevlerine bağlam içinde
//! erişilebilir.

use crate::app::{BirkatApp, Notice, Screen};
use eframe::egui;

/// Ekranı çizer.
pub fn render(app: &mut BirkatApp, ui: &mut egui::Ui) {
    match app.screen {
        Screen::Dashboard => dashboard(app, ui),
        Screen::Profile => profile(app, ui),
        Screen::Assignment => assignment(app, ui),
        Screen::CaseWorkspace => case_workspace(app, ui),
        Screen::Evidence => evidence(app, ui),
        Screen::Device => device(app, ui),
        Screen::AudioInventory => audio_inventory(app, ui),
        Screen::Transcription => transcription(app, ui),
        Screen::AudioReview => audio_review(app, ui),
        Screen::Questions => questions(app, ui),
        Screen::Findings => findings(app, ui),
        Screen::ReportEditor => report_editor(app, ui),
        Screen::UdfPreview => udf_preview(app, ui),
        Screen::Signature => signature(app, ui),
        Screen::AuxiliaryDocuments => auxiliary(app, ui),
        Screen::Calendar => calendar(app, ui),
        Screen::Archive => archive(app, ui),
        Screen::Backup => backup(app, ui),
        Screen::Update => update(app, ui),
        Screen::Health => health(app, ui),
    }
}

fn dashboard(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let dashboard = app.dashboard.clone();
    if let Some(d) = &dashboard {
        ui.label(d.summary());
        ui.separator();

        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("Devam eden dosyalar").strong());
            let cases: Vec<_> = d
                .active_cases
                .iter()
                .filter(|c| app.matches_search(&c.title))
                .cloned()
                .collect();
            if cases.is_empty() {
                cols[0].weak("Devam eden dosya yok.");
            }
            for c in &cases {
                let text = format!(
                    "{} — {} {}",
                    c.title,
                    c.state,
                    c.due_in.clone().unwrap_or_default()
                );
                if cols[0].button(text).clicked() {
                    app.open_case(&c.case_id);
                }
            }

            cols[1].label(egui::RichText::new("İnceleme bekleyen istisnalar").strong());
            if d.pending_exceptions.is_empty() {
                cols[1].weak("İstisna yok.");
            }
            for (t, m) in &d.pending_exceptions {
                cols[1].label(format!("! [{t}] {m}"));
            }
        });

        ui.separator();
        ui.columns(3, |cols| {
            cols[0].label(egui::RichText::new("Rapor kontrolü bekleyen").strong());
            for c in &d.awaiting_report_review {
                if cols[0].button(&c.title).clicked() {
                    app.open_case(&c.case_id);
                }
            }
            if d.awaiting_report_review.is_empty() {
                cols[0].weak("Yok.");
            }

            cols[1].label(egui::RichText::new("Teslim bekleyen").strong());
            for c in &d.awaiting_delivery {
                if cols[1].button(&c.title).clicked() {
                    app.open_case(&c.case_id);
                }
            }
            if d.awaiting_delivery.is_empty() {
                cols[1].weak("Yok.");
            }

            cols[2].label(egui::RichText::new("Yaklaşan görev süreleri").strong());
            for (t, l) in &d.upcoming_deadlines {
                cols[2].label(format!("[{t}] {l}"));
            }
            if d.upcoming_deadlines.is_empty() {
                cols[2].weak("Yaklaşan süre yok.");
            }
            cols[2].separator();
            cols[2].label(format!("Arşivlenmiş: {}", d.archived_count));
            cols[2].label(format!("Toplam dosya: {}", d.total_cases));
        });
    } else {
        ui.weak("Panel verisi yükleniyor...");
    }
}

fn profile(app: &mut BirkatApp, ui: &mut egui::Ui) {
    ui.label("Bilirkişi profili oluşturulur; kişisel bilgiler her dosyada yeniden yazılmaz.");
    ui.separator();
    egui::Grid::new("profil_formu").show(ui, |ui| {
        ui.label("Ad ve soyad");
        ui.text_edit_singleline(&mut app.profile_name);
        ui.end_row();
        ui.label("Unvan");
        ui.text_edit_singleline(&mut app.profile_title);
        ui.end_row();
        ui.label("Bilirkişi sicil numarası");
        ui.text_edit_singleline(&mut app.profile_registry);
        ui.end_row();
    });
    if ui.button("Profili kaydet").clicked() {
        app.add_profile();
    }
    ui.separator();
    ui.label(egui::RichText::new("Kayıtlı profiller").strong());
    egui::Grid::new("profil_listesi").striped(true).show(ui, |ui| {
        ui.label("Ad Soyad");
        ui.label("Unvan");
        ui.label("Sicil No");
        ui.label("Uzmanlık");
        ui.end_row();
        for p in app.profiles() {
            ui.label(&p.full_name);
            ui.label(p.title.as_deref().unwrap_or("-"));
            ui.label(p.registry_number.as_deref().unwrap_or("-"));
            ui.label(&p.specialty);
            ui.end_row();
        }
    });
}

fn assignment(app: &mut BirkatApp, ui: &mut egui::Ui) {
    ui.label("Yeni görevlendirme açılır; belgeler ve sorular bu dosyaya bağlanır.");
    ui.separator();
    egui::Grid::new("vaka_formu").show(ui, |ui| {
        ui.label("Vaka başlığı");
        ui.text_edit_singleline(&mut app.case_title);
        ui.end_row();
        ui.label("Görevlendiren merci");
        ui.text_edit_singleline(&mut app.case_court);
        ui.end_row();
        ui.label("Dosya esas numarası");
        ui.text_edit_singleline(&mut app.case_file_number);
        ui.end_row();
        ui.label("İnceleme konusu");
        ui.text_edit_singleline(&mut app.case_subject);
        ui.end_row();
    });
    if ui.button("Vaka oluştur ve aç").clicked() {
        app.create_case();
    }
    ui.separator();
    ui.label(egui::RichText::new("Kayıtlı vakalar").strong());
    egui::Grid::new("vaka_listesi").striped(true).show(ui, |ui| {
        ui.label("Başlık");
        ui.label("Dosya No");
        ui.label("Durum");
        ui.label("İşlem");
        ui.end_row();
        let cases = app.cases();
        for c in cases {
            if !app.matches_search(&c.title) {
                continue;
            }
            ui.label(&c.title);
            ui.label(c.assignment.file_number.as_deref().unwrap_or("-"));
            ui.label(c.workflow.state.label());
            if ui.button("Aç").clicked() {
                app.open_case(c.id.as_str());
            }
            ui.end_row();
        }
    });
}

fn case_workspace(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let Some(ws) = &app.workspace else {
        ui.weak("Açık vaka yok. Görevlendirme ekranından vaka oluşturun.");
        return;
    };
    let title = ws.case.title.clone();
    let state = ws.case.workflow.state.label();
    let file_no = ws
        .case
        .assignment
        .file_number
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let objects = ws.vault.object_count();
    let locked = ws.vault.is_locked();
    let case_id = ws.case.id.to_string();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&title).heading());
        ui.label(format!("| Durum: {state} | Dosya No: {file_no} | Kasa nesnesi: {objects}"));
    });
    ui.separator();

    ui.label(egui::RichText::new("Vaka işlemleri").strong());
    ui.horizontal(|ui| {
        ui.label("Materyal yolu:");
        ui.add(
            egui::TextEdit::singleline(&mut app.material_path)
                .hint_text("/materyal/klasoru")
                .desired_width(320.0),
        );
        if ui.button("Materyalleri al ve işle").clicked() {
            app.run_workflow();
        }
    });

    ui.horizontal(|ui| {
        ui.label("Kasa parolası:");
        ui.add(
            egui::TextEdit::singleline(&mut app.vault_password)
                .password(true)
                .desired_width(220.0),
        );
        if locked {
            if ui.button("Kasayı aç").clicked() {
                app.unlock_vault();
            }
        } else {
            ui.label("Kasa açık (şifreleme etkin).");
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Rapor ve UDF üret").clicked() {
            app.generate_report();
        }
        if ui.button("İşlem geçmişini doğrula").clicked() {
            app.verify_audit();
        }
        if ui.button("Yedek al ve geri yükleme testi yap").clicked() {
            app.create_backup();
        }
        if ui.button("Arşivle").clicked() {
            app.archive_case();
        }
    });

    if let Some(s) = &app.last_workflow_summary {
        ui.separator();
        ui.label(format!("Son iş akışı: {s}"));
    }
    if let Some(d) = &app.last_report_dir {
        ui.label(format!("Son rapor dizini: {}", d.display()));
    }

    ui.separator();
    ui.label(format!("Vaka kimliği: {case_id}"));
}

fn evidence(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let Some(ws) = &app.workspace else {
        ui.weak("Açık vaka yok.");
        return;
    };
    ui.label("Kasadaki delil nesneleri. Her nesne SHA-256 ile bütünlüğe bağlıdır.");
    ui.separator();
    egui::Grid::new("delil_listesi").striped(true).show(ui, |ui| {
        ui.label("Yol");
        ui.label("Özgün ad");
        ui.label("Boyut");
        ui.label("Şifreli");
        ui.label("SHA-256 (kısa)");
        ui.end_row();
        for o in &ws.vault.manifest().objects {
            if !app.matches_search(&o.relative_path) {
                continue;
            }
            ui.label(&o.relative_path);
            ui.label(&o.original_name);
            ui.label(format!("{} bayt", o.size_bytes));
            ui.label(if o.encrypted { "evet" } else { "hayır" });
            let sha = o
                .hashes
                .iter()
                .find(|h| h.algorithm == birkat_core::evidence::HashAlgorithm::Sha256)
                .map(|h| h.value.chars().take(16).collect::<String>())
                .unwrap_or_default();
            ui.label(sha);
            ui.end_row();
        }
    });
}

fn device(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let _ = app;
    ui.label(
        "Aygıt ve edinim yönetimi: USB/SD/harici aygıt tanımlama, yazma koruması \
         sorgulama ve edinim planlama.",
    );
    ui.separator();
    ui.label(egui::RichText::new("Aygıt tanımlama alanları").strong());
    ui.label("Üretici, model, seri numarası, kapasite, disk numarası, bölümler,");
    ui.label("dosya sistemi, bağlantı türü, sürücü harfi, bağlanma/ayrılma zamanı");
    ui.label("ve yazma koruması durumu kaydedilir.");
    ui.separator();
    ui.label(egui::RichText::new("Yazma koruması sınıflandırması").strong());
    for s in [
        "Donanımsal yazma engelleyici",
        "Aygıtın kendi salt okunur anahtarı",
        "Windows yazılımsal salt okunur",
        "Koruma yok",
        "Durum doğrulanamadı",
    ] {
        ui.label(format!("• {s}"));
    }
    ui.separator();
    ui.label(
        "Not: Yazılımsal koruma donanımsal korumaya eşdeğer gösterilmez. \
         İşletim sistemi aygıtı uygulama algılamadan önce bağlayabileceği için, \
         sonradan yapılan koruma doğrulaması ilk bağlantıdan itibaren hiç yazma \
         yapılmadığının kanıtı olarak sunulamaz.",
    );
}

fn audio_inventory(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let Some(ws) = &app.workspace else {
        ui.weak("Açık vaka yok.");
        return;
    };
    ui.label("Ses dosyalarının teknik envanteri. Kaynakta olmayan metadata üretilmez.");
    ui.separator();

    let mut shown = 0;
    for o in &ws.vault.manifest().objects {
        if !o.relative_path.contains("transkriptler/") && !o.relative_path.ends_with(".json") {
            // Yalnızca ses olabilecek nesneleri göster.
        }
        if !app.matches_search(&o.relative_path) {
            continue;
        }
        if o.relative_path.starts_with("rapor/") {
            continue;
        }
        shown += 1;
        ui.label(format!(
            "{} — {} bayt, şifreli: {}",
            o.relative_path, o.size_bytes, o.encrypted
        ));
    }
    if shown == 0 {
        ui.weak("Envanterde ses dosyası yok. Dosya çalışma alanından materyal alın.");
    }
}

fn transcription(app: &mut BirkatApp, ui: &mut egui::Ui) {
    ui.label(
        "Transkripsiyon işlem merkezi. Bilirkişiden model, sıcaklık, beam size, \
         batch size, GPU veya CUDA parametreleri sorulmaz.",
    );
    ui.separator();
    ui.label(egui::RichText::new("Otomatik iş kuyruğu").strong());
    for s in [
        "Dosya bazlı görev",
        "İşlem önceliği",
        "Duraklatma / devam / iptal",
        "Hata sonrası yeniden deneme",
        "Kesinti sonrası devam (kontrol noktası)",
        "İlerleme yüzdesi",
        "Tahmini kalan süre (yalnızca ölçümlere dayalıysa)",
    ] {
        ui.label(format!("• {s}"));
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Materyal yolu:");
        ui.add(
            egui::TextEdit::singleline(&mut app.material_path)
                .hint_text("/materyal/klasoru")
                .desired_width(320.0),
        );
        if ui.button("Kuyruğu başlat").clicked() {
            app.run_workflow();
        }
    });
    if let Some(s) = &app.last_workflow_summary {
        ui.label(format!("Son sonuç: {s}"));
    }
}

fn audio_review(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let _ = app;
    ui.label(
        "Ses ve transkriptin eşzamanlı incelendiği çift panelli ekran. \
         Metin satırına tıklanınca ses ilgili bölüme gider; ses oynarken ilgili \
         satır vurgulanır.",
    );
    ui.separator();
    ui.columns(2, |cols| {
        cols[0].label(egui::RichText::new("Ses").strong());
        for s in [
            "Oynat/duraklat",
            "Başlat ve bitir",
            "Zaman koduna git",
            "İleri/geri sar",
            "Hızlı/yavaş oynatma",
            "Döngüsel oynatma",
            "Seçili bölümü tekrar dinleme",
            "Kanal seçimi (sol/sağ ayrı dinleme)",
            "Dalga biçimi",
            "Spektrogram",
            "Zaman çizelgesi",
            "İşaretleme ve not ekleme",
        ] {
            cols[0].label(format!("• {s}"));
        }
        cols[1].label(egui::RichText::new("Transkript").strong());
        for s in [
            "Kelime/cümle düzeltme",
            "Zaman kodu düzenleme",
            "Konuşmacı etiketini düzenleme",
            "Belirsiz bölüm belirleme",
            "Not ekleme",
            "Metin arama ve bul/değiştir",
            "Geri al/yinele",
            "Klavye kısayolları",
            "Sürümleri karşılaştırma",
        ] {
            cols[1].label(format!("• {s}"));
        }
    });
    ui.separator();
    ui.label(birkat_audio::waveform::interpretation_note());
}

fn questions(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let Some(ws) = &app.workspace else {
        ui.weak("Açık vaka yok.");
        return;
    };
    ui.label("Görevlendirme soruları orijinal sırayla korunur; her yanıt dayanaklarla bağlanır.");
    ui.separator();
    let mut qs = ws.case.assignment.questions.clone();
    qs.sort_by_key(|q| q.ordinal);
    if qs.is_empty() {
        ui.weak("Görevlendirmede soru belirtilmedi.");
    }
    for q in &qs {
        ui.label(egui::RichText::new(format!("{}. {}", q.ordinal, q.original_text)).strong());
        match &q.final_answer {
            Some(a) => ui.label(format!("Yanıt: {a}")),
            None => ui.label("Yanıt: (bekliyor — mesleki değerlendirme gerekir)"),
        };
        if q.related_segments.is_empty() && q.related_findings.is_empty() {
            ui.weak("Teknik dayanak bağlanmadı.");
        }
        ui.separator();
    }
}

fn findings(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let _ = app;
    ui.label(
        "Teknik bulgular ve mesleki notlar. Bulgular bilirkişi onayı olmadan \
         makine çıktısı olarak işaretlenir.",
    );
    ui.separator();
    ui.label("Bulgu türleri: Teknik bulgu, mesleki değerlendirme, yöntem sınırlılığı, uyarı, istatistik.");
    ui.separator();
    ui.label(
        "Yapay zekâ, bilirkişinin mesleki kanaatini kendiliğinden oluşturmaz. \
         Makine çıktısı, incelemeye hazır içerik ve bilirkişi onaylı içerik \
         birbirine karıştırılmaz.",
    );
}

fn report_editor(app: &mut BirkatApp, ui: &mut egui::Ui) {
    ui.label(
        "Rapor üretimi tek bir kanonik veri modelinden yapılır. UDF, PDF ve DOCX \
         aynı onaylanmış rapor verisinden üretilir.",
    );
    ui.separator();
    if ui.button("Raporu ve UDF'yi üret").clicked() {
        app.generate_report();
    }
    if let Some(d) = &app.last_report_dir {
        ui.label(format!("Çıktı dizini: {}", d.display()));
        ui.label("Dosyalar: BILIRKISI_RAPORU.udf (birincil), .pdf, .docx, .txt, .json");
    } else {
        ui.weak("Henüz rapor üretilmedi.");
    }
    ui.separator();
    ui.label(egui::RichText::new("Zorunlu rapor alanları").strong());
    for (label, _) in birkat_report::validate::required_fields() {
        ui.label(format!("• {label}"));
    }
}

fn udf_preview(app: &mut BirkatApp, ui: &mut egui::Ui) {
    ui.label(
        "Uygulamanın kendi ürettiği UDF'nin sayfa görünümü. Bu önizleme, gerçek \
         UYAP Editör görünümünü doğrulamanın yerine geçmez; bir ön kontrol \
         katmanıdır.",
    );
    ui.separator();
    let Some(dir) = &app.last_report_dir else {
        ui.weak("Önizleme için önce rapor üretin.");
        return;
    };
    let path = dir.join("BILIRKISI_RAPORU.udf");
    if !path.exists() {
        ui.weak("UDF dosyası bulunamadı.");
        return;
    }
    match birkat_udf::read_udf(&path) {
        Ok(r) => {
            let preview = birkat_report::preview::build_preview(&r.document);
            ui.label(format!(
                "{} sayfa, metin havuzu {} karakter, format_id {}",
                preview.page_count(),
                r.pool.chars().count(),
                r.format_id.as_deref().unwrap_or("-")
            ));
            if !preview.warnings.is_empty() {
                for w in &preview.warnings {
                    ui.label(format!("! {w}"));
                }
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for page in &preview.pages {
                    ui.label(egui::RichText::new(format!("— Sayfa {} —", page.number)).strong());
                    for line in &page.lines {
                        let mut text = egui::RichText::new(&line.text);
                        if line.is_heading {
                            text = text.strong();
                        }
                        if line.is_table_row {
                            text = text.monospace();
                        }
                        ui.label(text);
                    }
                    ui.add_space(8.0);
                }
            });
        }
        Err(e) => {
            ui.label(format!("UDF okunamadı: {e}"));
        }
    }
}

fn signature(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let _ = app;
    ui.label(
        "İmza hazırlığı ve UYAP teslim hazırlığı. Resmî e-imza altyapısı ve UYAP \
         Portal güvenliği aşılmaya çalışılmaz.",
    );
    ui.separator();
    ui.label(egui::RichText::new("İmza hazırlığı").strong());
    for s in [
        "Son UDF raporu hazırlanır",
        "İmza öncesi dosya hash değeri kaydedilir",
        "İmzalanacak dosyalar listelenir",
        "UDF resmî UYAP Editör'de açılabilir",
        "Kullanıcı resmî imza sürecine yönlendirilir",
        "İmzalı dosya yeniden vaka arşivine alınır",
        "İmza sonrası dosyanın değişip değişmediği kontrol edilir",
    ] {
        ui.label(format!("• {s}"));
    }
    ui.separator();
    ui.label(egui::RichText::new("Teslim kontrol listesi").strong());
    for item in birkat_report::auxiliary::default_delivery_checklist() {
        ui.label(format!("[ ] {item}"));
    }
    ui.separator();
    ui.label(
        "Uygulama kendi ekranındaki bir onay kutusunu gerçek e-imza gibi sunmaz; \
         PIN, kart parolası veya gizli anahtar saklamaz. UYAP Portal üzerinde \
         nihai yükleme ve imza işlemi kullanıcının yetkisiyle yapılır.",
    );
}

fn auxiliary(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let _ = app;
    ui.label("Ek rapor ve yardımcı mesleki belgeler; doğrudan UDF olarak üretilebilir.");
    ui.separator();
    for k in [
        birkat_report::auxiliary::AuxiliaryDocumentKind::ExtensionRequest,
        birkat_report::auxiliary::AuxiliaryDocumentKind::MissingMaterialRequest,
        birkat_report::auxiliary::AuxiliaryDocumentKind::AdditionalRecordsRequest,
        birkat_report::auxiliary::AuxiliaryDocumentKind::ScopeClarificationRequest,
        birkat_report::auxiliary::AuxiliaryDocumentKind::WithdrawalNotice,
        birkat_report::auxiliary::AuxiliaryDocumentKind::MaterialHandoverForm,
        birkat_report::auxiliary::AuxiliaryDocumentKind::MaterialReturnForm,
        birkat_report::auxiliary::AuxiliaryDocumentKind::EvidenceSchedule,
        birkat_report::auxiliary::AuxiliaryDocumentKind::ReportAttachmentList,
        birkat_report::auxiliary::AuxiliaryDocumentKind::DeliveryChecklist,
    ] {
        ui.label(format!("• {} ({})", k.title(), k.suggested_file_name()));
    }
}

fn calendar(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let Some(ws) = &app.workspace else {
        ui.weak("Açık vaka yok.");
        return;
    };
    ui.label("Takvim, ücret ve gider takibi.");
    ui.separator();
    ui.label(egui::RichText::new("Süreler").strong());
    if ws.case.deadlines.is_empty() {
        ui.weak("Kayıtlı süre yok.");
    }
    for d in &ws.case.deadlines {
        ui.label(format!(
            "{} — {} ({})",
            d.kind.label(),
            d.due_at.to_rfc3339(),
            if d.completed { "tamamlandı" } else { "bekliyor" }
        ));
    }
    ui.separator();
    ui.label(egui::RichText::new("Ücret ve giderler").strong());
    ui.label("Portalda görülen tutarlar resmî veri gibi gösterilmez; kullanıcı tarafından girilen tutar ile resmî portal bilgisi ayrı kaynak türleri olarak saklanır.");
    for s in [
        "Dosya bazında ücret ve belirlenme tarihi",
        "Tahakkuk ve ödeme bilgisi",
        "Yol/konaklama ve diğer giderler",
        "Belge/fatura ekleri",
        "Dosya bazında çalışma süresi",
        "Gelir-gider özeti",
    ] {
        ui.label(format!("• {s}"));
    }
}

fn archive(app: &mut BirkatApp, ui: &mut egui::Ui) {
    ui.label("Vaka arşivi. Arşivlenen dosyaların tüm belgeleri, sesleri, transkriptleri, raporları ve işlem geçmişi korunur.");
    ui.separator();
    let archived: Vec<_> = app
        .cases()
        .into_iter()
        .filter(|c| c.archived)
        .collect();
    if archived.is_empty() {
        ui.weak("Arşivlenmiş vaka yok.");
    }
    for c in archived {
        ui.label(format!("{} — {}", c.title, c.workflow.state.label()));
    }
    ui.separator();
    if ui.button("Açık vakayı arşivle").clicked() {
        app.archive_case();
    }
}

fn backup(app: &mut BirkatApp, ui: &mut egui::Ui) {
    ui.label("Şifreli yedekleme ve geri yükleme. Yedek şifrelidir, hash manifesti içerir ve geri yüklenebilir olmalıdır.");
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Parola:");
        ui.add(
            egui::TextEdit::singleline(&mut app.vault_password)
                .password(true)
                .desired_width(240.0),
        );
        if ui.button("Yedek al ve geri yükleme testi yap").clicked() {
            app.create_backup();
        }
    });
    ui.separator();
    ui.label(
        "Yalnızca yedek oluşturulması yeterli kabul edilmez. Başka bir konuma \
         gerçek geri yükleme ve bütünlük testi yapılmalıdır.",
    );
    let yedekler = app.data_root.join("yedekler");
    if yedekler.exists() {
        ui.separator();
        ui.label(egui::RichText::new("Mevcut yedekler").strong());
        if let Ok(rd) = std::fs::read_dir(&yedekler) {
            for e in rd.filter_map(|e| e.ok()) {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                ui.label(format!("{} — {} bayt", e.file_name().to_string_lossy(), size));
            }
        }
    }
}

fn update(app: &mut BirkatApp, ui: &mut egui::Ui) {
    let _ = app;
    ui.label("İmzalı güncelleme ve geri dönüş.");
    ui.separator();
    for s in [
        "İmzalı manifest",
        "Paket bütünlüğü doğrulaması",
        "Dijital imza doğrulaması (imzasız paket uygulanmaz)",
        "Ön yedek",
        "Güvenli kurulum",
        "Veritabanı migrasyonu",
        "Geri dönüş (rollback)",
        "Sürüm geçmişi",
        "Model sürüm takibi",
    ] {
        ui.label(format!("• {s}"));
    }
    ui.separator();
    ui.label("Güncelleme aktif transkripsiyon veya delil edinimi sırasında uygulanmaz.");
    ui.separator();
    ui.label(birkat_updater::network_note());
}

fn health(app: &mut BirkatApp, ui: &mut egui::Ui) {
    if ui.button("Sağlığı yeniden denetle").clicked() {
        app.refresh_health();
    }
    ui.separator();
    let Some(h) = &app.health else {
        ui.weak("Sağlık verisi yok.");
        return;
    };
    ui.label(h.summary());
    ui.separator();
    egui::Grid::new("saglik").striped(true).show(ui, |ui| {
        let rows = [
            ("Veritabanı", h.database_ok),
            ("Kasa", h.vault_ok),
            ("Model paketi", h.model_ok),
            ("Yedek dizini", h.backup_dir_ok),
        ];
        for (label, ok) in rows {
            ui.label(label);
            ui.label(if ok { "tamam" } else { "eksik" });
            ui.end_row();
        }
        ui.label("CPU çekirdek");
        ui.label(h.cpu_cores.to_string());
        ui.end_row();
        ui.label("GPU derleme desteği");
        ui.label(if h.gpu_support_compiled {
            "var"
        } else {
            "yok (CPU kullanılır)"
        });
        ui.end_row();
    });
    if !h.warnings.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("Uyarılar").strong());
        for w in &h.warnings {
            ui.label(format!("! {w}"));
        }
    }
    ui.separator();
    ui.label(egui::RichText::new("Veri kök dizini").strong());
    ui.label(app.data_root.display().to_string());
    ui.separator();
    ui.label(egui::RichText::new("Ayarlar").strong());
    egui::Grid::new("ayarlar").show(ui, |ui| {
        ui.label("Tema");
        egui::ComboBox::from_id_salt("tema")
            .selected_text(app.settings.theme.label())
            .show_ui(ui, |ui| {
                for t in [
                    birkat_app::settings::Theme::Light,
                    birkat_app::settings::Theme::Dark,
                    birkat_app::settings::Theme::System,
                ] {
                    ui.selectable_value(&mut app.settings.theme, t, t.label());
                }
            });
        ui.end_row();
        ui.label("Arayüz ölçeği");
        ui.add(egui::Slider::new(&mut app.settings.ui_scale, 0.5..=3.0));
        ui.end_row();
        ui.label("Otomatik kilitleme (dakika)");
        ui.add(egui::DragValue::new(&mut app.settings.auto_lock_minutes).range(0..=120));
        ui.end_row();
        ui.label("Otomatik yedekleme");
        ui.checkbox(&mut app.settings.auto_backup, "açık");
        ui.end_row();
    });
    let mut s = app.settings.clone();
    let warnings = s.validate();
    for w in warnings {
        ui.label(format!("! {w}"));
    }
    if ui.button("Ayarları doğrula ve uygula").clicked() {
        app.settings = s;
        app.notice(Notice::Success("Ayarlar uygulandı.".to_string()));
    }
}
