//! Cliente tipado da **Sigma Reseller API v1**.
//!
//! Contrato: `docs/official-api/openapi.json` (OpenAPI 3.1.0).
//! Autenticação: `Authorization: Bearer <token>`.
//! Rate limit: 120 req/min por usuário/tenant; `429` traz `Retry-After`.

pub mod cap;
pub mod client;
pub mod config;
pub mod error;
pub mod models;

pub use client::SigmaClient;
pub use config::SigmaConfig;
pub use error::SigmaError;
pub use models::*;
