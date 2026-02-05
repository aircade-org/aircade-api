//! In-memory session connection manager for `WebSocket` relay.
//!
//! Tracks active `WebSocket` connections per session, supporting the host (one per session)
//! and players (many per session). Provides broadcast and targeted message delivery.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// A message destined for a specific `WebSocket` client.
pub type WsTx = mpsc::UnboundedSender<String>;

/// Identifies a connected client within a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientRole {
    Host,
    Player(Uuid),
}

/// Tracks all active `WebSocket` connections across all sessions.
#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    /// `session_id` → map of `ClientRole` → sender channel
    sessions: Arc<DashMap<Uuid, DashMap<ClientRole, WsTx>>>,
}

impl SessionManager {
    /// Create a new empty session manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Register a client connection for a session.
    pub fn register(&self, session_id: Uuid, role: ClientRole, tx: WsTx) {
        self.sessions
            .entry(session_id)
            .or_default()
            .insert(role, tx);
    }

    /// Unregister a client connection from a session.
    pub fn unregister(&self, session_id: Uuid, role: &ClientRole) {
        if let Some(clients) = self.sessions.get(&session_id) {
            clients.remove(role);
            if clients.is_empty() {
                drop(clients);
                self.sessions.remove(&session_id);
            }
        }
    }

    /// Send a message to the host of a session.
    pub fn send_to_host(&self, session_id: Uuid, message: &str) {
        if let Some(clients) = self.sessions.get(&session_id)
            && let Some(tx) = clients.get(&ClientRole::Host)
        {
            let _ = tx.send(message.to_string());
        }
    }

    /// Send a message to a specific player in a session.
    pub fn send_to_player(&self, session_id: Uuid, player_id: Uuid, message: &str) {
        if let Some(clients) = self.sessions.get(&session_id)
            && let Some(tx) = clients.get(&ClientRole::Player(player_id))
        {
            let _ = tx.send(message.to_string());
        }
    }

    /// Broadcast a message to all connected clients in a session.
    pub fn broadcast(&self, session_id: Uuid, message: &str) {
        if let Some(clients) = self.sessions.get(&session_id) {
            for entry in clients.iter() {
                let _ = entry.value().send(message.to_string());
            }
        }
    }

    /// Broadcast a message to all players (not the host) in a session.
    pub fn broadcast_to_players(&self, session_id: Uuid, message: &str) {
        if let Some(clients) = self.sessions.get(&session_id) {
            for entry in clients.iter() {
                if matches!(entry.key(), ClientRole::Player(_)) {
                    let _ = entry.value().send(message.to_string());
                }
            }
        }
    }

    /// Remove all connections for a session (used when ending a session).
    pub fn remove_session(&self, session_id: Uuid) {
        self.sessions.remove(&session_id);
    }

    /// Check if a specific client is connected.
    #[must_use]
    pub fn is_connected(&self, session_id: Uuid, role: &ClientRole) -> bool {
        self.sessions
            .get(&session_id)
            .is_some_and(|clients| clients.contains_key(role))
    }

    /// Check if any players are connected to a session.
    #[must_use]
    pub fn has_connected_players(&self, session_id: Uuid) -> bool {
        self.sessions.get(&session_id).is_some_and(|clients| {
            clients
                .iter()
                .any(|entry| matches!(entry.key(), ClientRole::Player(_)))
        })
    }
}
