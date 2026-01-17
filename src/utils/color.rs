/// Predefined player colors (hex format)
/// Chosen for good visibility and distinctiveness
pub const PLAYER_COLORS: &[&str] = &[
    "#FF6B6B", // Red
    "#4ECDC4", // Teal
    "#45B7D1", // Blue
    "#96CEB4", // Green
    "#FFEAA7", // Yellow
    "#DDA0DD", // Plum
    "#98D8C8", // Mint
    "#F7DC6F", // Gold
    "#BB8FCE", // Purple
    "#85C1E9", // Light Blue
    "#F8B500", // Orange
    "#58D68D", // Lime
    "#EC7063", // Coral
    "#5DADE2", // Sky Blue
    "#F1948A", // Pink
    "#82E0AA", // Light Green
];

/// Assign a color to a player based on their position in the game
#[allow(dead_code)]
pub fn assign_player_color(player_index: usize) -> &'static str {
    PLAYER_COLORS[player_index % PLAYER_COLORS.len()]
}

/// Get the next available color that hasn't been taken
pub fn get_next_available_color(taken_colors: &[String]) -> &'static str {
    PLAYER_COLORS
        .iter()
        .find(|&&color| !taken_colors.contains(&color.to_string()))
        .copied()
        .unwrap_or(PLAYER_COLORS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_player_color() {
        assert_eq!(assign_player_color(0), "#FF6B6B");
        assert_eq!(assign_player_color(1), "#4ECDC4");
    }

    #[test]
    fn test_color_wraps_around() {
        let color_at_16 = assign_player_color(16);
        let color_at_0 = assign_player_color(0);
        assert_eq!(color_at_16, color_at_0);
    }

    #[test]
    fn test_get_next_available_color() {
        let taken = vec!["#FF6B6B".to_string(), "#4ECDC4".to_string()];
        assert_eq!(get_next_available_color(&taken), "#45B7D1");
    }

    #[test]
    fn test_get_next_available_color_all_taken() {
        // When all colors are taken, should return the first color
        let taken: Vec<String> = PLAYER_COLORS.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(get_next_available_color(&taken), "#FF6B6B");
    }
}
