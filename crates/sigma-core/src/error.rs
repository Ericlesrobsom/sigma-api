/// Erros do cliente e do servidor.
#[derive(Debug, thiserror::Error)]
pub enum SigmaError {
    #[error("configuracao: {0}")]
    Config(String),

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("url: {0}")]
    Url(#[from] url::ParseError),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("upstream {status}: {message}")]
    Upstream {
        status: u16,
        code: Option<String>,
        message: String,
        retry_after: Option<u64>,
    },
}

impl SigmaError {
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Upstream { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Upstream { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    pub fn retry_after(&self) -> Option<u64> {
        match self {
            Self::Upstream { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}
