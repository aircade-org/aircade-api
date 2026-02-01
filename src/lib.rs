//! AirCade API - Backend for browser-based party games
//!
//! This crate provides the REST API for AirCade, enabling:
//! - Game session creation and management
//! - Player joining and game participation
//! - Real-time game state updates (coming in future versions)

pub mod api;
pub mod config;
pub mod dto;
pub mod entities;
pub mod errors;
pub mod middleware;
pub mod migrations;
pub mod routes;
pub mod services;
pub mod utils;
