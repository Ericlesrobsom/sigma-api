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

/// Cliente com sessÃ£o de revenda: faz login (CAP + usuÃ¡rio/senha) e
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

    pub async fn create_customer(&self, body: &Value) -> Result<Value, SigmaError> {
        self.post_value("/api/customers", body).await
    }

    /// Atualiza por patch: busca o cliente atual, aplica sÃ³ os campos
    /// gravÃ¡veis e envia o objeto completo (o painel espera o full object).
    pub async fn update_customer(&self, id: &str, patch: &Value) -> Result<Value, SigmaError> {
        const GRAVAVEIS: [&str; 12] = [
            "username",
            "password",
            "mac_address",
            "name",
            "email",
            "telegram",
            "whatsapp",
            "note",
            "plan_price",
            "bouquets",
            "playlist",
            "parent_can_edit_personal_data",
        ];
        let atual = self.customer(id).await?;
        let mut data = atual.get("data").cloned().unwrap_or(atual);
        let obj = data
            .as_object_mut()
            .ok_or_else(|| SigmaError::Config("cliente atual sem objeto data".into()))?;
        let patch_obj = patch
            .as_object()
            .ok_or_else(|| SigmaError::Config("patch do cliente precisa ser objeto".into()))?;
        for (chave, valor) in patch_obj {
            if GRAVAVEIS.contains(&chave.as_str()) {
                obj.insert(chave.clone(), valor.clone());
            }
        }
        self.put_value(&format!("/api/customers/{}", segment(id)), &data)
            .await
    }

    pub async fn renew_customer(&self, id: &str, body: &Value) -> Result<Value, SigmaError> {
        self.post_value(&format!("/api/customers/{}/renew", segment(id)), body)
            .await
    }

    pub async fn change_package(&self, id: &str, package_id: &str) -> Result<Value, SigmaError> {
        self.post_value(
            &format!("/api/customers/{}/change-package", segment(id)),
            &json!({ "package_id": package_id }),
        )
        .await
    }

    pub async fn change_connections(
        &self,
        id: &str,
        package_id: &str,
        connections: i64,
    ) -> Result<Value, SigmaError> {
        self.post_value(
            &format!("/api/customers/{}/change-connections", segment(id)),
            &json!({ "package_id": package_id, "connections": connections }),
        )
        .await
    }

    pub async fn toggle_customer_status(&self, id: &str) -> Result<Value, SigmaError> {
        self.put_value(
            &format!("/api/customers/{}/toggle-status", segment(id)),
            &json!({}),
        )
        .await
    }

    pub async fn delete_customer(&self, id: &str) -> Result<Value, SigmaError> {
        self.delete_value(&format!("/api/customers/{}", segment(id)))
            .await
    }

    /// Cria revenda preenchendo os defaults observados no painel.
    /// O body precisa de `username`, `password` e `credits` (minimo 10).
    pub async fn create_reseller(&self, body: &Value) -> Result<Value, SigmaError> {
        let me = self.me().await?;
        let parent = me
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SigmaError::Config("me sem id".into()))?;
        let mut full = json!({
            "id": "",
            "name": "",
            "parent_user_id": parent,
            "username": "",
            "password": "",
            "email": "",
            "role": "reseller",
            "status": "ACTIVE",
            "telegram": "",
            "whatsapp": "",
            "credits": 10,
            "note": "",
            "parent": "",
            "disable_after_days_without_recharge": 0,
            "subresellers": [],
            "servers": [],
            "server_list_count": [],
            "need_reset_password": "NO",
            "membership_active": "NO",
            "membership_type": "PER_ACTIVE_CUSTOMER",
            "membership_plan_type": "PER_ACTIVE_CUSTOMER",
            "membership_cost": "",
            "membership_cost_extra": "",
            "membership_credits_to_recharge": 0,
            "membership_days_recurrency": 30,
            "membership_min_customers_to_charge": 0,
            "membership_max_number_of_customers": 0,
            "membership_expiry_date": "",
            "membership_expiry_time": "23:30",
            "membership_reset_credits_on_expiry_date": "NO",
            "parent_can_edit_personal_data": "YES"
        });
        if let (Some(dest), Some(src)) = (full.as_object_mut(), body.as_object()) {
            for (k, v) in src {
                dest.insert(k.clone(), v.clone());
            }
        }
        self.post_value("/api/resellers", &full).await
    }

    pub async fn add_reseller_credits(
        &self,
        id: &str,
        credits: f64,
        reference: &str,
        private_note: &str,
        create_manual_credit_order: bool,
    ) -> Result<Value, SigmaError> {
        self.post_value(
            &format!("/api/resellers/{}/add-credits", segment(id)),
            &json!({
                "credits": credits,
                "reference": reference,
                "private_note": private_note,
                "create_manual_credit_order": create_manual_credit_order,
                "manual_payment_total": null
            }),
        )
        .await
    }

    pub async fn add_courtesy_days(&self, id: &str) -> Result<Value, SigmaError> {
        self.post_value(
            &format!("/api/customers/{}/add-courtesy-days", segment(id)),
            &json!({}),
        )
        .await
    }

    pub async fn migrate_customer(
        &self,
        id: &str,
        server_id: &str,
        package_id: &str,
    ) -> Result<Value, SigmaError> {
        self.put_value(
            &format!("/api/customers/{}/server-migration", segment(id)),
            &json!({ "server_id": server_id, "package_id": package_id }),
        )
        .await
    }

    pub async fn toggle_asn_lock(&self, id: &str) -> Result<Value, SigmaError> {
        self.put_value(
            &format!("/api/customers/{}/toggle-asn-lock", segment(id)),
            &json!({}),
        )
        .await
    }

    pub async fn toggle_isp_lock(&self, id: &str) -> Result<Value, SigmaError> {
        self.put_value(
            &format!("/api/customers/{}/toggle-isp-lock", segment(id)),
            &json!({}),
        )
        .await
    }

    pub async fn reseller(&self, id: &str) -> Result<Value, SigmaError> {
        let lista = self.resellers().await?;
        let arr = lista
            .as_array()
            .cloned()
            .or_else(|| lista.get("data").and_then(|v| v.as_array()).cloned())
            .unwrap_or_default();
        arr.into_iter()
            .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or_else(|| SigmaError::Config(format!("revenda {id} nao encontrada")))
    }

    /// Atualiza revenda por patch, enviando o objeto completo como o painel exige.
    pub async fn update_reseller(&self, id: &str, patch: &Value) -> Result<Value, SigmaError> {
        const GRAVAVEIS: [&str; 12] = [
            "username",
            "password",
            "name",
            "email",
            "telegram",
            "whatsapp",
            "note",
            "role",
            "servers",
            "disable_after_days_without_recharge",
            "need_reset_password",
            "parent_can_edit_personal_data",
        ];
        let mut atual = self.reseller(id).await?;
        let obj = atual
            .as_object_mut()
            .ok_or_else(|| SigmaError::Config("revenda atual sem objeto".into()))?;
        let patch_obj = patch
            .as_object()
            .ok_or_else(|| SigmaError::Config("patch da revenda precisa ser objeto".into()))?;
        for (chave, valor) in patch_obj {
            if GRAVAVEIS.contains(&chave.as_str()) {
                obj.insert(chave.clone(), valor.clone());
            }
        }
        self.put_value(&format!("/api/resellers/{}", segment(id)), &atual)
            .await
    }

    pub async fn remove_reseller_credits(
        &self,
        id: &str,
        credits: f64,
        reference: &str,
        private_note: &str,
    ) -> Result<Value, SigmaError> {
        self.post_value(
            &format!("/api/resellers/{}/remove-credits", segment(id)),
            &json!({
                "credits": credits,
                "reference": reference,
                "private_note": private_note
            }),
        )
        .await
    }

    pub async fn toggle_reseller_status(&self, id: &str) -> Result<Value, SigmaError> {
        self.put_value(
            &format!("/api/resellers/{}/toggle-status", segment(id)),
            &json!({}),
        )
        .await
    }

    pub async fn delete_reseller(&self, id: &str) -> Result<Value, SigmaError> {
        self.delete_value(&format!("/api/resellers/{}", segment(id)))
            .await
    }

    pub async fn live_connections(
        &self,
        server_id: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Value, SigmaError> {
        self.get_value(
            &format!("/api/customers/live-connections/{}", segment(server_id)),
            &[
                ("page".into(), page.to_string()),
                ("perPage".into(), per_page.to_string()),
            ],
        )
        .await
    }

    /// Link público do BotBot: POST sem login cria um teste no servidor.
    pub async fn chatbot_link(&self, server_id: &str) -> Result<Value, SigmaError> {
        let me = self.me().await?;
        let user_id = me
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SigmaError::Config("me sem id".into()))?;
        Ok(json!({
            "url": format!(
                "{}/api/chatbot/{}/{}",
                self.base,
                segment(user_id),
                segment(server_id)
            )
        }))
    }

    /// Cria um teste real via link público do BotBot (sem sessão).
    pub async fn create_chatbot_test(&self, server_id: &str) -> Result<Value, SigmaError> {
        let link = self.chatbot_link(server_id).await?;
        let url = link
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SigmaError::Config("link do chatbot sem url".into()))?;
        let resp = self.http.post(url).send().await?;
        parse_response(resp).await
    }

    pub async fn post_value(&self, path: &str, body: &Value) -> Result<Value, SigmaError> {
        self.send(Method::POST, path, &[], Some(body)).await
    }

    pub async fn put_value(&self, path: &str, body: &Value) -> Result<Value, SigmaError> {
        self.send(Method::PUT, path, &[], Some(body)).await
    }

    pub async fn delete_value(&self, path: &str) -> Result<Value, SigmaError> {
        self.send(Method::DELETE, path, &[], None).await
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
