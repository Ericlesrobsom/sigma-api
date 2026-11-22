use std::sync::Arc;

use reqwest::{Client, Method, Response};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::cap;
use crate::config::SigmaConfig;
use crate::error::SigmaError;
use crate::models::CustomerQuery;

#[derive(Debug, Deserialize)]
struct CapParams {
    c: usize,
    s: usize,
    d: usize,
}

#[derive(Debug, Deserialize)]
struct CapChallenge {
    challenge: CapParams,
    token: String,
}

#[derive(Debug, Deserialize)]
struct CapRedeem {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginResp {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    errors: Option<Value>,
}

/// Cliente com sessão de revenda: faz login (CAP + usuário/senha) e
/// renova o token sozinho em caso de 401.
#[derive(Clone)]
pub struct SigmaClient {
    http: Client,
    base: String,
    username: String,
    password: String,
    token: Arc<RwLock<Option<String>>>,
}

impl SigmaClient {
    pub fn new(config: SigmaConfig) -> Result<Self, SigmaError> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs.max(1)))
            .user_agent(concat!("sigma-api/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            http,
            base: config.panel_url.trim_end_matches('/').to_string(),
            username: config.username,
            password: config.password,
            token: Arc::new(RwLock::new(None)),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    /// Faz o login completo: desafio CAP, proof-of-work, redeem e login.
    pub async fn login(&self) -> Result<String, SigmaError> {
        let challenge: CapChallenge = self.plain_post("/api/cap/challenge", None).await?;
        let (c, s, d) = (
            challenge.challenge.c,
            challenge.challenge.s,
            challenge.challenge.d,
        );
        let cap_token = challenge.token.clone();
        let solutions = tokio::task::spawn_blocking(move || cap::solve(&cap_token, c, s, d))
            .await
            .map_err(|e| SigmaError::Cap(format!("solver: {e}")))?;

        let redeem: CapRedeem = self
            .plain_post(
                "/api/cap/redeem",
                Some(&json!({ "token": challenge.token, "solutions": solutions })),
            )
            .await?;
        if !redeem.success {
            return Err(SigmaError::Login(
                redeem.error.unwrap_or_else(|| "CAP recusado".to_string()),
            ));
        }
        let cap_token = redeem
            .token
            .ok_or_else(|| SigmaError::Login("CAP sem token".into()))?;

        let body = json!({
            "cap-token": cap_token,
            "username": self.username,
            "password": self.password,
            "twofactor_code": "",
            "twofactor_recovery_code": "",
            "twofactor_challenge": "",
            "twofactor_trusted_device_id": "",
        });
        let login: LoginResp = self
            .plain_post_with_headers("/api/auth/login", &body, &[("X-Cap-Capable", "1")])
            .await?;
        let token = login.token.ok_or_else(|| {
            SigmaError::Login(login.message.unwrap_or_else(|| "sem token".into()))
        })?;
        *self.token.write().await = Some(token.clone());
        Ok(token)
    }

    async fn ensure_token(&self) -> Result<String, SigmaError> {
        if let Some(token) = self.token.read().await.clone() {
            return Ok(token);
        }
        self.login().await
    }

    async fn plain_post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T, SigmaError> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.http.post(url);
        if let Some(value) = body {
            req = req.json(value);
        }
        parse_response(req.send().await?).await
    }

    async fn plain_post_with_headers<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Value,
        headers: &[(&str, &str)],
    ) -> Result<T, SigmaError> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.http.post(url).json(body);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        parse_response(req.send().await?).await
    }

    async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&Value>,
    ) -> Result<T, SigmaError> {
        let token = self.ensure_token().await?;
        let first = self
            .request(method.clone(), path, query, body, Some(&token))
            .await?;
        if first.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.token.write().await.take();
            let token = self.login().await?;
            let retry = self
                .request(method, path, query, body, Some(&token))
                .await?;
            return parse_response(retry).await;
        }
        parse_response(first).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&Value>,
        token: Option<&str>,
    ) -> Result<Response, SigmaError> {
        let url = format!("{}{}", self.base, path);
        let mut req = self
            .http
            .request(method, url)
            .header("Accept", "application/json")
            .header("X-App-Version", "4.0")
            .header("Locale", "pt")
            .header("Accept-Language", "pt-BR");
        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(value) = body {
            req = req.json(value);
        }
        Ok(req.send().await?)
    }

    pub async fn get_value(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Value, SigmaError> {
        self.send(Method::GET, path, query, None).await
    }

    pub async fn me(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/auth/me", &[]).await
    }

    pub async fn customers(&self, query: &CustomerQuery) -> Result<Value, SigmaError> {
        self.get_value("/api/customers", &query.pairs()).await
    }

    pub async fn customer(&self, id: &str) -> Result<Value, SigmaError> {
        self.get_value(&format!("/api/customers/{}", segment(id)), &[])
            .await
    }

    pub async fn customer_by_username(&self, username: &str) -> Result<Value, SigmaError> {
        let query = CustomerQuery {
            username: Some(username.to_string()),
            ..Default::default()
        };
        self.customers(&query).await
    }

    pub async fn packages(&self) -> Result<Value, SigmaError> {
        self.get_value(
            "/api/packages/price",
            &[("page".into(), "1".into()), ("name".into(), String::new())],
        )
        .await
    }

    pub async fn servers(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/servers", &[]).await
    }

    pub async fn credit_orders(&self) -> Result<Value, SigmaError> {
        self.get_value(
            "/api/creditorders",
            &[("page".into(), "1".into()), ("perPage".into(), "20".into())],
        )
        .await
    }

    pub async fn credit_transactions(&self, page: u32) -> Result<Value, SigmaError> {
        self.get_value(
            "/api/reports/credit-transactions",
            &[("page".into(), page.to_string())],
        )
        .await
    }

    pub async fn credit_summary(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/reports/credit-transactions/summary", &[])
            .await
    }

    pub async fn resellers(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/resellers/list", &[]).await
    }

    pub async fn customers_count(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/resellers/customers-count", &[]).await
    }

    pub async fn customers_count_by_server(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/resellers/customers-count-by-server", &[])
            .await
    }

    pub async fn expiring(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/customers/expiring", &[]).await
    }

    pub async fn notices(&self) -> Result<Value, SigmaError> {
        self.get_value("/api/notices/list", &[]).await
    }
}

async fn parse_response<T: DeserializeOwned>(resp: Response) -> Result<T, SigmaError> {
    let status = resp.status();
    let retry_after = resp
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok());
    let text = resp.text().await?;
    if !status.is_success() {
        let parsed: Option<ErrorBody> = serde_json::from_str(&text).ok();
        let message = parsed
            .as_ref()
            .and_then(|e| {
                e.message
                    .clone()
                    .or_else(|| e.error.clone())
                    .or_else(|| e.errors.as_ref().map(|v| v.to_string()))
            })
            .unwrap_or_else(|| text.chars().take(500).collect());
        return Err(SigmaError::Upstream {
            status: status.as_u16(),
            code: None,
            message,
            retry_after,
        });
    }
    if text.trim().is_empty() {
        return Ok(serde_json::from_str("null")?);
    }
    Ok(serde_json::from_str(&text)?)
}

fn segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | ':' => out.push(ch),
            _ => {
                let mut buf = [0u8; 4];
                for b in ch.encode_utf8(&mut buf).bytes() {
                    out.push_str(&format!("%{b:02X}"));
                }
            }
        }
    }
    out
}
