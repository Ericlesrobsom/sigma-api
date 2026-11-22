use std::sync::Arc;

use anyhow::{bail, Context};
use sigma_core::{SigmaClient, SigmaConfig};
use sigma_server::{app, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("sigma_serve=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = SigmaConfig::from_env().context("config do painel")?;
    let server_token = std::env::var("SIGMA_SERVER_TOKEN")
        .context("SIGMA_SERVER_TOKEN nao definido (token do nosso servidor)")?;
    if server_token.trim().is_empty() {
        bail!("SIGMA_SERVER_TOKEN vazio");
    }
    let bind = std::env::var("SIGMA_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());

    let client = Arc::new(SigmaClient::new(config)?);
    let state = AppState::new(client, server_token.trim());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("bind em {bind}"))?;

    tracing::info!(%bind, "sigma-serve ouvindo");
    axum::serve(listener, app(state)).await?;
    Ok(())
}
