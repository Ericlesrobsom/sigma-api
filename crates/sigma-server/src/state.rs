use std::sync::Arc;

use sigma_core::SigmaClient;

/// Estado compartilhado do servidor.
#[derive(Clone)]
pub struct AppState {
    pub client: Arc<SigmaClient>,
    /// Token exigido nas rotas `/v1/*` (não é o token do painel).
    pub server_token: Arc<String>,
}

impl AppState {
    pub fn new(client: Arc<SigmaClient>, server_token: impl Into<String>) -> Self {
        Self {
            client,
            server_token: Arc::new(server_token.into()),
        }
    }
}
