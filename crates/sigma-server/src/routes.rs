use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::middleware;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sigma_core::CustomerQuery;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::require_bearer;
use crate::envelope;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct PageQuery {
    #[serde(default)]
    page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PackageBody {
    package_id: String,
}

#[derive(Debug, Deserialize)]
struct ConnectionsBody {
    package_id: String,
    connections: i64,
}

#[derive(Debug, Deserialize)]
struct AddCreditsBody {
    credits: f64,
    #[serde(default)]
    reference: String,
    #[serde(default)]
    private_note: String,
    #[serde(default)]
    create_manual_credit_order: bool,
}

#[derive(Debug, Deserialize)]
struct RemoveCreditsBody {
    credits: f64,
    #[serde(default)]
    reference: String,
    #[serde(default)]
    private_note: String,
}

#[derive(Debug, Deserialize)]
struct MigrationBody {
    server_id: String,
    package_id: String,
}

#[derive(Debug, Deserialize)]
struct LiveQuery {
    #[serde(default)]
    page: Option<u32>,
    #[serde(default, alias = "perPage")]
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ChatbotBody {
    server_id: String,
}

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/v1/me", get(me))
        .route("/v1/customers", get(list_customers).post(create_customer))
        .route(
            "/v1/customers/by-username/{username}",
            get(customer_by_username),
        )
        .route(
            "/v1/customers/{id}",
            get(get_customer)
                .put(update_customer)
                .delete(delete_customer),
        )
        .route("/v1/customers/{id}/renew", post(renew_customer))
        .route("/v1/customers/{id}/change-package", post(change_package))
        .route(
            "/v1/customers/{id}/change-connections",
            post(change_connections),
        )
        .route(
            "/v1/customers/{id}/toggle-status",
            put(toggle_customer_status),
        )
        .route("/v1/customers/{id}/courtesy-days", post(add_courtesy_days))
        .route("/v1/customers/{id}/server-migration", put(migrate_customer))
        .route("/v1/customers/{id}/asn-lock", put(toggle_asn_lock))
        .route("/v1/customers/{id}/isp-lock", put(toggle_isp_lock))
        .route("/v1/packages", get(packages))
        .route("/v1/servers", get(servers))
        .route("/v1/credit-orders", get(credit_orders))
        .route("/v1/credit-transactions", get(credit_transactions))
        .route("/v1/credit-summary", get(credit_summary))
        .route("/v1/resellers", get(resellers).post(create_reseller))
        .route(
            "/v1/resellers/{id}",
            get(get_reseller)
                .put(update_reseller)
                .delete(delete_reseller),
        )
        .route("/v1/resellers/{id}/credits", post(add_reseller_credits))
        .route(
            "/v1/resellers/{id}/credits/remove",
            post(remove_reseller_credits),
        )
        .route(
            "/v1/resellers/{id}/toggle-status",
            put(toggle_reseller_status),
        )
        .route("/v1/customers-count", get(customers_count))
        .route(
            "/v1/customers-count-by-server",
            get(customers_count_by_server),
        )
        .route("/v1/expiring", get(expiring))
        .route("/v1/notices", get(notices))
        .route("/v1/live-connections/{server_id}", get(live_connections))
        .route("/v1/chatbot/link/{server_id}", get(chatbot_link))
        .route("/v1/chatbot/tests", post(create_chatbot_test))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi))
        .merge(protected)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "data": {
            "service": "sigma-serve",
            "version": env!("CARGO_PKG_VERSION"),
            "mode": "panel-session"
        }
    }))
}

async fn openapi() -> impl axum::response::IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("../../../docs/openapi.json"),
    )
}

async fn me(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.me().await?))
}

async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<CustomerQuery>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.customers(&query).await?))
}

async fn customer_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.customer_by_username(&username).await?,
    ))
}

async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.customer(&id).await?))
}

async fn packages(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.packages().await?))
}

async fn servers(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.servers().await?))
}

async fn credit_orders(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.credit_orders().await?))
}

async fn credit_transactions(
    State(state): State<AppState>,
    Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state
            .client
            .credit_transactions(query.page.unwrap_or(1))
            .await?,
    ))
}

async fn credit_summary(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.credit_summary().await?))
}

async fn resellers(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.resellers().await?))
}

async fn customers_count(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.customers_count().await?))
}

async fn customers_count_by_server(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.customers_count_by_server().await?,
    ))
}

async fn expiring(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.expiring().await?))
}

async fn notices(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.notices().await?))
}

async fn create_customer(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.create_customer(&body).await?))
}

async fn update_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.update_customer(&id, &patch).await?,
    ))
}

async fn renew_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.renew_customer(&id, &body).await?))
}

async fn change_package(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PackageBody>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.change_package(&id, &body.package_id).await?,
    ))
}

async fn change_connections(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ConnectionsBody>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state
            .client
            .change_connections(&id, &body.package_id, body.connections)
            .await?,
    ))
}

async fn toggle_customer_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.toggle_customer_status(&id).await?,
    ))
}

async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.delete_customer(&id).await?))
}

async fn create_reseller(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.create_reseller(&body).await?))
}

async fn add_reseller_credits(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AddCreditsBody>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state
            .client
            .add_reseller_credits(
                &id,
                body.credits,
                &body.reference,
                &body.private_note,
                body.create_manual_credit_order,
            )
            .await?,
    ))
}

async fn add_courtesy_days(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.add_courtesy_days(&id).await?))
}

async fn migrate_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<MigrationBody>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state
            .client
            .migrate_customer(&id, &body.server_id, &body.package_id)
            .await?,
    ))
}

async fn toggle_asn_lock(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.toggle_asn_lock(&id).await?))
}

async fn toggle_isp_lock(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.toggle_isp_lock(&id).await?))
}

async fn get_reseller(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.reseller(&id).await?))
}

async fn update_reseller(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.update_reseller(&id, &patch).await?,
    ))
}

async fn remove_reseller_credits(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RemoveCreditsBody>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state
            .client
            .remove_reseller_credits(&id, body.credits, &body.reference, &body.private_note)
            .await?,
    ))
}

async fn toggle_reseller_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.toggle_reseller_status(&id).await?,
    ))
}

async fn delete_reseller(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.delete_reseller(&id).await?))
}

async fn live_connections(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
    Query(query): Query<LiveQuery>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state
            .client
            .live_connections(
                &server_id,
                query.page.unwrap_or(1),
                query.per_page.unwrap_or(20),
            )
            .await?,
    ))
}

async fn chatbot_link(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.chatbot_link(&server_id).await?))
}

async fn create_chatbot_test(
    State(state): State<AppState>,
    Json(body): Json<ChatbotBody>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.create_chatbot_test(&body.server_id).await?,
    ))
}
