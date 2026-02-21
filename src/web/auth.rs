//! JWT authentication for web server.
#![allow(dead_code)]

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT secret key (should be configurable).
const JWT_SECRET: &[u8] = b"tinyvegeta-secret-key-change-in-production";

/// Token expiration in seconds (24 hours).
const TOKEN_EXPIRATION: u64 = 86400;

/// JWT claims.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // Subject (user ID)
    pub name: String, // User name
    pub exp: usize,   // Expiration time
    pub iat: usize,   // Issued at
}

/// Generate a JWT token.
pub fn generate_token(user_id: &str, name: &str) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        name: name.to_string(),
        exp: now + TOKEN_EXPIRATION as usize,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .map_err(|e| e.to_string())
}

/// Validate a JWT token.
pub fn validate_token(token: &str) -> Result<Claims, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|e| e.to_string())?;

    Ok(token_data.claims)
}

/// Extract token from Authorization header.
pub fn extract_token(auth_header: Option<&str>) -> Result<&str, String> {
    let header = auth_header.ok_or("Missing Authorization header")?;

    if !header.starts_with("Bearer ") {
        return Err("Invalid Authorization header format".to_string());
    }

    Ok(&header[7..])
}

/// Hash a password.
pub fn hash_password(password: &str) -> Result<String, String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())
}

/// Verify a password.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    bcrypt::verify(password, hash).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let token = generate_token("user123", "Test User").unwrap();
        let claims = validate_token(&token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.name, "Test User");
    }

    #[test]
    fn test_password_hashing() {
        let hash = hash_password("password123").unwrap();
        assert!(verify_password("password123", &hash).unwrap());
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }
}
