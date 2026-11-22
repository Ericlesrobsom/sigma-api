use serde_json::json;
use sigma_core::{CustomerQuery, SigmaClient, SigmaConfig};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> SigmaClient {
    SigmaClient::new(SigmaConfig::new(server.uri(), "test-token")).expect("cliente")
}

#[tokio::test]
async fn me_envia_bearer_e_parseia_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "u1",
                "username": "revenda",
                "creditsBalance": 1234,
                "integrationStatus": [{"key": "resellerApi", "status": "active"}],
                "membership": {"active": true}
            }
        })))
        .mount(&server)
        .await;

    let me = client(&server).me().await.expect("me");
    assert_eq!(me.id.as_deref(), Some("u1"));
    assert_eq!(me.username.as_deref(), Some("revenda"));
    assert_eq!(me.credits_balance, Some(1234));
    assert_eq!(me.integration_status.len(), 1);
}

#[tokio::test]
async fn customers_respeita_filtros_e_paginacao() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customers"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "5"))
        .and(query_param("username", "abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "c1", "username": "abc", "status": "ACTIVE", "connections": 2}],
            "meta": {"current_page": 2, "per_page": 5, "total": 7, "last_page": 2, "from": 6, "to": 7}
        })))
        .mount(&server)
        .await;

    let query = CustomerQuery {
        page: Some(2),
        per_page: Some(5),
        username: Some("abc".into()),
        ..Default::default()
    };
    let page = client(&server).customers(&query).await.expect("customers");
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id.as_deref(), Some("c1"));
    assert_eq!(page.meta.as_ref().and_then(|m| m.total), Some(7));
}

#[tokio::test]
async fn erro_401_vira_upstream_com_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
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
async fn erro_429_expoe_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "7")
                .set_body_json(json!({"message": "Too Many Requests"})),
        )
        .mount(&server)
        .await;

    let err = client(&server).me().await.expect_err("deveria falhar");
    assert_eq!(err.status(), Some(429));
    assert_eq!(err.retry_after(), Some(7));
}
