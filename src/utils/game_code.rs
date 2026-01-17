use rand::Rng;

/// Characters allowed in game codes (uppercase alphanumeric, excluding confusing chars)
/// Removed: 0, O, I, 1, L to avoid confusion
const GAME_CODE_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const GAME_CODE_LENGTH: usize = 6;

/// Generate a unique 6-character game code
pub fn generate_game_code() -> String {
    let mut rng = rand::thread_rng();
    (0..GAME_CODE_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..GAME_CODE_CHARS.len());
            GAME_CODE_CHARS[idx] as char
        })
        .collect()
}

/// Validate game code format
pub fn is_valid_game_code(code: &str) -> bool {
    code.len() == GAME_CODE_LENGTH
        && code
            .chars()
            .all(|c| GAME_CODE_CHARS.contains(&(c.to_ascii_uppercase() as u8)))
}

/// Normalize game code (uppercase, trimmed)
pub fn normalize_game_code(code: &str) -> String {
    code.trim().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_game_code_length() {
        let code = generate_game_code();
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn test_generate_game_code_valid_chars() {
        for _ in 0..100 {
            let code = generate_game_code();
            assert!(is_valid_game_code(&code));
        }
    }

    #[test]
    fn test_game_code_uniqueness() {
        let codes: std::collections::HashSet<String> =
            (0..1000).map(|_| generate_game_code()).collect();
        // Should have very few collisions (likely none in 1000 codes)
        assert!(codes.len() > 990);
    }

    #[test]
    fn test_is_valid_game_code() {
        assert!(is_valid_game_code("ABC234"));
        assert!(is_valid_game_code("XYZNMK"));
        assert!(!is_valid_game_code("abc")); // too short
        assert!(!is_valid_game_code("ABC1234")); // too long (7 chars)
        assert!(!is_valid_game_code("ABC12!")); // invalid char
    }

    #[test]
    fn test_normalize_game_code() {
        assert_eq!(normalize_game_code("  abc234  "), "ABC234");
        assert_eq!(normalize_game_code("XyZ789"), "XYZ789");
    }
}
