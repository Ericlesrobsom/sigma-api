use crate::error::SigmaError;

/// Configuração do acesso ao painel como revenda.
#[derive(Clone, Debug)]
pub struct SigmaConfig {
    /// Raiz do painel, sem barra final. Ex.: https://espaceoneplay.sigmab.pro
    pub panel_url: String,
    pub username: String,
    pub password: String,
    pub timeout_secs: u64,
}

impl SigmaConfig {
    pub fn new(
        panel_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            panel_url: panel_url.into().trim_end_matches('/').to_string(),
            username: username.into(),
            password: password.into(),
            timeout_secs: 30,
        }
    }

    pub fn from_env() -> Result<Self, SigmaError> {
        let panel_url = std::env::var("SIGMA_PANEL_URL")
            .unwrap_or_else(|_| "https://espaceoneplay.sigmab.pro".into());
        let username = std::env::var("SIGMA_USER")
            .map_err(|_| SigmaError::Config("SIGMA_USER nao definido".into()))?;
        let password = std::env::var("SIGMA_PASS")
            .map_err(|_| SigmaError::Config("SIGMA_PASS nao definido".into()))?;
        if username.trim().is_empty() {
            return Err(SigmaError::Config("SIGMA_USER vazio".into()));
        }
        if password.is_empty() {
            return Err(SigmaError::Config("SIGMA_PASS vazio".into()));
        }
        let timeout_secs = std::env::var("SIGMA_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        Ok(Self::new(panel_url, username.trim(), password).with_timeout(timeout_secs))
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs.max(1);
        self
    }
}
