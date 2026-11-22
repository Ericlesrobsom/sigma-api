use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Envelope `{ "data": ... }` usado pela API oficial.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Envelope<T> {
    pub data: T,
}

/// Coleção paginada `{ "data": [...], "meta": {...} }`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Collection<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub meta: Option<Pagination>,
}

/// `meta` das listagens oficiais.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pagination {
    #[serde(default)]
    pub current_page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
    #[serde(default)]
    pub total: Option<i64>,
    #[serde(default)]
    pub last_page: Option<i64>,
    #[serde(default)]
    pub from: Option<i64>,
    #[serde(default)]
    pub to: Option<i64>,
}

/// Perfil da conta dona do token.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Me {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default, rename = "creditsBalance")]
    pub credits_balance: Option<i64>,
    #[serde(default, rename = "integrationStatus")]
    pub integration_status: Vec<Value>,
    #[serde(default)]
    pub membership: Value,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Cliente do painel. `password` nunca deve ser logado.
#[derive(Clone, Deserialize, Serialize)]
pub struct Customer {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(default)]
    pub package_id: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub mac_address: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub is_trial: Option<String>,
    #[serde(default)]
    pub is_mag: Option<String>,
    #[serde(default)]
    pub is_asnlock: Option<String>,
    #[serde(default)]
    pub is_isplock: Option<String>,
    #[serde(default)]
    pub connections: Option<i64>,
    #[serde(default)]
    pub has_multiple_connections: Option<bool>,
    #[serde(default)]
    pub is_multi_server: Option<bool>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub expires_at_tz: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub telegram: Option<String>,
    #[serde(default)]
    pub whatsapp: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub plan_price: Option<i64>,
    #[serde(default)]
    pub playlist: Option<String>,
    #[serde(default)]
    pub m3u_url: Option<String>,
    #[serde(default)]
    pub m3u_url_short: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub package_is_adult: Option<bool>,
    #[serde(default)]
    pub bouquets: Value,
    #[serde(default)]
    pub additional_servers: Value,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Pacote/plano do catálogo.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Package {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub is_trial: Option<String>,
    #[serde(default)]
    pub is_mag: Option<String>,
    #[serde(default)]
    pub is_asnlock: Option<String>,
    #[serde(default)]
    pub is_isplock: Option<String>,
    #[serde(default)]
    pub is_adult: Option<bool>,
    #[serde(default)]
    pub is_restreamer: Option<String>,
    #[serde(default)]
    pub plan_price: Option<i64>,
    #[serde(default)]
    pub credits: Option<f64>,
    #[serde(default)]
    pub duration: Option<i64>,
    #[serde(default)]
    pub duration_in: Option<String>,
    #[serde(default)]
    pub connections: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Servidor de provisionamento.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Server {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub connection_type: Option<String>,
    #[serde(default)]
    pub has_default_customer_password: Option<bool>,
    #[serde(default)]
    pub will_count_membership: Option<String>,
    #[serde(default)]
    pub is_isplock: Option<String>,
    #[serde(default)]
    pub max_customers: Option<i64>,
    #[serde(default)]
    pub max_connections: Option<i64>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub allowed_bouquets: Value,
    #[serde(default)]
    pub cost_per_customer: Value,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Filtros aceitos por `GET /customers`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CustomerQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "perPage")]
    pub per_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "isTrial")]
    pub is_trial: Option<String>,
}

impl CustomerQuery {
    pub fn pairs(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(v) = self.page {
            out.push(("page".into(), v.to_string()));
        }
        if let Some(v) = self.per_page {
            out.push(("perPage".into(), v.to_string()));
        }
        if let Some(v) = self.username.as_deref().filter(|s| !s.is_empty()) {
            out.push(("username".into(), v.to_string()));
        }
        if let Some(v) = self.status.as_deref().filter(|s| !s.is_empty()) {
            out.push(("status".into(), v.to_string()));
        }
        if let Some(v) = self.is_trial.as_deref().filter(|s| !s.is_empty()) {
            out.push(("isTrial".into(), v.to_string()));
        }
        out
    }
}

/// Body de `POST /customers`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCreate {
    pub server_id: String,
    pub package_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_price: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bouquets: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_mag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_asnlock: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_isplock: Option<String>,
}

/// Body de `PUT /customers/{customer}` (todos opcionais).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomerUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_price: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bouquets: Option<Value>,
}

/// Body de `POST /customers/{customer}/renew`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenewRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<i64>,
}

/// Body de `PUT /customers/{customer}/package`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageChange {
    pub package_id: String,
}

/// Body de `PUT /customers/{customer}/connections`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsChange {
    pub connections: i64,
}

/// Body de `POST /customers/{customer}/courtesy-days`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtesyDays {
    pub days: i64,
}
