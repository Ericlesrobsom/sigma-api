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
        .route("/v1/packages", get(packages))
        .route("/v1/servers", get(servers))
        .route("/v1/credit-orders", get(credit_orders))
        .route("/v1/credit-transactions", get(credit_transactions))
        .route("/v1/credit-summary", get(credit_summary))
        .route("/v1/resellers", get(resellers).post(create_reseller))
        .route("/v1/resellers/{id}/credits", post(add_reseller_credits))
        .route("/v1/customers-count", get(customers_count))
        .route(
            "/v1/customers-count-by-server",
            get(customers_count_by_server),
        )
        .route("/v1/expiring", get(expiring))
        .route("/v1/notices", get(notices))
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
