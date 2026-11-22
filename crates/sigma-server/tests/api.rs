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
    let client = SigmaClient::new(SigmaConfig::new(upstream.uri(), "upstream-token"))
        .expect("cliente upstream");
    AppState::new(Arc::new(client), server_token)
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

fn request(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", "Bearer server-token")
        .body(Body::empty())
        .unwrap()
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
async fn me_encaminha_token_do_upstream_e_devolve_envelope() {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"id": "u1", "username": "revenda", "creditsBalance": 10}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("GET", "/v1/me"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["data"]["username"], "revenda");
}

#[tokio::test]
async fn customers_encaminha_query() {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customers"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "meta": {"current_page": 2, "per_page": 5, "total": 0}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("GET", "/v1/customers?page=2&per_page=5"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["meta"]["total"], 0);
}

#[tokio::test]
async fn upstream_429_preserva_retry_after() {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "9")
                .set_body_json(json!({"message": "Too Many Requests"})),
        )
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("GET", "/v1/me"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers().get("retry-after").unwrap(), "9");
}

#[tokio::test]
async fn by_username_encaminha_path() {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customers/by-username/abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"id": "c1", "username": "abc123"}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("GET", "/v1/customers/by-username/abc123"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["username"], "abc123");
}

#[tokio::test]
async fn coupons_lista() {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/coupons"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "cp1", "code": "PROMO"}]
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("GET", "/v1/coupons"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["data"][0]["code"], "PROMO");
}

#[tokio::test]
async fn reseller_credits_encaminha_body() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/resellers/r1/credits/add"))
        .and(body_json(json!({"credits": 10})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"updated": true}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request_json(
            "POST",
            "/v1/resellers/r1/credits/add",
            json!({"credits": 10}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn dns_history_lista() {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resellers/dns/history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "pagination": {"current_page": 1, "total": 0}
        })))
        .mount(&upstream)
        .await;

    let response = app(build_state(&upstream, "server-token"))
        .oneshot(request("GET", "/v1/resellers/dns/history"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
