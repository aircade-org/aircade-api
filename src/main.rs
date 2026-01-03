mod api;
mod config;
mod dto;
mod entities;
mod errors;
mod middleware;
mod migrations;
mod routes;
mod services;
mod utils;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    Ok(api::Api::launch().await?)
}
