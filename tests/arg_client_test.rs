use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use azdocs::arg::{ArgClient, RetryPolicy};
use azdocs::auth::{ClientCredentialsProvider, StaticTokenProvider, TokenProvider};
use azdocs::config::Credentials;
use azdocs::error::{ArgError, AuthError};

fn test_credentials() -> Credentials {
    Credentials {
        tenant_id: "test-tenant".into(),
        client_id: "test-client".into(),
        client_secret: "test-secret".into(),
    }
}

fn arg_client(server: &MockServer) -> ArgClient<StaticTokenProvider> {
    ArgClient::with_endpoint(
        reqwest::Client::new(),
        StaticTokenProvider("test-token".into()),
        &server.uri(),
    )
    .with_retry_policy(RetryPolicy {
        max_attempts: 3,
        base_delay: std::time::Duration::from_millis(1),
        default_retry_after: std::time::Duration::from_millis(1),
    })
}

const ARG_PATH: &str = "/providers/Microsoft.ResourceGraph/resources";

mod token_provider {
    use super::*;

    #[tokio::test]
    async fn sends_client_credentials_form_and_returns_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/test-tenant/oauth2/v2.0/token"))
            .respond_with(move |req: &Request| {
                let body = String::from_utf8_lossy(&req.body);
                assert!(body.contains("grant_type=client_credentials"));
                assert!(body.contains("client_id=test-client"));
                assert!(body.contains("scope=https%3A%2F%2Fmanagement.azure.com%2F.default"));
                ResponseTemplate::new(200)
                    .set_body_json(json!({"access_token": "tok-1", "expires_in": 3600}))
            })
            .expect(1)
            .mount(&server)
            .await;
        let provider = ClientCredentialsProvider::with_authority(
            reqwest::Client::new(),
            test_credentials(),
            &server.uri(),
        );

        let token = provider.token().await.unwrap();

        assert_eq!(token, "tok-1");
    }

    #[tokio::test]
    async fn caches_token_across_calls() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/test-tenant/oauth2/v2.0/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"access_token": "tok-1", "expires_in": 3600})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = ClientCredentialsProvider::with_authority(
            reqwest::Client::new(),
            test_credentials(),
            &server.uri(),
        );

        provider.token().await.unwrap();
        let second = provider.token().await.unwrap();

        assert_eq!(second, "tok-1");
    }

    #[tokio::test]
    async fn surfaces_entra_error_code_on_rejection() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "invalid_client",
                "error_description": "AADSTS7000215: Invalid client secret.\nTrace: x"
            })))
            .mount(&server)
            .await;
        let provider = ClientCredentialsProvider::with_authority(
            reqwest::Client::new(),
            test_credentials(),
            &server.uri(),
        );

        let err = provider.token().await.unwrap_err();

        let AuthError::Rejected { status, detail } = err else {
            panic!("expected Rejected, got {err:?}");
        };
        assert_eq!((status, detail.contains("invalid_client")), (401, true));
    }
}

mod query_all {
    use super::*;

    #[tokio::test]
    async fn returns_rows_from_single_page() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalRecords": 2, "count": 2,
                "data": [{"id": "a"}, {"id": "b"}]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let outcome = arg_client(&server)
            .query_all("resources", &[])
            .await
            .unwrap();

        assert_eq!(outcome.rows.len(), 2);
    }

    #[tokio::test]
    async fn follows_skip_token_across_pages() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .and(body_partial_json(
                json!({"options": {"$skipToken": "page2"}}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalRecords": 3, "count": 1, "data": [{"id": "c"}]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalRecords": 3, "count": 2,
                "data": [{"id": "a"}, {"id": "b"}],
                "$skipToken": "page2"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let outcome = arg_client(&server)
            .query_all("resources", &[])
            .await
            .unwrap();

        assert_eq!(
            (outcome.rows.len(), outcome.pages),
            (3, 2),
            "rows: {:?}",
            outcome.rows
        );
    }

    #[tokio::test]
    async fn scopes_request_to_subscriptions() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .and(body_partial_json(json!({"subscriptions": ["sub-1"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalRecords": 0, "count": 0, "data": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        let outcome = arg_client(&server)
            .query_all("resources", &["sub-1".into()])
            .await
            .unwrap();

        assert!(outcome.rows.is_empty());
    }

    #[tokio::test]
    async fn retries_on_429_then_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalRecords": 1, "count": 1, "data": [{"id": "a"}]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let outcome = arg_client(&server)
            .query_all("resources", &[])
            .await
            .unwrap();

        assert_eq!(outcome.rows.len(), 1);
    }

    #[tokio::test]
    async fn gives_up_after_repeated_429s() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
            .expect(3)
            .mount(&server)
            .await;

        let err = arg_client(&server)
            .query_all("resources", &[])
            .await
            .unwrap_err();

        assert!(matches!(err, ArgError::ThrottledOut { attempts: 3 }));
    }

    #[tokio::test]
    async fn retries_server_errors_with_backoff() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalRecords": 1, "count": 1, "data": [{"id": "a"}]
            })))
            .mount(&server)
            .await;

        let outcome = arg_client(&server)
            .query_all("resources", &[])
            .await
            .unwrap();

        assert_eq!(outcome.rows.len(), 1);
    }

    #[tokio::test]
    async fn surfaces_api_error_without_retry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(ARG_PATH))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": {"code": "BadRequest", "message": "Query is invalid."}
            })))
            .expect(1)
            .mount(&server)
            .await;

        let err = arg_client(&server)
            .query_all("bad kql", &[])
            .await
            .unwrap_err();

        let ArgError::Api { status, detail } = err else {
            panic!("expected Api error, got {err:?}");
        };
        assert_eq!((status, detail.contains("BadRequest")), (400, true));
    }
}

#[tokio::test]
async fn unit_rejects_truncated_arg_results_without_a_continuation_token() {
    for response in [
        json!({"data":[{"id":"a"}], "resultTruncated":true}),
        json!({"data":[{"id":"a"}], "resultTruncated":"true"}),
        json!({"data":[{"id":"a"}], "totalRecords":2}),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        let result = arg_client(&server)
            .query_all("resources | project id | order by id asc", &[])
            .await;
        assert!(matches!(result, Err(ArgError::Truncated)));
    }
}
