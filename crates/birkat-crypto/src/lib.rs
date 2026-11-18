//! # BİRKÂT şifreleme katmanı
//!
//! Bu kitaplık, 19.2 maddesindeki şifreleme gereksinimlerini uygular:
//!
//! * Vaka verisi şifreli tutulur.
//! * Büyük dosyalar şifreli blob depolama üzerinden saklanır.
//! * AES-256-GCM (AEAD) kullanılır.
//! * Anahtarlar parola + Argon2id ile türetilir ve bellekten silinir.
//! * Parola veya PIN günlük dosyasına yazılmaz.
//!
//! ## Blob biçimi
//!
//! ```text
//! [ "BKAT" (4) ][ sürüm (1) ][ tuz (16) ][ nonce (12) ][ şifreli metin + etiket ]
//! ```
//!
//! Anahtar türetme parametreleri başlığa yazılmaz; sistem varsayılanları
//! kullanılır ve sürüm numarasıyla sabitlenir.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use birkat_core::error::{CoreError, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Şifreli blob sihirli baytları.
pub const MAGIC: &[u8; 4] = b"BKAT";
/// Şifreli blob biçim sürümü.
pub const BLOB_VERSION: u8 = 1;
/// Tuz uzunluğu (bayt).
pub const SALT_LEN: usize = 16;
/// Nonce uzunluğu (bayt).
pub const NONCE_LEN: usize = 12;
/// Anahtar uzunluğu (bayt).
pub const KEY_LEN: usize = 32;
/// Şifreli blob başlık uzunluğu.
pub const HEADER_LEN: usize = 4 + 1 + SALT_LEN + NONCE_LEN;

/// Argon2id varsayılan bellek maliyeti (KiB).
pub const ARGON2_MEMORY_KIB: u32 = 64 * 1024;
/// Argon2id yineleme sayısı.
pub const ARGON2_ITERATIONS: u32 = 3;
/// Argon2id paralellik derecesi.
pub const ARGON2_PARALLELISM: u32 = 1;

/// 32 baytlık simetrik anahtar. Düşürüldüğünde bellekten silinir.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey([u8; KEY_LEN]);

impl SecretKey {
    /// Rastgele anahtar üretir.
    pub fn generate() -> Self {
        let mut key = [0u8; KEY_LEN];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }

    /// Ham baytlardan anahtar oluşturur.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    /// Anahtarın ham baytlarına erişir.
    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    /// Anahtarı Base64 olarak dışa aktarır (yedek/kurtarma için).
    pub fn export_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self.0)
    }

    /// Base64'ten anahtar içe alır.
    pub fn import_base64(value: &str) -> Result<Self> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value.trim())
            .map_err(|e| CoreError::Validation(format!("Geçersiz anahtar kodlaması: {e}")))?;
        let arr: [u8; KEY_LEN] = bytes
            .try_into()
            .map_err(|_| CoreError::Validation("Anahtar uzunluğu 32 bayt olmalıdır.".into()))?;
        Ok(Self(arr))
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey(<gizli>)")
    }
}

/// Argon2id parametreleri.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KdfParams {
    /// Bellek maliyeti (KiB).
    pub memory_kib: u32,
    /// Yineleme sayısı.
    pub iterations: u32,
    /// Paralellik derecesi.
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            memory_kib: ARGON2_MEMORY_KIB,
            iterations: ARGON2_ITERATIONS,
            parallelism: ARGON2_PARALLELISM,
        }
    }
}

/// Parola doğrulama sonucu.
#[derive(Debug, Clone)]
pub struct PasswordCheck {
    /// Parola politikayı karşılıyor mu?
    pub ok: bool,
    /// Karşılanmayan ölçütler.
    pub issues: Vec<String>,
}

/// Parola politikasını değerlendirir.
///
/// Parola hiçbir yere kaydedilmez.
pub fn check_password_policy(password: &str) -> PasswordCheck {
    let mut issues = Vec::new();
    if password.chars().count() < 12 {
        issues.push("Parola en az 12 karakter olmalıdır.".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        issues.push("Parola en az bir rakam içermelidir.".to_string());
    }
    if !password.chars().any(|c| c.is_alphabetic()) {
        issues.push("Parola en az bir harf içermelidir.".to_string());
    }
    let weak = ["123456", "password", "parola", "qwerty", "birkat"];
    let lowered = password.to_lowercase();
    if weak.iter().any(|w| lowered.contains(w)) {
        issues.push("Parola yaygın/zayıf bir ifade içermemelidir.".to_string());
    }
    PasswordCheck {
        ok: issues.is_empty(),
        issues,
    }
}

/// Parola + tuzdan Argon2id ile anahtar türetir.
pub fn derive_key(password: &str, salt: &[u8], params: KdfParams) -> Result<SecretKey> {
    let argon_params = argon2::Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(KEY_LEN),
    )
    .map_err(|e| CoreError::Validation(format!("Argon2 parametre hatası: {e}")))?;
    let argon = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon_params,
    );

    let mut out = [0u8; KEY_LEN];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|e| CoreError::Validation(format!("Anahtar türetme hatası: {e}")))?;
    Ok(SecretKey(out))
}

/// Şifreli blob üstbilgisi.
#[derive(Debug, Clone, Copy)]
pub struct BlobHeader {
    /// Biçim sürümü.
    pub version: u8,
    /// Tuz.
    pub salt: [u8; SALT_LEN],
    /// Nonce.
    pub nonce: [u8; NONCE_LEN],
}

/// Veriyi AEAD ile şifreler.
pub fn encrypt(key: &SecretKey, plaintext: &[u8]) -> Result<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);
    encrypt_with(key, &salt, &nonce_bytes, plaintext)
}

fn encrypt_with(
    key: &SecretKey,
    salt: &[u8; SALT_LEN],
    nonce_bytes: &[u8; NONCE_LEN],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_bytes()));
    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CoreError::Data("Şifreleme başarısız.".to_string()))?;

    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(BLOB_VERSION);
    out.extend_from_slice(salt);
    out.extend_from_slice(nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Şifreli blob başlığını okur.
pub fn read_header(blob: &[u8]) -> Result<BlobHeader> {
    if blob.len() < HEADER_LEN {
        return Err(CoreError::Data("Şifreli blob çok kısa.".to_string()));
    }
    if &blob[0..4] != MAGIC {
        return Err(CoreError::Data("Şifreli blob imzası geçersiz.".to_string()));
    }
    let version = blob[4];
    if version != BLOB_VERSION {
        return Err(CoreError::Data(format!(
            "Desteklenmeyen şifreli blob sürümü: {version}"
        )));
    }
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&blob[5..5 + SALT_LEN]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&blob[5 + SALT_LEN..HEADER_LEN]);
    Ok(BlobHeader {
        version,
        salt,
        nonce,
    })
}

/// Blob'u verilen anahtarla çözer.
pub fn decrypt(key: &SecretKey, blob: &[u8]) -> Result<Vec<u8>> {
    let header = read_header(blob)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_bytes()));
    let nonce = Nonce::from_slice(&header.nonce);
    cipher
        .decrypt(nonce, &blob[HEADER_LEN..])
        .map_err(|_| CoreError::Data("Çözme başarısız: anahtar hatalı veya veri bozuk.".to_string()))
}

/// Parola ile veriyi şifreler.
pub fn encrypt_with_password(password: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);
    let key = derive_key(password, &salt, KdfParams::default())?;
    encrypt_with(&key, &salt, &nonce_bytes, plaintext)
}

/// Parola ile şifrelenmiş blob'u çözer.
pub fn decrypt_with_password(password: &str, blob: &[u8]) -> Result<Vec<u8>> {
    let header = read_header(blob)?;
    let key = derive_key(password, &header.salt, KdfParams::default())?;
    decrypt(&key, blob)
}

/// Kasa anahtarını parolayla sarar (kurtarma anahtarı üretir).
pub fn wrap_vault_key(vault_key: &SecretKey, password: &str) -> Result<String> {
    use base64::Engine;
    let blob = encrypt_with_password(password, vault_key.as_bytes())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(blob))
}

/// Sarılmış kasa anahtarını parolayla çözer.
pub fn unwrap_vault_key(wrapped: &str, password: &str) -> Result<SecretKey> {
    use base64::Engine;
    let blob = base64::engine::general_purpose::STANDARD
        .decode(wrapped.trim())
        .map_err(|e| CoreError::Data(format!("Sarılmış anahtar çözülemedi: {e}")))?;
    let raw = decrypt_with_password(password, &blob)?;
    let arr: [u8; KEY_LEN] = raw
        .try_into()
        .map_err(|_| CoreError::Data("Kasa anahtarı uzunluğu geçersiz.".into()))?;
    Ok(SecretKey(arr))
}

/// Başarısız parola denemesi politikası (19.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedAttemptPolicy {
    /// İzin verilen azami başarısız deneme.
    pub max_attempts: u32,
    /// Denemeler arası bekleme (saniye).
    pub lockout_seconds: u64,
    /// Şu ana kadarki başarısız deneme sayısı.
    pub failed_attempts: u32,
    /// Kilit bitiş zamanı.
    pub locked_until: Option<birkat_core::time_util::Timestamp>,
}

impl Default for FailedAttemptPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            lockout_seconds: 300,
            failed_attempts: 0,
            locked_until: None,
        }
    }
}

impl FailedAttemptPolicy {
    /// Şu anda kilitli mi?
    pub fn is_locked(&self) -> bool {
        match self.locked_until {
            Some(until) => birkat_core::time_util::Timestamp::now() < until,
            None => false,
        }
    }

    /// Başarısız deneme kaydeder; gerekirse kilitler.
    pub fn record_failure(&mut self) {
        self.failed_attempts += 1;
        if self.failed_attempts >= self.max_attempts {
            let until = birkat_core::time_util::Timestamp::now().inner()
                + time::Duration::seconds(self.lockout_seconds as i64);
            self.locked_until = Some(birkat_core::time_util::Timestamp::from(until));
        }
    }

    /// Başarılı girişte sayaçları sıfırlar.
    pub fn record_success(&mut self) {
        self.failed_attempts = 0;
        self.locked_until = None;
    }

    /// Kalan deneme hakkı.
    pub fn remaining_attempts(&self) -> u32 {
        self.max_attempts.saturating_sub(self.failed_attempts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let key = SecretKey::generate();
        let plain = "Bilirkişi raporu — Türkçe karakterler: ğüşıöçİĞÜŞÖÇ".as_bytes();
        let blob = encrypt(&key, plain).unwrap();
        assert_eq!(decrypt(&key, &blob).unwrap(), plain);
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let k1 = SecretKey::generate();
        let k2 = SecretKey::generate();
        let blob = encrypt(&k1, b"gizli").unwrap();
        assert!(decrypt(&k2, &blob).is_err());
    }

    #[test]
    fn tampered_blob_fails_authentication() {
        let key = SecretKey::generate();
        let mut blob = encrypt(&key, b"gizli veri").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0xFF;
        assert!(decrypt(&key, &blob).is_err());
    }

    #[test]
    fn password_round_trip() {
        let blob = encrypt_with_password("Güçlü-Parola-2026", b"vaka verisi").unwrap();
        assert_eq!(
            decrypt_with_password("Güçlü-Parola-2026", &blob).unwrap(),
            b"vaka verisi"
        );
        assert!(decrypt_with_password("yanlış", &blob).is_err());
    }

    #[test]
    fn blob_has_expected_layout() {
        let key = SecretKey::generate();
        let blob = encrypt(&key, b"x").unwrap();
        assert_eq!(&blob[0..4], MAGIC);
        assert_eq!(blob[4], BLOB_VERSION);
        let h = read_header(&blob).unwrap();
        assert_eq!(h.version, BLOB_VERSION);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut blob = vec![0u8; HEADER_LEN + 16];
        blob[0] = b'X';
        assert!(read_header(&blob).is_err());
    }

    #[test]
    fn vault_key_wrap_unwrap() {
        let key = SecretKey::generate();
        let wrapped = wrap_vault_key(&key, "Kasa-Parolası-1").unwrap();
        let out = unwrap_vault_key(&wrapped, "Kasa-Parolası-1").unwrap();
        assert_eq!(out.as_bytes(), key.as_bytes());
        assert!(unwrap_vault_key(&wrapped, "yanlış").is_err());
    }

    #[test]
    fn key_base64_round_trip() {
        let key = SecretKey::generate();
        let b64 = key.export_base64();
        let back = SecretKey::import_base64(&b64).unwrap();
        assert_eq!(back.as_bytes(), key.as_bytes());
    }

    #[test]
    fn secret_key_debug_hides_material() {
        let key = SecretKey::generate();
        assert_eq!(format!("{key:?}"), "SecretKey(<gizli>)");
    }

    #[test]
    fn password_policy_rejects_weak() {
        assert!(!check_password_policy("kısa").ok);
        assert!(!check_password_policy("123456789012").ok);
        assert!(!check_password_policy("Guvenli-Parola-42").ok, "zayıf sözcük içeriyor");
        assert!(check_password_policy("Adli-Bilisim-2026").ok);
    }

    #[test]
    fn lockout_policy_locks_after_max_attempts() {
        let mut p = FailedAttemptPolicy::default();
        for _ in 0..5 {
            p.record_failure();
        }
        assert!(p.is_locked());
        assert_eq!(p.remaining_attempts(), 0);
        p.record_success();
        assert!(!p.is_locked());
        assert_eq!(p.remaining_attempts(), 5);
    }
}
