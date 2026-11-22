use serde_json::json;
use sigma_core::{SigmaClient, SigmaConfig};
use wiremock::matchers::{body_json, header, method, path};
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

#[tokio::test]
async fn cria_cliente_com_body_observado() {
    let server = MockServer::start().await;
    mock_login(&server).await;
    let body = json!({
        "server_id": "s1",
        "package_id": "p1",
        "username": "testeapi",
        "password": "senha123",
        "connections": 1
    });
    Mock::given(method("POST"))
        .and(path("/api/customers"))
        .and(body_json(body.clone()))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "data": {"id": "c1", "username": "testeapi"}
        })))
        .mount(&server)
        .await;

    let criado = client(&server)
        .create_customer(&body)
        .await
        .expect("criar cliente");
    assert_eq!(criado["data"]["id"], "c1");
}

#[tokio::test]
async fn update_cliente_faz_merge_do_objeto_completo() {
    let server = MockServer::start().await;
    mock_login(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/customers/c1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "c1",
                "server_id": "s1",
                "username": "testeapi",
                "name": "Antigo",
                "note": "observacao"
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/customers/c1"))
        .and(body_json(json!({
            "id": "c1",
            "server_id": "s1",
            "username": "testeapi",
            "name": "Novo",
            "note": "observacao"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"id": "c1", "name": "Novo"}
        })))
        .mount(&server)
        .await;

    let atualizado = client(&server)
        .update_customer("c1", &json!({"name": "Novo"}))
        .await
        .expect("atualizar cliente");
    assert_eq!(atualizado["data"]["name"], "Novo");
}

#[tokio::test]
async fn toggle_e_delete_cliente() {
    let server = MockServer::start().await;
    mock_login(&server).await;
    Mock::given(method("PUT"))
        .and(path("/api/customers/c1/toggle-status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"id": "c1", "status": "ACTIVE"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/customers/c1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "deleted_at": "2026-11-22T21:35:11Z"
        })))
        .mount(&server)
        .await;

    let st = client(&server)
        .toggle_customer_status("c1")
        .await
        .expect("toggle");
    assert_eq!(st["data"]["status"], "ACTIVE");
    client(&server).delete_customer("c1").await.expect("delete");
}

#[tokio::test]
async fn adiciona_creditos_revenda() {
    let server = MockServer::start().await;
    mock_login(&server).await;
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
        .mount(&server)
        .await;

    let out = client(&server)
        .add_reseller_credits("r1", 10.0, "", "", false)
        .await
        .expect("creditos");
    assert_eq!(out["data"]["credits"], 20);
}
