use std::sync::LazyLock;

use argon2::{
  Argon2, PasswordVerifier,
  password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, errors::Error as JwtError};
use jsonwebtoken::{DecodingKey, Validation, decode};
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
  sub: String,
  exp: u64,
}

static JWT_SECRET_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| {
  let mut rng = rand::rng();
  let mut jwt_secret_key = vec![0u8; 32];
  rng.fill_bytes(&mut jwt_secret_key[..]);
  jwt_secret_key
});

pub fn get_jwt_secret_key() -> &'static [u8] {
  &JWT_SECRET_KEY
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
  let salt = SaltString::generate(&mut OsRng);

  let argon2 = Argon2::default();

  argon2
    .hash_password(password.as_bytes(), &salt)
    .map(|hash| hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
  let parsed_hash = argon2::password_hash::PasswordHash::new(hash)?;

  Argon2::default()
    .verify_password(password.as_bytes(), &parsed_hash)
    .map(|_| true)
    .map_err(|_| argon2::password_hash::Error::Password)
}

/// Generate JWT
/// # Arguments
/// - `user_id`: Unique user identifier
/// - `secret`: Secret key for signing (byte array)
pub fn generate_jwt(user_id: &str, secret: &[u8]) -> Result<String, JwtError> {
  let expiration_time = (Utc::now() + Duration::hours(1)).timestamp() as u64;
  let my_claims = Claims {
    sub: user_id.to_owned(),
    exp: expiration_time,
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
