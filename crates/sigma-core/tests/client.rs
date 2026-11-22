use serde_json::json;
use sigma_core::{SigmaClient, SigmaConfig};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> SigmaClient {
    SigmaClient::new(SigmaConfig::new(server.uri(), "revenda", "senha")).expect("cliente")
}

async fn mock_login(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/api/cap/challenge"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "challenge": {"c": 1, "s": 32, "d": 1},
            "token": "cap-token-teste",
            "expires": 9999999999999u64
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/cap/redeem"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "token": "cap-redeem-token",
            "expires": 9999999999999u64
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "u1",
            "username": "revenda",
            "token": "session-token"
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn faz_login_com_cap_e_usa_bearer() {
    let server = MockServer::start().await;
    mock_login(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .and(header("authorization", "Bearer session-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "u1",
            "username": "revenda",
            "credits": 10
        })))
        .mount(&server)
        .await;

    let me = client(&server).me().await.expect("me");
    assert_eq!(me["id"], "u1");
    assert_eq!(me["username"], "revenda");
}

#[tokio::test]
async fn clientes_encaminham_filtros_do_painel() {
    let server = MockServer::start().await;
    mock_login(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "c1", "username": "abc"}],
            "meta": {"current_page": 1, "total": 1}
        })))
        .mount(&server)
        .await;

    let query = sigma_core::CustomerQuery {
        page: Some(1),
        per_page: Some(20),
        username: Some("abc".into()),
        ..Default::default()
    };
    let page = client(&server).customers(&query).await.expect("customers");
    assert_eq!(page["data"][0]["id"], "c1");
}

#[tokio::test]
async fn erro_401_do_painel_vira_upstream() {
    let server = MockServer::start().await;
    mock_login(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({"message": "Unauthenticated."})),
        )
        .mount(&server)
        .await;

    let err = client(&server).me().await.expect_err("deveria falhar");
    assert_eq!(err.status(), Some(401));
    assert!(err.to_string().contains("Unauthenticated"));
}
