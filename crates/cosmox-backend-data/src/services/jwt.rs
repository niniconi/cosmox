use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use chrono::Utc;
use jsonwebtoken::errors::Error as JwtError;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use cosmox_configuration::Configuration;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub jti: String,
    pub iat: u64,
    pub exp: u64,
}

/// 32-byte JWT secret persisted at `{state.path}/jwt_secret.key`.
///
/// Generated once on first start and reused across restarts so existing
/// tokens stay valid. A fresh random key is created if the file is missing.
/// The file is created with owner-only permissions (0600) so other local
/// users cannot read it.
static JWT_SECRET_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let state_path = PathBuf::from(&Configuration::get_global_configuration().cosmox.state.path);
    let secret_path = state_path.join("jwt_secret.key");

    if let Ok(secret) = std::fs::read(&secret_path)
        && secret.len() == 32
    {
        return secret;
    }

    let mut secret = vec![0u8; 32];
    rand::rng().fill_bytes(&mut secret[..]);

    if let Err(err) = std::fs::create_dir_all(&state_path) {
        log::error!("Failed to create state directory {:?}: {err}", state_path);
    } else if let Err(err) = persist_secret(&secret_path, &secret) {
        log::error!("Failed to persist jwt secret to {:?}: {err}", secret_path);
    }

    secret
});

/// Write the secret to disk with mode 0600.
///
/// `mode(0o600)` applies at creation; an explicit `set_permissions` also
/// normalizes a pre-existing file (e.g. written by an earlier version with
/// the default umask).
fn persist_secret(secret_path: &Path, secret: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(secret_path)?;
        file.write_all(secret)?;
        std::fs::set_permissions(secret_path, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        std::fs::write(secret_path, secret)
    }
}

pub fn get_jwt_secret_key() -> &'static [u8] {
    &JWT_SECRET_KEY
}

/// Generate a fresh jti for a new token.
pub fn generate_jti() -> String {
    Uuid::new_v4().to_string()
}

/// Generate JWT
/// # Arguments
/// - `user_id`: Unique user identifier
/// - `jti`: Unique token identifier (see [`generate_jti`])
/// - `secret`: Secret key for signing (byte array)
/// - `expire_secs`: Token lifetime in seconds
pub fn generate_jwt(
    user_id: &str,
    jti: &str,
    secret: &[u8],
    expire_secs: u64,
) -> Result<String, JwtError> {
    let now = Utc::now().timestamp() as u64;
    let my_claims = Claims {
        sub: user_id.to_owned(),
        jti: jti.to_owned(),
        iat: now,
        exp: now + expire_secs,
    };

    let header = Header::new(Algorithm::HS256);

    encode(&header, &my_claims, &EncodingKey::from_secret(secret))
}

/// verify and decode JWT
/// # Arguments
/// - `token`: JWT string
/// - `secret`: Secret key for signature validation (byte array)
pub fn verify_and_decode_jwt(token: &str, secret: &[u8]) -> Result<Claims, JwtError> {
    let validation = Validation::new(Algorithm::HS256);

    let decoded_token = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;

    Ok(decoded_token.claims)
}

/// Verify the signature without enforcing the `exp` claim.
///
/// Used by the expiry cleanup path: an expired token still passes signature
/// validation, so its `jti` can be recovered to delete the device row.
pub fn decode_claims_without_exp_check(token: &str, secret: &[u8]) -> Result<Claims, JwtError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;

    let decoded_token = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;

    Ok(decoded_token.claims)
}

/// Whether the verification error means the token has expired.
pub fn is_token_expired(err: &JwtError) -> bool {
    matches!(
        err.kind(),
        jsonwebtoken::errors::ErrorKind::ExpiredSignature
    )
}
