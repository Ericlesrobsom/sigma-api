use std::time::Duration;

use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::config::SigmaConfig;
use crate::error::SigmaError;
use crate::models::{
    Collection, ConnectionsChange, CourtesyDays, Customer, CustomerCreate, CustomerQuery,
    CustomerUpdate, Envelope, Me, Package, PackageChange, RenewRequest, Server,
};

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Cliente HTTP da API oficial.
#[derive(Clone)]
pub struct SigmaClient {
    http: Client,
    base: String,
    token: String,
}

impl SigmaClient {
    pub fn new(config: SigmaConfig) -> Result<Self, SigmaError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs.max(1)))
            .user_agent(concat!("sigma-api/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            http,
            base: config.base_url.trim_end_matches('/').to_string(),
            token: config.token,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    pub async fn me(&self) -> Result<Me, SigmaError> {
        let env: Envelope<Me> = self.get("/me", &[]).await?;
        Ok(env.data)
    }

    pub async fn customers(
        &self,
        query: &CustomerQuery,
    ) -> Result<Collection<Customer>, SigmaError> {
        self.get("/customers", &query.pairs()).await
    }

    pub async fn customer(&self, id: &str) -> Result<Customer, SigmaError> {
        let env: Envelope<Customer> = self
            .get(&format!("/customers/{}", segment(id)), &[])
            .await?;
        Ok(env.data)
    }

    pub async fn create_customer(&self, body: &CustomerCreate) -> Result<Customer, SigmaError> {
        let value = serde_json::to_value(body)?;
        let env: Envelope<Customer> = self
            .send(Method::POST, "/customers", &[], Some(&value))
            .await?;
        Ok(env.data)
    }

    pub async fn update_customer(
        &self,
        id: &str,
        body: &CustomerUpdate,
    ) -> Result<Customer, SigmaError> {
        let value = serde_json::to_value(body)?;
        let env: Envelope<Customer> = self
            .send(
                Method::PUT,
                &format!("/customers/{}", segment(id)),
                &[],
                Some(&value),
            )
            .await?;
        Ok(env.data)
    }

    pub async fn delete_customer(&self, id: &str) -> Result<(), SigmaError> {
        let _: Value = self
            .send(
                Method::DELETE,
                &format!("/customers/{}", segment(id)),
                &[],
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn renew_customer(&self, id: &str, body: &RenewRequest) -> Result<Value, SigmaError> {
        let value = serde_json::to_value(body)?;
        self.send(
            Method::POST,
            &format!("/customers/{}/renew", segment(id)),
            &[],
            Some(&value),
        )
        .await
    }

    pub async fn change_package(
        &self,
        id: &str,
        body: &PackageChange,
    ) -> Result<Value, SigmaError> {
        let value = serde_json::to_value(body)?;
        self.send(
            Method::PUT,
            &format!("/customers/{}/package", segment(id)),
            &[],
            Some(&value),
        )
        .await
    }

    pub async fn change_connections(
        &self,
        id: &str,
        body: &ConnectionsChange,
    ) -> Result<Value, SigmaError> {
        let value = serde_json::to_value(body)?;
        self.send(
            Method::PUT,
            &format!("/customers/{}/connections", segment(id)),
            &[],
            Some(&value),
        )
        .await
    }

    pub async fn courtesy_days(&self, id: &str, body: &CourtesyDays) -> Result<Value, SigmaError> {
        let value = serde_json::to_value(body)?;
        self.send(
            Method::POST,
            &format!("/customers/{}/courtesy-days", segment(id)),
            &[],
            Some(&value),
        )
        .await
    }

    pub async fn resync_customer(&self, id: &str) -> Result<Value, SigmaError> {
        self.send(
            Method::POST,
            &format!("/customers/{}/resync", segment(id)),
            &[],
            None,
        )
        .await
    }

    pub async fn migrate_customer(&self, id: &str, body: &Value) -> Result<Value, SigmaError> {
        self.send(
            Method::PUT,
            &format!("/customers/{}/server-migration", segment(id)),
            &[],
            Some(body),
        )
        .await
    }

    pub async fn customer_by_username(&self, username: &str) -> Result<Customer, SigmaError> {
        let env: Envelope<Customer> = self
            .get(
                &format!("/customers/by-username/{}", segment(username)),
                &[],
            )
            .await?;
        Ok(env.data)
    }

    pub async fn coupons(&self) -> Result<Value, SigmaError> {
        self.get_value("/coupons", &[]).await
    }

    pub async fn coupon(&self, id: &str) -> Result<Value, SigmaError> {
        self.get_value(&format!("/coupons/{}", segment(id)), &[])
            .await
    }

    pub async fn create_coupon(&self, body: &Value) -> Result<Value, SigmaError> {
        self.post_value("/coupons", body).await
    }

    pub async fn delete_coupon(&self, id: &str) -> Result<Value, SigmaError> {
        self.delete_value(&format!("/coupons/{}", segment(id)))
            .await
    }

    pub async fn resellers(&self) -> Result<Value, SigmaError> {
        self.get_value("/resellers", &[]).await
    }

    pub async fn reseller(&self, id: &str) -> Result<Value, SigmaError> {
        self.get_value(&format!("/resellers/{}", segment(id)), &[])
            .await
    }

    pub async fn create_reseller(&self, body: &Value) -> Result<Value, SigmaError> {
        self.post_value("/resellers", body).await
    }

    pub async fn update_reseller(&self, id: &str, body: &Value) -> Result<Value, SigmaError> {
        self.put_value(&format!("/resellers/{}", segment(id)), body)
            .await
    }

    pub async fn reseller_credits(
        &self,
        id: &str,
        action: &str,
        body: &Value,
    ) -> Result<Value, SigmaError> {
        let action = match action {
            "add" | "remove" | "recharge" => action,
            other => {
                return Err(SigmaError::Config(format!(
                    "acao de creditos invalida: {other} (use add, remove ou recharge)"
                )))
            }
        };
        self.post_value(
            &format!("/resellers/{}/credits/{}", segment(id), action),
            body,
        )
        .await
    }

    pub async fn own_dns(&self) -> Result<Value, SigmaError> {
        self.get_value("/me/dns", &[]).await
    }

    pub async fn set_own_dns(&self, body: &Value) -> Result<Value, SigmaError> {
        self.put_value("/me/dns", body).await
    }

    pub async fn resellers_dns(&self) -> Result<Value, SigmaError> {
        self.get_value("/resellers/dns", &[]).await
    }

    pub async fn reseller_dns(&self, id: &str) -> Result<Value, SigmaError> {
        self.get_value(&format!("/resellers/{}/dns", segment(id)), &[])
            .await
    }

    pub async fn set_reseller_dns(&self, id: &str, body: &Value) -> Result<Value, SigmaError> {
        self.put_value(&format!("/resellers/{}/dns", segment(id)), body)
            .await
    }

    pub async fn replace_dns(&self, body: &Value) -> Result<Value, SigmaError> {
        self.post_value("/resellers/dns/replace", body).await
    }

    pub async fn apply_dns(&self, body: &Value) -> Result<Value, SigmaError> {
        self.post_value("/resellers/dns/apply", body).await
    }

    pub async fn dns_history(&self) -> Result<Value, SigmaError> {
        self.get_value("/resellers/dns/history", &[]).await
    }

    pub async fn revert_dns(&self, batch: &str) -> Result<Value, SigmaError> {
        self.send(
            Method::POST,
            &format!("/resellers/dns/history/{}/revert", segment(batch)),
            &[],
            None,
        )
        .await
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

    pub async fn packages(&self) -> Result<Collection<Package>, SigmaError> {
        self.get("/packages", &[]).await
    }

    pub async fn servers(&self) -> Result<Collection<Server>, SigmaError> {
        self.get("/servers", &[]).await
    }

    pub async fn report(&self, kind: &str) -> Result<Value, SigmaError> {
        self.get(&format!("/reports/{}", segment(kind)), &[]).await
    }

    pub async fn get_value(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Value, SigmaError> {
        self.get(path, query).await
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, SigmaError> {
        self.send(Method::GET, path, query, None).await
    }

    pub async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&Value>,
    ) -> Result<T, SigmaError> {
        let url = format!("{}/{}", self.base, path.trim_start_matches('/'));
        let mut req = self
            .http
            .request(method, url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json");
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(value) = body {
            req = req.json(value);
        }

        let resp = req.send().await?;
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
                .and_then(|e| e.message.clone().or_else(|| e.error.clone()))
                .unwrap_or_else(|| text.chars().take(500).collect());
            let code = parsed.and_then(|e| e.code);
            return Err(SigmaError::Upstream {
                status: status.as_u16(),
                code,
                message,
                retry_after,
            });
        }

        if text.trim().is_empty() {
            return Ok(serde_json::from_str("null")?);
        }
        Ok(serde_json::from_str(&text)?)
    }
}

/// Escapa um segmento de path para não permitir barra/`..` injetado.
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
