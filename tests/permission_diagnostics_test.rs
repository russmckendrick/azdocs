use azdocs::auth::StaticTokenProvider;
use azdocs::auth::diagnostics::{
    AccessIssueKind, PermissionVerdict, classify_permissions, inspect_at,
};
use base64::Engine;
use serde_json::{Value, json};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PRINCIPAL: &str = "11111111-1111-4111-8111-111111111111";
const SUB: &str = "22222222-2222-4222-8222-222222222222";
fn token() -> StaticTokenProvider {
    StaticTokenProvider(format!(
        "header.{}.signature",
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(format!(r#"{{"oid":"{PRINCIPAL}"}}"#))
    ))
}
fn permissions(actions: Value, exclusions: Value, data: Value, data_exclusions: Value) -> Value {
    json!({"actions":actions,"notActions":exclusions,"dataActions":data,"notDataActions":data_exclusions})
}
fn reader() -> Value {
    permissions(json!(["*/read"]), json!([]), json!([]), json!([]))
}
fn assignment(id: &str, role: &str, scope: &str) -> Value {
    json!({"id":format!("/assignments/{id}"),"properties":{"scope":scope,"roleDefinitionId":format!("/providers/Microsoft.Authorization/roleDefinitions/{role}"),"principalId":PRINCIPAL}})
}
async fn discovery(server: &MockServer) {
    Mock::given(method("POST")).and(path("/providers/Microsoft.ResourceGraph/resources"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":format!("/subscriptions/{SUB}"),"subscriptionId":SUB,"name":"Test"}],"totalRecords":1}))).mount(server).await;
}
async fn role(server: &MockServer, id: &str, name: &str, blocks: Value) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/providers/Microsoft.Authorization/roleDefinitions/{id}"
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"properties":{"roleName":name,"permissions":blocks}})),
        )
        .mount(server)
        .await;
}
async fn assignments(server: &MockServer, rows: Value) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/subscriptions/{SUB}/providers/Microsoft.Authorization/roleAssignments"
        )))
        .and(query_param("$filter", format!("assignedTo('{PRINCIPAL}')")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":rows})))
        .expect(1)
        .mount(server)
        .await;
}
#[test]
fn unit_classifies_permission_blocks_without_using_role_names() {
    let cases = [
        (json!([reader()]), PermissionVerdict::ReadOnly),
        (
            json!([
                reader(),
                permissions(
                    json!(["*"]),
                    json!(["Microsoft.Authorization/*/Write"]),
                    json!([]),
                    json!([])
                )
            ]),
            PermissionVerdict::BroaderGrants,
        ),
        (
            json!([permissions(
                json!(["Microsoft.Compute/*"]),
                json!(["Microsoft.Compute/*"]),
                json!([]),
                json!([])
            )]),
            PermissionVerdict::ReadOnly,
        ),
        (
            json!([permissions(
                json!(["Microsoft.Compute/virtualMachines/write"]),
                json!(["Microsoft.Compute/*/write"]),
                json!([]),
                json!([])
            )]),
            PermissionVerdict::ReadOnly,
        ),
        (
            json!([permissions(
                json!([]),
                json!([]),
                json!(["Microsoft.Storage/storageAccounts/blobServices/containers/blobs/write"]),
                json!([])
            )]),
            PermissionVerdict::BroaderGrants,
        ),
        (
            json!([permissions(
                json!([]),
                json!([]),
                json!(["Microsoft.Storage/*/read"]),
                json!([])
            )]),
            PermissionVerdict::ReadOnly,
        ),
        (
            json!([permissions(
                json!([]),
                json!([]),
                json!(["Microsoft.Storage/*"]),
                json!(["Microsoft.Storage/*"])
            )]),
            PermissionVerdict::ReadOnly,
        ),
        (
            json!([permissions(
                json!(["Vendor.Service/*"]),
                json!(["Vendor.Service/widgets/write"]),
                json!([]),
                json!([])
            )]),
            PermissionVerdict::UnableToVerify,
        ),
        (
            json!([permissions(
                json!(["Vendor.Service/?/read"]),
                json!([]),
                json!([]),
                json!([])
            )]),
            PermissionVerdict::UnableToVerify,
        ),
        (
            json!([{"actions":["*/read"]}]),
            PermissionVerdict::UnableToVerify,
        ),
    ];
    for (blocks, expected) in cases {
        assert_eq!(classify_permissions(&blocks).0, expected, "{blocks}");
    }
}
#[test]
fn unit_applies_exclusions_only_inside_their_permission_block() {
    let excluded = permissions(json!(["*"]), json!(["*"]), json!([]), json!([]));
    let grant = permissions(
        json!(["Microsoft.Compute/virtualMachines/write"]),
        json!([]),
        json!([]),
        json!([]),
    );
    assert_eq!(
        classify_permissions(&json!([excluded, grant])).0,
        PermissionVerdict::BroaderGrants
    );
}
#[tokio::test]
async fn unit_checks_inherited_and_narrower_group_grants() {
    let server = MockServer::start().await;
    discovery(&server).await;
    let parent = "/providers/Microsoft.Management/managementGroups/parent";
    let child = format!("/subscriptions/{SUB}/resourceGroups/production");
    let mut group = assignment("group", "reader", &child);
    group["properties"]["principalType"] = json!("Group");
    group["properties"]["principalId"] = json!("group-principal");
    assignments(
        &server,
        json!([assignment("inherited", "reader", parent), group]),
    )
    .await;
    // An alarming name with read-only permissions is still a read-only grant.
    role(&server, "reader", "Owner in name only", json!([reader()])).await;
    let result = inspect_at(reqwest::Client::new(), &token(), &[], &server.uri())
        .await
        .unwrap();
    assert_eq!(result.verdict, PermissionVerdict::ReadOnly);
    assert_eq!(result.grants.len(), 2);
    assert!(result.issues.is_empty());
    assert!(result.grants.iter().any(|g| g.scope == parent));
    assert!(result.grants.iter().any(|g| g.scope == child));
}
#[tokio::test]
async fn unit_keeps_broader_grants_when_another_role_cannot_be_read() {
    let server = MockServer::start().await;
    discovery(&server).await;
    assignments(
        &server,
        json!([
            assignment("a", "writer", "/"),
            assignment("b", "missing", "/")
        ]),
    )
    .await;
    role(
        &server,
        "writer",
        "Reader in name only",
        json!([permissions(
            json!(["*"]),
            json!(["Microsoft.Authorization/*/Write"]),
            json!([]),
            json!([])
        )]),
    )
    .await;
    let result = inspect_at(
        reqwest::Client::new(),
        &token(),
        &[PRINCIPAL.into()],
        &server.uri(),
    )
    .await
    .unwrap();
    assert_eq!(result.verdict, PermissionVerdict::BroaderGrants);
    assert_eq!(result.inaccessible_subscriptions, [PRINCIPAL]);
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, AccessIssueKind::DefinitionReadFailed))
    );
}
#[tokio::test]
async fn unit_retains_page_evidence_when_later_assignment_pages_fail() {
    let server = MockServer::start().await;
    discovery(&server).await;
    Mock::given(path(format!("/subscriptions/{SUB}/providers/Microsoft.Authorization/roleAssignments")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[assignment("a","writer","/")],"nextLink":format!("{}/page-two",server.uri())}))).mount(&server).await;
    Mock::given(path("/page-two"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    role(
        &server,
        "writer",
        "Contributor",
        json!([permissions(json!(["*"]), json!([]), json!([]), json!([]))]),
    )
    .await;
    let result = inspect_at(reqwest::Client::new(), &token(), &[], &server.uri())
        .await
        .unwrap();
    assert_eq!(result.verdict, PermissionVerdict::BroaderGrants);
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, AccessIssueKind::AssignmentReadFailed))
    );
}
#[tokio::test]
async fn unit_follows_assignment_pagination_and_retries_throttling() {
    let server = MockServer::start().await;
    discovery(&server).await;
    Mock::given(path(format!("/subscriptions/{SUB}/providers/Microsoft.Authorization/roleAssignments")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[assignment("a","reader","/")],"nextLink":format!("{}/page-two",server.uri())}))).mount(&server).await;
    Mock::given(path("/page-two"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(path("/page-two"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"value":[assignment("b","reader","/narrow")]})),
        )
        .with_priority(2)
        .mount(&server)
        .await;
    role(&server, "reader", "Reader", json!([reader()])).await;
    let result = inspect_at(reqwest::Client::new(), &token(), &[], &server.uri())
        .await
        .unwrap();
    assert_eq!(result.verdict, PermissionVerdict::ReadOnly);
    assert_eq!(result.grants.len(), 2);
}
#[tokio::test]
async fn unit_marks_missing_identity_and_conditional_grants_unverifiable() {
    let server = MockServer::start().await;
    discovery(&server).await;
    let missing = inspect_at(
        reqwest::Client::new(),
        &StaticTokenProvider("opaque".into()),
        &[],
        &server.uri(),
    )
    .await
    .unwrap();
    assert_eq!(missing.verdict, PermissionVerdict::UnableToVerify);
    let mut conditional = assignment("a", "reader", "/");
    conditional["properties"]["condition"] = json!("unsupported condition");
    assignments(&server, json!([conditional])).await;
    role(&server, "reader", "Reader", json!([reader()])).await;
    let result = inspect_at(reqwest::Client::new(), &token(), &[], &server.uri())
        .await
        .unwrap();
    assert_eq!(result.verdict, PermissionVerdict::UnableToVerify);
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, AccessIssueKind::ConditionalGrant))
    );
}
#[tokio::test]
async fn unit_returns_authentication_failures_as_errors() {
    let server = MockServer::start().await;
    discovery(&server).await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_string("never-echo-this-body"))
        .mount(&server)
        .await;
    let error = inspect_at(reqwest::Client::new(), &token(), &[], &server.uri())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        azdocs::auth::diagnostics::DiagnosticError::Auth(_)
    ));
    assert!(!format!("{error:?}").contains("never-echo-this-body"));
}
