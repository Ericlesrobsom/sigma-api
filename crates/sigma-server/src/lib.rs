//! API REST local que fala com a Sigma Reseller API oficial.

pub mod auth;
pub mod envelope;
pub mod error;
pub mod routes;
pub mod state;

pub use state::AppState;

/// Monta o Router completo (público + protegido).
pub fn app(state: AppState) -> axum::Router {
    routes::router(state)
}
