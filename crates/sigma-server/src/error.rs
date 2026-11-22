use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use sigma_core::SigmaError;

/// Erro da nossa API, com status/código estáveis.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub retry_after: Option<u64>,
}

impl ApiError {
    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: "informe Authorization: Bearer <SIGMA_SERVER_TOKEN>".into(),
            retry_after: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
            retry_after: None,
        }
    }
}

impl From<SigmaError> for ApiError {
    fn from(err: SigmaError) -> Self {
        let status = err
            .status()
            .and_then(|s| StatusCode::from_u16(s).ok())
            .unwrap_or(StatusCode::BAD_GATEWAY);
        let code = match status.as_u16() {
            401 => "unauthorized",
            402 => "integration_required",
            403 => "forbidden",
            404 => "not_found",
            409 => "conflict",
            422 => "validation_error",
            429 => "rate_limited",
            _ => "upstream_error",
        };
        Self {
            status,
            code,
            message: err.to_string(),
            retry_after: err.retry_after(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "ok": false,
            "error": { "code": self.code, "message": self.message }
        }));
        let mut response = (self.status, body).into_response();
        if let Some(secs) = self.retry_after {
            if let Ok(value) = HeaderValue::from_str(&secs.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
        }
        response
    }
}
