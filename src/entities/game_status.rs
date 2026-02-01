use serde::{Deserialize, Serialize};
use std::fmt;

/// Game lifecycle status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GameStatus {
    /// Game is in lobby, players can join
    #[default]
    Lobby,
    /// Game is in progress
    Playing,
    /// Game has ended
    Finished,
}

impl fmt::Display for GameStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lobby => write!(f, "lobby"),
            Self::Playing => write!(f, "playing"),
            Self::Finished => write!(f, "finished"),
        }
    }
}

impl GameStatus {
    /// Convert from database string representation
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lobby" => Some(Self::Lobby),
            "playing" => Some(Self::Playing),
            "finished" => Some(Self::Finished),
            _ => None,
        }
    }

    /// Convert to database string representation
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Lobby => "lobby",
            Self::Playing => "playing",
            Self::Finished => "finished",
        }
    }

    /// Check if players can join
    pub const fn can_join(&self) -> bool {
        matches!(self, Self::Lobby)
    }

    /// Check if game can be started
    pub const fn can_start(&self) -> bool {
        matches!(self, Self::Lobby)
    }

    /// Check if game is active
    #[allow(dead_code)]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Lobby | Self::Playing)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(GameStatus::from_str("lobby"), Some(GameStatus::Lobby));
        assert_eq!(GameStatus::from_str("LOBBY"), Some(GameStatus::Lobby));
        assert_eq!(GameStatus::from_str("playing"), Some(GameStatus::Playing));
        assert_eq!(GameStatus::from_str("finished"), Some(GameStatus::Finished));
        assert_eq!(GameStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_as_str() {
        assert_eq!(GameStatus::Lobby.as_str(), "lobby");
        assert_eq!(GameStatus::Playing.as_str(), "playing");
        assert_eq!(GameStatus::Finished.as_str(), "finished");
    }

    #[test]
    fn test_can_join() {
        assert!(GameStatus::Lobby.can_join());
        assert!(!GameStatus::Playing.can_join());
        assert!(!GameStatus::Finished.can_join());
    }

    #[test]
    fn test_can_start() {
        assert!(GameStatus::Lobby.can_start());
        assert!(!GameStatus::Playing.can_start());
        assert!(!GameStatus::Finished.can_start());
    }

    #[test]
    fn test_is_active() {
        assert!(GameStatus::Lobby.is_active());
        assert!(GameStatus::Playing.is_active());
        assert!(!GameStatus::Finished.is_active());
    }

    #[test]
    fn test_default() {
        assert_eq!(GameStatus::default(), GameStatus::Lobby);
    }
}
