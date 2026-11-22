use crate::error::SigmaError;

/// Configuração do cliente da API oficial.
#[derive(Clone, Debug)]
pub struct SigmaConfig {
    /// Base sem barra final. Default: painel ESPACEONEPLAY.
    pub base_url: String,
    /// Token de revenda (Bearer). Obrigatório.
    pub token: String,
    /// Timeout por chamada.
    pub timeout_secs: u64,
}

impl SigmaConfig {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
            timeout_secs: 30,
        }
    }

    pub fn from_env() -> Result<Self, SigmaError> {
        let base_url = std::env::var("SIGMA_API_BASE_URL")
            .unwrap_or_else(|_| "https://espaceoneplay.sigmab.pro/api/reseller-api/v1".into());
        let token = std::env::var("SIGMA_API_TOKEN")
            .map_err(|_| SigmaError::Config("SIGMA_API_TOKEN nao definido".into()))?;
        if token.trim().is_empty() {
            return Err(SigmaError::Config("SIGMA_API_TOKEN vazio".into()));
        }
        let timeout_secs = std::env::var("SIGMA_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        Ok(Self::new(base_url, token.trim()).with_timeout(timeout_secs))
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs.max(1);
        self
    }
}
