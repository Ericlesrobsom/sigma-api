use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

/// Envelope de sucesso `{ "ok": true, "data": ... }`.
pub fn ok<T: Serialize>(data: T) -> Json<Value> {
    Json(json!({ "ok": true, "data": data }))
}
