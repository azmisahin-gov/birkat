//! Gerçek Whisper/Candle çıkarımını doğrulayan örnek.
//!
//! Bu ikili, model paketini ve ses dosyalarını yükler, **gerçek model
//! ağırlıklarıyla** çıkarım yapar ve çıktıyı referans metinle karşılaştırır.
//! `cargo test` birim testlerinden farklı olarak gerçek modeli çalıştırır;
//! bu nedenle ayrı bir doğrulama adımı olarak kullanılır.
//!
//! ## Kullanım
//!
//! ```text
//! cargo run -p birkat-transcription --features whisper \
//!   --example whisper-dogrula -- \
//!   --model /yol/model.safetensors \
//!   --config /yol/config.json \
//!   --tokenizer /yol/tokenizer.json \
//!   --mel /yol/melfilters128.bytes \
//!   --ses /yol/kayit1.wav --referans /yol/metin1.txt \
//!   --dil tr
//! ```
//!
//! ## Dürüstlük notu
//!
//! Bu örnek, modelin çıktısını **raporlar**; doğruluk iddiası yalnızca
//! referans metin verildiğinde ve yalnızca o kayıt için geçerlidir. WER
//! sonucu, transkriptin bilirkişi tarafından doğrulandığı anlamına gelmez.

use candle_core::{DType, Device, IndexOp, Tensor, D};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, audio, Config};
use std::path::PathBuf;
use tokenizers::Tokenizer;

struct Args {
    model: PathBuf,
    config: PathBuf,
    tokenizer: PathBuf,
    mel: PathBuf,
    ses: Vec<PathBuf>,
    referans: Vec<PathBuf>,
    dil: Option<String>,
    gpu: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut model = None;
    let mut config = None;
    let mut tokenizer = None;
    let mut mel = None;
    let mut ses = Vec::new();
    let mut referans = Vec::new();
    let mut dil = None;
    let mut gpu = false;

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--model" => model = it.next().map(PathBuf::from),
            "--config" => config = it.next().map(PathBuf::from),
            "--tokenizer" => tokenizer = it.next().map(PathBuf::from),
            "--mel" => mel = it.next().map(PathBuf::from),
            "--ses" => {
                if let Some(v) = it.next() {
                    ses.push(PathBuf::from(v))
                }
            }
            "--referans" => {
                if let Some(v) = it.next() {
                    referans.push(PathBuf::from(v))
                }
            }
            "--dil" => dil = it.next(),
            "--gpu" => gpu = true,
            other => return Err(format!("Bilinmeyen argüman: {other}")),
        }
    }

    Ok(Args {
        model: model.ok_or("--model gerekli")?,
        config: config.ok_or("--config gerekli")?,
        tokenizer: tokenizer.ok_or("--tokenizer gerekli")?,
        mel: mel.ok_or("--mel gerekli")?,
        ses,
        referans,
        dil,
        gpu,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = parse_args().map_err(|e| format!("Argüman hatası: {e}"))?;

    // Cihaz seçimi: GPU istenirse dene, başarısızsa CPU'ya düş.
    let (device, cihaz_etiketi) = if args.gpu {
        #[cfg(feature = "cuda")]
        {
            if let Ok(d) = Device::new_cuda(0) {
                (d, "cuda:0".to_string())
            } else {
                (Device::Cpu, "cpu (CUDA başlatılamadı)".to_string())
            }
        }
        #[cfg(not(feature = "cuda"))]
        {
            (Device::Cpu, "cpu (CUDA derlenmedi)".to_string())
        }
    } else {
        (Device::Cpu, "cpu".to_string())
    };
    println!("Cihaz: {cihaz_etiketi}");

    let config: Config = serde_json::from_str(&std::fs::read_to_string(&args.config)?)?;
    let tokenizer = Tokenizer::from_file(&args.tokenizer)?;
    let mel_bytes = std::fs::read(&args.mel)?;
    let mel_filters: Vec<f32> = mel_bytes
        .chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    println!(
        "Mel filtresi: {} değer (beklenen {})",
        mel_filters.len(),
        (m::N_FFT / 2 + 1) * config.num_mel_bins
    );

    let vb =
        VarBuilder::from_buffered_safetensors(std::fs::read(&args.model)?, DType::F32, &device)?;
    let mut model = m::model::Whisper::load(&vb, config.clone())?;
    println!("Model yüklendi.");

    let sot = tokenizer
        .token_to_id(m::SOT_TOKEN)
        .ok_or("SOT belirteci bulunamadı")?;
    let transcribe = tokenizer
        .token_to_id(m::TRANSCRIBE_TOKEN)
        .ok_or("transcribe belirteci bulunamadı")?;
    let no_ts = tokenizer
        .token_to_id(m::NO_TIMESTAMPS_TOKEN)
        .ok_or("notimestamps belirteci bulunamadı")?;
    let eot = tokenizer
        .token_to_id(m::EOT_TOKEN)
        .ok_or("EOT belirteci bulunamadı")?;

    let mut toplam_sozcuk = 0usize;
    let mut toplam_hatali = 0usize;

    for (i, ses) in args.ses.iter().enumerate() {
        println!("\n=== {} ===", ses.display());
        let pcm = oku_wav(ses)?;
        println!(
            "Ses: {:.2} sn, {} örnek",
            pcm.len() as f64 / 16000.0,
            pcm.len()
        );

        // Whisper kodlayıcısı sabit 30 saniyelik pencere bekler
        // (3000 mel karesi). Kısa kayıtlar sıfırla doldurulur, uzun
        // kayıtlar kırpılır; aksi hâlde model anlamsız çıktı üretir.
        let mel = {
            let mut pencere = vec![0.0f32; m::N_SAMPLES];
            let n = pcm.len().min(m::N_SAMPLES);
            pencere[..n].copy_from_slice(&pcm[..n]);
            let mels = audio::pcm_to_mel(&config, &pencere, &mel_filters);
            let kare = mels.len() / config.num_mel_bins;
            // Candle'ın mel fonksiyonu ek dolgu kareleri ekler; kodlayıcı
            // sabit `N_FRAMES` (3000) kare bekler. Resmî candle örneğiyle
            // aynı biçimde kare eksenine göre kırpılır.
            let t = Tensor::from_vec(mels, (1, config.num_mel_bins, kare), &Device::Cpu)?
                .narrow(2, 0, m::N_FRAMES)?
                .to_device(&device)?;
            // Tanılama: mel istatistikleri (girdi gerçekten farklı mı?)
            let duz: Vec<f32> = t.flatten_all()?.to_vec1()?;
            let mn = duz.iter().cloned().fold(f32::INFINITY, f32::min);
            let mx = duz.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let ort: f32 = duz.iter().sum::<f32>() / duz.len() as f32;
            println!(
                "mel boyutları: {:?} | min {mn:.3} max {mx:.3} ort {ort:.3}",
                t.dims()
            );
            t
        };

        let audio_features = model.encoder.forward(&mel, true)?;
        // Tanılama: kodlayıcı çıktısı girdiye göre değişiyor mu?
        let af: Vec<f32> = audio_features.flatten_all()?.to_vec1()?;
        let af_ilk: f32 = af.iter().take(5).sum::<f32>();
        let af_ort: f32 = af.iter().sum::<f32>() / af.len() as f32;
        println!(
            "kodlayıcı çıktısı: {:?} | ilk5 toplam {af_ilk:.4} | ort {af_ort:.6}",
            audio_features.dims()
        );

        let mut tokens: Vec<u32> = vec![sot];
        if let Some(dil) = &args.dil {
            let tok = format!("<|{dil}|>");
            if let Some(id) = tokenizer.token_to_id(&tok) {
                tokens.push(id);
            }
        }
        tokens.push(transcribe);
        tokens.push(no_ts);

        let mut out_ids = Vec::new();
        // Her adımda belirteç dizisinin tamamı beslenir ve KV önbelleği
        // temizlenir. Bu, artımlı önbellek yolundan daha yavaştır ancak
        // doğruluğu garanti eder ve en fazla 224 belirteç üretildiğinden
        // maliyeti kabul edilebilir.
        for _ in 0..224 {
            let giris = Tensor::new(tokens.as_slice(), &device)?.unsqueeze(0)?;
            let gizli = model.decoder.forward(&giris, &audio_features, true)?;
            // Kodçözücü çıktısı gizli durumdur; sözcük dağarcığı logitleri
            // için final_linear uygulanır.
            let seq = gizli.dim(1)?;
            let logits = model
                .decoder
                .final_linear(&gizli.i((.., seq - 1.., ..))?)?
                .i(0)?
                .i(0)?
                .to_dtype(DType::F32)?;
            let next = logits.argmax(D::Minus1)?.to_scalar::<u32>()?;
            if next == eot {
                break;
            }
            tokens.push(next);
            out_ids.push(next);
        }

        let metin = tokenizer.decode(&out_ids, true)?;
        println!("Çıktı  : {metin}");

        if let Some(ref_path) = args.referans.get(i) {
            let referans = std::fs::read_to_string(ref_path)?.trim().to_string();
            println!("Referans: {referans}");
            let (w, n, h) = wer(&referans, &metin);
            println!("WER: {:.1}%  ({h}/{n} kelime)", w * 100.0);
            toplam_sozcuk += n;
            toplam_hatali += h;
        }
    }

    if toplam_sozcuk > 0 {
        println!(
            "\n=== TOPLAM ===\nWER: {:.1}%  ({toplam_hatali}/{toplam_sozcuk} kelime)",
            toplam_hatali as f64 / toplam_sozcuk as f64 * 100.0
        );
        println!(
            "Not: Bu ölçüm yalnızca bu kayıtlar için geçerlidir ve transkriptin \
             bilirkişi tarafından doğrulandığı anlamına gelmez."
        );
    }

    Ok(())
}

/// 16 kHz mono WAV dosyasını `f32` örneklerine okur.
fn oku_wav(path: &std::path::Path) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let mut r = hound::WavReader::open(path)?;
    let spec = r.spec();
    if spec.sample_rate != 16000 {
        return Err(format!("Beklenen 16 kHz, bulunan {} Hz", spec.sample_rate).into());
    }
    if spec.channels != 1 {
        return Err(format!("Beklenen mono, bulunan {} kanal", spec.channels).into());
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => r
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<Result<_, _>>()?,
        hound::SampleFormat::Float => r.samples::<f32>().collect::<Result<_, _>>()?,
    };
    Ok(samples)
}

/// Basit WER: (yerine koyma + silme + ekleme) / referans kelime sayısı.
fn wer(referans: &str, hipotez: &str) -> (f64, usize, usize) {
    let a: Vec<String> = norm(referans);
    let b: Vec<String> = norm(hipotez);
    let n = a.len();
    if n == 0 {
        return (0.0, 0, 0);
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let d = prev[b.len()];
    (d as f64 / n as f64, n, d)
}

/// Metni karşılaştırma için sadeleştirir (küçük harf, noktalama kaldırma).
fn norm(s: &str) -> Vec<String> {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|w| w.to_string())
        .collect()
}
