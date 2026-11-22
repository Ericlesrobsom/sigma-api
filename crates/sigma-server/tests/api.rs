use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use sigma_core::{SigmaClient, SigmaConfig};
use sigma_server::{app, AppState};
use tower::ServiceExt;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn build_state(upstream: &MockServer, server_token: &str) -> AppState {
    let client = SigmaClient::new(SigmaConfig::new(upstream.uri(), "revenda", "senha"))
        .expect("cliente upstream");
    AppState::new(Arc::new(client), server_token)
}

async fn mock_login(upstream: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/api/cap/challenge"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "challenge": {"c": 1, "s": 32, "d": 1},
            "token": "cap-token-teste",
            "expires": 9999999999999u64
        })))
        .mount(upstream)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/cap/redeem"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "token": "cap-redeem-token",
            "expires": 9999999999999u64
        })))
        .mount(upstream)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "u1",
            "username": "revenda",
            "token": "session-token"
        })))
        .mount(upstream)
        .await;
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

fn request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header("authorization", "Bearer server-token")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn health_e_publico() {
    let upstream = MockServer::start().await;
    let response = app(build_state(&upstream, "server-token"))
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn v1_exige_bearer() {
    let upstream = MockServer::start().await;
    let response = app(build_state(&upstream, "server-token"))
        .oneshot(
            Request::builder()
                .uri("/v1/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_faz_login_no_painel_e_devolve_envelope() {
    let upstream = MockServer::start().await;
    mock_login(&upstream).await;
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "u1",
            "username": "revenda",
            "credits": 10
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("/v1/me"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["data"]["username"], "revenda");
}

#[tokio::test]
async fn clientes_encaminham_per_page() {
    let upstream = MockServer::start().await;
    mock_login(&upstream).await;
    Mock::given(method("GET"))
        .and(path("/api/customers"))
        .and(query_param("perPage", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "c1"}],
            "meta": {"current_page": 1, "total": 1}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("/v1/customers?page=1&perPage=5"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["data"][0]["id"], "c1");
}

#[tokio::test]
async fn resumo_de_creditos() {
    let upstream = MockServer::start().await;
    mock_login(&upstream).await;
    Mock::given(method("GET"))
        .and(path("/api/reports/credit-transactions/summary"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"credits_total": 10}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("/v1/credit-summary"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["data"]["credits_total"], 10);
}

fn request_json(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", "Bearer server-token")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn cria_cliente_pela_api() {
    let upstream = MockServer::start().await;
    mock_login(&upstream).await;
    Mock::given(method("POST"))
        .and(path("/api/customers"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "data": {"id": "c1", "username": "testeapi"}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request_json(
            "POST",
            "/v1/customers",
            json!({"server_id": "s1", "package_id": "p1", "username": "testeapi"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["data"]["id"], "c1");
}

#[tokio::test]
async fn adiciona_creditos_pela_api() {
    let upstream = MockServer::start().await;
    mock_login(&upstream).await;
    Mock::given(method("POST"))
        .and(path("/api/resellers/r1/add-credits"))
        .and(body_json(json!({
            "credits": 10.0,
            "reference": "",
            "private_note": "",
            "create_manual_credit_order": false,
            "manual_payment_total": null
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"id": "r1", "credits": 20}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request_json(
            "POST",
            "/v1/resellers/r1/credits",
            json!({"credits": 10}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["data"]["credits"], 20);
}
