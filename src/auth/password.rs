use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;

/// Hash a password using `Argon2id`.
///
/// # Errors
///
/// Returns an error if hashing fails.
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {e}"))?;
    Ok(hash.to_string())
}

/// Verify a password against an `Argon2id` hash.
///
/// Returns `true` if the password matches, `false` otherwise.
///
/// # Errors
///
/// Returns an error if the hash format is invalid.
pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Validate password complexity rules.
///
/// Requirements: at least 8 characters, at most 128 characters.
///
/// # Errors
///
/// Returns a descriptive error message if validation fails.
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters.".to_string());
    }
    if password.len() > 128 {
        return Err("Password must be at most 128 characters.".to_string());
    }
    Ok(())
}

/// Validate email format (basic check for `@` and non-empty parts).
///
/// # Errors
///
/// Returns a descriptive error message if the email is invalid.
pub fn validate_email(email: &str) -> Result<(), String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err("Email is required.".to_string());
    }
    let parts: Vec<&str> = trimmed.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
        return Err("Invalid email format.".to_string());
    }
    Ok(())
}

/// Validate username format: 3-50 alphanumeric characters and underscores.
///
/// # Errors
///
/// Returns a descriptive error message if validation fails.
pub fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 3 {
        return Err("Username must be at least 3 characters.".to_string());
    }
    if username.len() > 50 {
        return Err("Username must be at most 50 characters.".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Username may only contain letters, numbers, and underscores.".to_string());
    }
    Ok(())
}
