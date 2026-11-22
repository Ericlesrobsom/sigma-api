use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::middleware;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::{json, Value};
use sigma_core::{
    ConnectionsChange, CourtesyDays, CustomerCreate, CustomerQuery, CustomerUpdate, PackageChange,
    RenewRequest,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::require_bearer;
use crate::envelope;
use crate::error::ApiError;
use crate::state::AppState;

const REPORTS: [&str; 6] = [
    "customers-count",
    "resellers-count",
    "membership",
    "low-credits",
    "new-resellers",
    "expiring",
];

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/v1/me", get(me))
        .route("/v1/customers", get(list_customers).post(create_customer))
        .route(
            "/v1/customers/{id}",
            get(get_customer)
                .put(update_customer)
                .delete(delete_customer),
        )
        .route("/v1/customers/{id}/renew", post(renew_customer))
        .route("/v1/customers/{id}/package", put(change_package))
        .route("/v1/customers/{id}/connections", put(change_connections))
        .route("/v1/customers/{id}/courtesy-days", post(courtesy_days))
        .route("/v1/customers/{id}/resync", post(resync_customer))
        .route("/v1/customers/{id}/server-migration", put(migrate_customer))
        .route("/v1/packages", get(packages))
        .route("/v1/servers", get(servers))
        .route("/v1/reports/{kind}", get(report))
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
            "upstream": "configured"
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

async fn create_customer(
    State(state): State<AppState>,
    Json(body): Json<CustomerCreate>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.create_customer(&body).await?))
}

async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.customer(&id).await?))
}

async fn update_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CustomerUpdate>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.update_customer(&id, &body).await?,
    ))
}

async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state.client.delete_customer(&id).await?;
    Ok(envelope::ok(Value::Null))
}

async fn renew_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RenewRequest>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.renew_customer(&id, &body).await?))
}

async fn change_package(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PackageChange>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.change_package(&id, &body).await?))
}

async fn change_connections(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ConnectionsChange>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.change_connections(&id, &body).await?,
    ))
}

async fn courtesy_days(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CourtesyDays>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.courtesy_days(&id, &body).await?))
}

async fn resync_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.resync_customer(&id).await?))
}

async fn migrate_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(
        state.client.migrate_customer(&id, &body).await?,
    ))
}

async fn packages(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.packages().await?))
}

async fn servers(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(envelope::ok(state.client.servers().await?))
}

async fn report(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<Value>, ApiError> {
    if !REPORTS.contains(&kind.as_str()) {
        return Err(ApiError::not_found(format!(
            "relatorio desconhecido: {kind}; use um de {REPORTS:?}"
        )));
    }
    Ok(envelope::ok(state.client.report(&kind).await?))
}
