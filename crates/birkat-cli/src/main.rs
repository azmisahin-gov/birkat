//! # BİRKÂT komut satırı arayüzü
//!
//! Bu ikili, ürünün tüm ana işlevlerine komut satırından erişim sağlar.
//! Masaüstü arayüzü (birkat-desktop) ile aynı servis katmanını kullanır.
//!
//! ## Kullanım
//!
//! ```text
//! birkat profil-ekle --ad "Ayşe Yılmaz" --sicil 12345
//! birkat vaka-olustur --baslik "2025/123" --mahkeme "Ankara 5. ACCM" --dosya-no "2025/123 Esas"
//! birkat materyal-ekle --vaka <ID> --yol /materyal
//! birkat transkript --vaka <ID> --yol /materyal
//! birkat rapor --vaka <ID> --cikti /tmp
//! birkat udf-dogrula --dosya rapor.udf
//! birkat yedek --vaka <ID> --parola "..." --cikti yedek.bin
//! birkat geri-yukle --yedek yedek.bin --parola "..." --hedef /tmp/geri
//! birkat saglik
//! birkat panel
//! ```

use anyhow::{Context, Result};
use birkat_app::store::CaseStore;
use birkat_app::workspace::{new_case_from_assignment, CaseWorkspace};
use birkat_core::case::CourtOrAuthority;
use birkat_core::ids::CourtId;
use birkat_core::profile::ExpertProfile;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// BİRKÂT — 56.12 Bilirkişi Dijital Çalışma Operatörü
#[derive(Parser)]
#[command(name = "birkat", version, about, long_about = None)]
struct Cli {
    /// Veri kök dizini.
    #[arg(long, global = true, default_value = "./birkat-veri")]
    veri: PathBuf,

    #[command(subcommand)]
    komut: Komut,
}

#[derive(Subcommand)]
enum Komut {
    /// Bilirkişi profili ekler.
    ProfilEkle {
        /// Ad ve soyad.
        #[arg(long)]
        ad: String,
        /// Unvan.
        #[arg(long)]
        unvan: Option<String>,
        /// Bilirkişi sicil numarası.
        #[arg(long)]
        sicil: Option<String>,
        /// Uzmanlık alanı.
        #[arg(long, default_value = "56.12")]
        uzmanlik: String,
    },

    /// Profilleri listeler.
    Profiller,

    /// Yeni vaka oluşturur.
    VakaOlustur {
        /// Vaka başlığı.
        #[arg(long)]
        baslik: String,
        /// Görevlendiren merci.
        #[arg(long)]
        mahkeme: String,
        /// Dosya esas numarası.
        #[arg(long)]
        dosya_no: Option<String>,
        /// İnceleme konusu.
        #[arg(long)]
        konu: Option<String>,
    },

    /// Vakaları listeler.
    Vakalar,

    /// Vakaya materyal ekler (kasaya alır).
    MateryalEkle {
        /// Vaka kimliği.
        #[arg(long)]
        vaka: String,
        /// Materyal yolu (dosya veya klasör).
        #[arg(long)]
        yol: PathBuf,
        /// Kasa parolası (şifreleme için).
        #[arg(long)]
        parola: Option<String>,
    },

    /// Materyalleri envantere alır ve transkripsiyonu çalıştırır.
    Transkript {
        /// Vaka kimliği.
        #[arg(long)]
        vaka: String,
        /// Materyal yolu.
        #[arg(long)]
        yol: PathBuf,
    },

    /// Vaka için rapor ve UDF üretir.
    Rapor {
        /// Vaka kimliği.
        #[arg(long)]
        vaka: String,
        /// Çıktı dizini.
        #[arg(long)]
        cikti: PathBuf,
    },

    /// UDF dosyasını doğrular.
    UdfDogrula {
        /// UDF dosyası.
        #[arg(long)]
        dosya: PathBuf,
    },

    /// UDF dosyasını okur ve metni yazdırır.
    UdfOku {
        /// UDF dosyası.
        #[arg(long)]
        dosya: PathBuf,
    },

    /// Ses dosyasını inceler (teknik envanter).
    SesIncele {
        /// Ses dosyası.
        #[arg(long)]
        dosya: PathBuf,
    },

    /// Vakayı şifreli yedeğe alır.
    Yedek {
        /// Vaka kimliği.
        #[arg(long)]
        vaka: String,
        /// Parola.
        #[arg(long)]
        parola: String,
        /// Yedek dosyası.
        #[arg(long)]
        cikti: PathBuf,
    },

    /// Yedeği geri yükler ve bütünlüğü doğrular.
    GeriYukle {
        /// Yedek dosyası.
        #[arg(long)]
        yedek: PathBuf,
        /// Parola.
        #[arg(long)]
        parola: String,
        /// Hedef dizin.
        #[arg(long)]
        hedef: PathBuf,
    },

    /// İşlem geçmişi (audit) zincirini doğrular.
    DenetimDogrula,

    /// Sistem sağlığını denetler.
    Saglik,

    /// Ana kontrol panelini gösterir.
    Panel,

    /// Sürüm ve derleme bilgilerini gösterir.
    Surum,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Günlükçüyü başlat (Türkçe düzey etiketleriyle).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    std::fs::create_dir_all(&cli.veri).context("Veri kök dizini oluşturulamadı")?;
    let db_path = cli.veri.join("birkat.db");
    let mut store = CaseStore::open(&db_path)?;

    match cli.komut {
        Komut::ProfilEkle {
            ad,
            unvan,
            sicil,
            uzmanlik,
        } => {
            let mut p = ExpertProfile::new(ad);
            p.title = unvan;
            p.registry_number = sicil;
            p.specialty = uzmanlik;
            store.save_profile(&p)?;
            println!("Profil eklendi: {} (kimlik: {})", p.full_name, p.id);
            if !p.is_report_ready() {
                println!(
                    "Not: Sicil numarası girilmediği için profil rapor üretimine hazır değil."
                );
            }
        }

        Komut::Profiller => {
            let list = store.list_profiles()?;
            if list.is_empty() {
                println!("Kayıtlı profil yok.");
            } else {
                println!(
                    "{:<38} {:<24} {:<12} Uzmanlık",
                    "Kimlik", "Ad Soyad", "Sicil No"
                );
                for p in list {
                    println!(
                        "{:<38} {:<24} {:<12} {}",
                        p.id,
                        p.full_name,
                        p.registry_number.as_deref().unwrap_or("-"),
                        p.specialty
                    );
                }
            }
        }

        Komut::VakaOlustur {
            baslik,
            mahkeme,
            dosya_no,
            konu,
        } => {
            let court = CourtOrAuthority {
                id: CourtId::new(),
                name: mahkeme,
                unit_kind: "Mahkeme".to_string(),
                contact: None,
                source_ref: None,
            };
            let case = new_case_from_assignment(baslik, court, dosya_no, konu);
            let id = case.id.clone();
            let ws = CaseWorkspace::create(&cli.veri, case, &mut store)?;
            println!("Vaka oluşturuldu: {}", ws.case.title);
            println!("Vaka kimliği: {id}");
            println!("Durum: {}", ws.case.workflow.state.label());
        }

        Komut::Vakalar => {
            let cases = store.list_cases()?;
            if cases.is_empty() {
                println!("Kayıtlı vaka yok.");
            } else {
                println!("{:<38} {:<28} {:<22} Durum", "Kimlik", "Başlık", "Dosya No");
                for c in cases {
                    println!(
                        "{:<38} {:<28} {:<22} {}",
                        c.id,
                        c.title,
                        c.assignment.file_number.as_deref().unwrap_or("-"),
                        c.workflow.state.label()
                    );
                }
            }
        }

        Komut::MateryalEkle { vaka, yol, parola } => {
            let case_id = vaka.parse::<birkat_core::ids::CaseId>()?;
            let mut ws = CaseWorkspace::open(&cli.veri, &case_id, &mut store)?;
            if let Some(p) = parola {
                ws.unlock_vault(&p)?;
                println!("Kasa şifrelemesi etkin.");
            }
            if yol.is_dir() {
                let mut count = 0;
                for e in walkdir::WalkDir::new(&yol)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if !e.file_type().is_file() {
                        continue;
                    }
                    let rel = e
                        .path()
                        .strip_prefix(&yol)
                        .unwrap_or(e.path())
                        .to_string_lossy()
                        .replace('\\', "/");
                    ws.vault
                        .store_file(e.path(), &rel, &yol.display().to_string())?;
                    count += 1;
                }
                println!("{count} dosya kasaya alındı.");
            } else {
                let rel = yol
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                ws.vault
                    .store_file(&yol, &rel, &yol.display().to_string())?;
                println!("Dosya kasaya alındı: {rel}");
            }
            ws.commit(
                &mut store,
                birkat_audit::AuditAction::MaterialAdded,
                "Materyal kasaya alındı",
                None,
                serde_json::json!({ "yol": yol.display().to_string() }),
            )?;
            println!("Kasadaki nesne sayısı: {}", ws.vault.object_count());
        }

        Komut::Transkript { vaka, yol } => {
            let case_id = vaka.parse::<birkat_core::ids::CaseId>()?;
            let mut ws = CaseWorkspace::open(&cli.veri, &case_id, &mut store)?;
            let secim = birkat_transcription::select_engine(&cli.veri, false);
            println!("{}", secim.notice);
            println!("Kullanılan motor: {}", secim.kind.label());
            println!("Otomatik iş akışı başlatılıyor...");
            let out = birkat_app::run_automatic_workflow(
                &mut ws,
                &mut store,
                &[yol],
                secim.engine.as_ref(),
                &cli.veri,
            )?;
            println!("{}", out.summary());
            println!("\nYürütülen adımlar:");
            for s in &out.steps {
                println!("  - {s}");
            }
            if out.has_exceptions() {
                println!("\nİncelenmesi gereken istisnalar:");
                for w in &out.warnings {
                    println!("  ! {w}");
                }
            }
            if let Some(udf) = &out.udf {
                let bytes = birkat_udf::write_udf_to_bytes(udf.clone())?;
                let v = birkat_udf::validate_udf(&bytes);
                println!("\nUDF taslağı doğrulaması: {}", v.summary());
            }
            println!("\nSon kontrol listesi:");
            for c in &out.checklist {
                println!("  [ ] {c}");
            }
        }

        Komut::Rapor { vaka, cikti } => {
            let case_id = vaka.parse::<birkat_core::ids::CaseId>()?;
            let mut ws = CaseWorkspace::open(&cli.veri, &case_id, &mut store)?;
            std::fs::create_dir_all(&cikti)?;
            let experts = store.list_profiles()?;

            // Otomatik iş akışının ürettiği rapor girdisini kullan (varsa).
            let input = ws
                .vault
                .read_object("rapor/rapor-girdisi.json")
                .ok()
                .and_then(|d| {
                    serde_json::from_slice::<birkat_report::builder::ReportBuildInput>(&d).ok()
                })
                .unwrap_or_default();
            let report = birkat_report::build_report(
                &ws.case,
                &experts,
                &input,
                birkat_core::report::ReportKind::Main,
                1,
            );

            // UDF (birincil çıktı)
            let udf = birkat_report::report_to_udf(
                &report,
                &birkat_report::ReportToUdfOptions::default(),
            );
            let udf_bytes = birkat_udf::write_udf_to_bytes(udf)?;
            let udf_path = cikti.join("BILIRKISI_RAPORU.udf");
            std::fs::write(&udf_path, &udf_bytes)?;

            let v = birkat_udf::validate_udf(&udf_bytes);
            println!("UDF yazıldı: {}", udf_path.display());
            println!("Doğrulama: {}", v.summary());
            if !v.ok {
                for e in v.errors() {
                    eprintln!("  HATA: {}", e.message);
                }
                anyhow::bail!("UDF doğrulaması başarısız; dosya hazır olarak sunulmaz.");
            }

            // Yardımcı çıktılar
            let pdf_bytes = birkat_report::pdf::report_to_pdf(&report)
                .map_err(|e| anyhow::anyhow!("PDF üretilemedi: {e}"))?;
            std::fs::write(cikti.join("BILIRKISI_RAPORU.pdf"), &pdf_bytes)?;

            let docx = birkat_report::docx::report_to_docx(&report)
                .map_err(|e| anyhow::anyhow!("DOCX üretilemedi: {e}"))?;
            std::fs::write(cikti.join("BILIRKISI_RAPORU.docx"), &docx)?;

            std::fs::write(
                cikti.join("BILIRKISI_RAPORU.txt"),
                birkat_report::text::to_text(&report),
            )?;
            std::fs::write(
                cikti.join("BILIRKISI_RAPORU.json"),
                birkat_report::text::to_json(&report),
            )?;

            ws.commit(
                &mut store,
                birkat_audit::AuditAction::ReportGenerated,
                "Rapor ve UDF üretildi",
                Some(udf_path.display().to_string()),
                serde_json::json!({ "dogrulama": v.ok }),
            )?;

            println!("Yardımcı çıktılar: PDF, DOCX, TXT, JSON");
            println!("Not: {}", birkat_report::text::auxiliary_note());
        }

        Komut::UdfDogrula { dosya } => {
            let bytes = std::fs::read(&dosya)?;
            let v = birkat_udf::validate_udf(&bytes);
            println!("{}", v.summary());
            for c in &v.checks {
                let mark = match c.severity {
                    birkat_udf::validator::CheckSeverity::Info => "OK ",
                    birkat_udf::validator::CheckSeverity::Warning => "UYARI",
                    birkat_udf::validator::CheckSeverity::Error => "HATA",
                };
                println!("  [{mark}] {}: {}", c.name, c.message);
            }
            println!(
                "Metin havuzu: {} karakter, Türkçe karakter: {}",
                v.pool_chars, v.has_turkish_characters
            );
            if !v.ok {
                anyhow::bail!("UDF doğrulaması başarısız.");
            }
        }

        Komut::UdfOku { dosya } => {
            let r = birkat_udf::read_udf(&dosya)?;
            println!("format_id: {}", r.format_id.as_deref().unwrap_or("-"));
            println!("Metin havuzu ({} karakter):", r.pool.chars().count());
            println!("{}", r.pool);
            if !r.warnings.is_empty() {
                println!("\nUyarılar:");
                for w in &r.warnings {
                    println!("  ! {w}");
                }
            }
        }

        Komut::SesIncele { dosya } => {
            let probe = birkat_audio::probe_file(&dosya)?;
            println!("Kapsayıcı      : {}", probe.metadata.container);
            println!("Codec          : {}", probe.metadata.codec);
            println!(
                "Süre           : {}",
                probe
                    .metadata
                    .duration_secs
                    .map(|d| format!("{d:.3} sn"))
                    .unwrap_or_else(|| "(bilinmiyor)".to_string())
            );
            println!(
                "Örnekleme      : {}",
                probe
                    .metadata
                    .sample_rate
                    .map(|s| format!("{s} Hz"))
                    .unwrap_or_else(|| "(bilinmiyor)".to_string())
            );
            println!(
                "Kanal          : {}",
                probe
                    .metadata
                    .channels
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "(bilinmiyor)".to_string())
            );
            println!(
                "Bit derinliği  : {}",
                probe
                    .metadata
                    .bit_depth
                    .map(|b| format!("{b} bit"))
                    .unwrap_or_else(|| "(bilinmiyor)".to_string())
            );
            let unknown = probe.metadata.unknown_fields();
            if !unknown.is_empty() {
                println!("Kaynakta bulunmayan alanlar: {}", unknown.join(", "));
            }
        }

        Komut::Yedek {
            vaka,
            parola,
            cikti,
        } => {
            let case_id = vaka.parse::<birkat_core::ids::CaseId>()?;
            let case = store.load_case(&case_id)?;
            let kasa = cli.veri.join("kasalar").join(case_id.as_str());
            let result = birkat_backup::create_backup(
                &kasa,
                &cikti,
                &birkat_backup::BackupOptions {
                    password: parola.clone(),
                    ..Default::default()
                },
                vec![case_id.to_string()],
                None,
                Some(format!(
                    "{} vaka olayı",
                    store.load_audit_chain()?.events.len()
                )),
            )?;
            println!("Yedek oluşturuldu: {}", result.path.display());
            println!(
                "Dosya sayısı: {}, toplam: {} bayt, boyut: {} bayt",
                result.manifest.file_count(),
                result.manifest.total_bytes,
                result.size_bytes
            );
            println!("Manifesto doğrulaması: {}", result.manifest.verify());
            let _ = case;

            // Gerçek geri yükleme testi (19.4).
            let test = birkat_backup::verify_restore(&cikti, &parola, &cli.veri)?;
            println!("Geri yükleme testi: {}", test.summary());
        }

        Komut::GeriYukle {
            yedek,
            parola,
            hedef,
        } => {
            let r = birkat_backup::restore_backup(&yedek, &hedef, &parola)?;
            println!("{}", r.summary());
            if !r.ok() {
                for m in &r.hash_mismatches {
                    eprintln!("  Hash uyuşmazlığı: {m}");
                }
                anyhow::bail!("Geri yükleme bütünlük doğrulamasından geçemedi.");
            }
        }

        Komut::DenetimDogrula => {
            let chain = store.load_audit_chain()?;
            let v = chain.verify();
            println!("{}", v.message);
            println!("Doğrulanan olay: {}", v.checked);
            println!(
                "Bağımsız zaman damgası yerine geçer mi: {}",
                if v.is_independent_timestamp {
                    "evet"
                } else {
                    "hayır"
                }
            );
            println!(
                "Not: Yerel hash zinciri, harici güvenilir zaman damgası veya \
                 nitelikli elektronik imza olmadan bağımsız ve değiştirilemez \
                 zaman damgası olarak sunulamaz."
            );
            if !v.valid {
                anyhow::bail!("İşlem geçmişi bütünlüğü bozuk.");
            }
        }

        Komut::Saglik => {
            let h = birkat_app::settings::check_health(
                &cli.veri,
                None,
                Some(&cli.veri.join("yedekler")),
            );
            println!("{}", h.summary());
            println!("Veritabanı : {}", ok_str(h.database_ok));
            println!("Kasa       : {}", ok_str(h.vault_ok));
            println!("Model      : {}", ok_str(h.model_ok));
            println!("Yedek dizini: {}", ok_str(h.backup_dir_ok));
            println!("CPU çekirdek: {}", h.cpu_cores);
            println!(
                "GPU derleme desteği: {}",
                if h.gpu_support_compiled {
                    "var"
                } else {
                    "yok (CPU kullanılır)"
                }
            );
            if !h.warnings.is_empty() {
                println!("\nUyarılar:");
                for w in &h.warnings {
                    println!("  ! {w}");
                }
            }
        }

        Komut::Panel => {
            let d = birkat_app::build_dashboard(&store, 30)?;
            println!("{}", d.summary());
            println!("\nDevam eden dosyalar ({}):", d.active_cases.len());
            for c in &d.active_cases {
                println!(
                    "  {:<30} {:<22} {}",
                    c.title,
                    c.state,
                    c.due_in.as_deref().unwrap_or("-")
                );
            }
            if !d.pending_exceptions.is_empty() {
                println!(
                    "\nİnceleme bekleyen istisnalar ({}):",
                    d.pending_exceptions.len()
                );
                for (t, m) in &d.pending_exceptions {
                    println!("  [{t}] {m}");
                }
            }
            if !d.awaiting_report_review.is_empty() {
                println!(
                    "\nRapor kontrolü bekleyen dosyalar ({}):",
                    d.awaiting_report_review.len()
                );
                for c in &d.awaiting_report_review {
                    println!("  {:<30} {}", c.title, c.state);
                }
            }
            if !d.awaiting_delivery.is_empty() {
                println!(
                    "\nTeslim bekleyen dosyalar ({}):",
                    d.awaiting_delivery.len()
                );
                for c in &d.awaiting_delivery {
                    println!("  {:<30} {}", c.title, c.state);
                }
            }
            if !d.upcoming_deadlines.is_empty() {
                println!("\nYaklaşan görev süreleri:");
                for (t, l) in &d.upcoming_deadlines {
                    println!("  [{t}] {l}");
                }
            }
            println!("\nArşivlenmiş dosya: {}", d.archived_count);
        }

        Komut::Surum => {
            println!("BİRKÂT {}", birkat_core::APP_VERSION);
            println!(
                "Uzmanlık: {} — {}",
                birkat_core::SPECIALTY_CODE,
                birkat_core::SPECIALTY_NAME
            );
            println!("Veri modeli sürümü: {}", birkat_core::MODEL_VERSION);
            println!("Veri kök dizini: {}", cli.veri.display());
            println!(
                "Whisper desteği: {}",
                if birkat_transcription::runtime::whisper_compiled() {
                    "derlemede etkin"
                } else {
                    "derlemede KAPALI (gerçek transkripsiyon yapılamaz)"
                }
            );
            match birkat_transcription::discover_model_dir(&cli.veri) {
                Some(p) => println!("Model paketi: {} (hazır)", p.root.display()),
                None => println!("Model paketi: BULUNAMADI"),
            }
        }
    }

    Ok(())
}

fn ok_str(v: bool) -> &'static str {
    if v {
        "tamam"
    } else {
        "eksik"
    }
}
