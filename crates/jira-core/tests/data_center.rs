use jira_core::{
    config::{JiraAuthType, JiraConfig, JiraDeployment},
    model::{CreateIssueRequestV2, UpdateIssueRequest},
    JiraClient,
};
use serde_json::{json, Value};
use wiremock::{
    matchers::{body_json, header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

fn client(server: &MockServer, deployment: JiraDeployment) -> JiraClient {
    let dc = deployment == JiraDeployment::DataCenter;
    JiraClient::new(JiraConfig {
        base_url: format!("{}/jira/", server.uri()),
        token: Some("test-token".into()),
        api_version: if dc { 2 } else { 3 },
        auth_type: if dc {
            JiraAuthType::DataCenterPat
        } else {
            JiraAuthType::CloudApiToken
        },
        deployment,
        ..JiraConfig::default()
    })
}

fn issue(key: &str) -> Value {
    json!({"id":"1", "key":key, "fields":{"summary":key, "description":"Text [~alice]", "project":{"key":"IT"}}})
}

#[tokio::test]
async fn data_center_search_follows_actual_page_size_and_stops_at_total() {
    let server = MockServer::start().await;
    for (offset, key) in [(0, "IT-1"), (1, "IT-2")] {
        Mock::given(method("POST"))
            .and(path("/jira/rest/api/2/search"))
            .and(header("authorization", "Bearer test-token"))
            .and(wiremock::matchers::body_partial_json(
                json!({"startAt":offset,"maxResults":100,"jql":"project = IT"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "startAt":offset,"maxResults":1,"total":2,"issues":[issue(key)]
            })))
            .expect(1)
            .mount(&server)
            .await;
    }
    let issues = client(&server, JiraDeployment::DataCenter)
        .get_all_issues("project = IT")
        .await
        .unwrap();
    assert_eq!(
        issues.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(),
        ["IT-1", "IT-2"]
    );
    assert_eq!(issues[0].description, Some(json!("Text [~alice]")));
}

#[tokio::test]
async fn data_center_search_empty_page_stops_and_invalid_token_never_sends() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/jira/rest/api/2/search"))
        .and(body_json(
            json!({"startAt":7,"maxResults":3,"jql":"x","fields":["summary"]}),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"startAt":7,"total":20,"issues":[]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server, JiraDeployment::DataCenter);
    let result = c
        .search_issues_with_fields("x", Some("7"), Some(3), &["summary"])
        .await
        .unwrap();
    assert!(result.issues.is_empty());
    assert_eq!(result.next_page_token, None);
    assert_eq!(result.total, Some(20));
    assert!(c
        .search_issues("x", Some("not-an-offset"), None)
        .await
        .unwrap_err()
        .to_string()
        .contains("continuation token"));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn cloud_search_keeps_opaque_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/jira/rest/api/3/search/jql"))
        .and(body_json(
            json!({"nextPageToken":"opaque", "maxResults":2,"jql":"x","fields":["summary"]}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"nextPageToken":"next", "issues":[issue("IT-1")]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server, JiraDeployment::Cloud);
    let result = c
        .search_issues_with_fields("x", Some("opaque"), Some(2), &["summary"])
        .await
        .unwrap();
    assert_eq!(result.issues[0].key, "IT-1");
    assert_eq!(result.next_page_token.as_deref(), Some("next"));
}

#[tokio::test]
async fn data_center_metadata_reads_all_values_pages() {
    let server = MockServer::start().await;
    for offset in 0..2 {
        Mock::given(method("GET"))
            .and(path("/jira/rest/api/2/issue/createmeta/IT/issuetypes"))
            .and(query_param("startAt", offset.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "startAt":offset,"total":2,"isLast":offset==1,
                "values":[{"id":offset.to_string(),"name":format!("Type {offset}"),"subtask":false}]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET")).and(path("/jira/rest/api/2/issue/createmeta/IT/issuetypes/0"))
            .and(query_param("startAt",offset.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "startAt":offset,"isLast":offset==1,"values":[{"fieldId":format!("field{offset}"),"name":"Field","required":true,"schema":{"type":"string"}}]
            }))).expect(1).mount(&server).await;
    }
    let c = client(&server, JiraDeployment::DataCenter);
    assert_eq!(
        c.get_issue_types("IT")
            .await
            .unwrap()
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["0", "1"]
    );
    let fields = c.get_fields_for_issue_type("IT", "0").await.unwrap();
    assert_eq!(
        fields.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
        ["field0", "field1"]
    );
    assert!(fields[1].required);
    assert_eq!(fields[1].field_type, "string");
}

#[tokio::test]
async fn data_center_identity_users_and_watchers_use_names() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/jira/rest/api/2/myself"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"name":"alice","key":"JIRAUSER1","timeZone":"Europe/Moscow"}),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jira/rest/api/2/user/search"))
        .and(query_param("username", "alice@example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!([{"name":"alice","displayName":"Alice","emailAddress":"alice@example.com"}]),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jira/rest/api/2/issue/IT-1/watchers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"watchCount":1,"watchers":[{"name":"alice","displayName":"Alice"}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/jira/rest/api/2/issue/IT-1/watchers"))
        .and(query_param("username", "alice"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server, JiraDeployment::DataCenter);
    assert_eq!(c.get_myself().await.unwrap(), "alice");
    assert_eq!(
        c.search_users("alice@example.com").await.unwrap()[0].account_id,
        "alice"
    );
    assert_eq!(
        c.list_watchers("IT-1").await.unwrap().watchers[0].account_id,
        "alice"
    );
    c.remove_watcher("IT-1", "alice").await.unwrap();
}

#[tokio::test]
async fn data_center_create_update_comment_and_worklog_send_strings() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/jira/rest/api/2/issue"))
        .and(body_json(json!({"fields":{"project":{"key":"IT"},"issuetype":{"name":"Task"},"summary":"Test","description":"Description","assignee":{"name":"alice"}}})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"1","key":"IT-1"})))
        .expect(1).mount(&server).await;
    Mock::given(method("GET"))
        .and(path("/jira/rest/api/2/issue/IT-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issue("IT-1")))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/jira/rest/api/2/issue/IT-1"))
        .and(body_json(
            json!({"fields":{"description":"Changed","assignee":{"name":"alice"}}}),
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/jira/rest/api/2/issue/IT-1/comment"))
        .and(body_json(json!({"body":"Comment [~alice]"})))
        .respond_with(
            ResponseTemplate::new(201)
                .set_body_json(json!({"id":"1","body":"Comment [~alice]","author":{"name":"bob"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/jira/rest/api/2/issue/IT-1/worklog"))
        .and(body_json(
            json!({"timeSpent":"1h","started":"2026-10-15T10:00:00.000+0300","comment":"Work"}),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(
            json!({"id":"1","timeSpent":"1h","timeSpentSeconds":3600,"comment":"Work"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server, JiraDeployment::DataCenter);
    let created = c
        .create_issue_v2(CreateIssueRequestV2 {
            project_key: "IT".into(),
            summary: "Test".into(),
            issue_type: "Task".into(),
            description: Some("Description".into()),
            assignee: Some("alice".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(created.key, "IT-1");
    c.update_issue(
        "IT-1",
        UpdateIssueRequest {
            description: Some("Changed".into()),
            assignee: Some("alice".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let comment = c.add_comment("IT-1", "Comment [~alice]").await.unwrap();
    assert_eq!(comment.body.as_deref(), Some("Comment [~alice]"));
    assert_eq!(comment.mentions, ["alice"]);
    assert_eq!(comment.author_account_id.as_deref(), Some("bob"));
    let log = c
        .add_worklog(
            "IT-1",
            "1h",
            Some("Work"),
            Some("2026-10-15T10:00:00.000+0300"),
        )
        .await
        .unwrap();
    assert_eq!(log.time_spent_seconds, 3600);
    assert_eq!(log.comment.as_deref(), Some("Work"));
}

#[tokio::test]
async fn data_center_adf_input_becomes_text_and_read_pages_are_complete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/jira/rest/api/2/issue/IT-1/comment"))
        .and(body_json(json!({"body":"Hello [~alice]"})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({"id":"1","body":"Hello [~alice]"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    for field in ["comment", "worklog"] {
        let array = if field == "comment" {
            "comments"
        } else {
            "worklogs"
        };
        for offset in 0..2 {
            Mock::given(method("GET")).and(path(format!("/jira/rest/api/2/issue/IT-1/{field}")))
                .and(query_param("startAt",offset.to_string()))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"startAt":offset,"total":2,(array):[{"id":offset.to_string(),"body":"Text"}]})))
                .expect(1).mount(&server).await;
        }
    }
    let c = client(&server, JiraDeployment::DataCenter);
    let comment = c
        .add_comment_adf("IT-1", {
            let mut adf = jira_core::adf::markdown_to_adf("Hello @Alice");
            jira_core::adf::inject_mentions(&mut adf, &[("Alice".into(), "alice".into())]);
            adf
        })
        .await
        .unwrap();
    assert_eq!(comment.body.as_deref(), Some("Hello [~alice]"));
    assert_eq!(
        c.get_comments("IT-1")
            .await
            .unwrap()
            .iter()
            .map(|v| v.id.as_str())
            .collect::<Vec<_>>(),
        ["0", "1"]
    );
    assert_eq!(
        c.get_worklogs("IT-1")
            .await
            .unwrap()
            .iter()
            .map(|v| v.id.as_str())
            .collect::<Vec<_>>(),
        ["0", "1"]
    );
}

#[tokio::test]
async fn data_center_archive_uses_per_issue_endpoint_and_cloud_only_commands_fail_locally() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/jira/rest/api/2/issue/IT-1/archive"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server, JiraDeployment::DataCenter);
    c.archive_issues(&["IT-1".into()]).await.unwrap();
    assert!(c
        .move_issue("IT-1", "IT", "1", None)
        .await
        .unwrap_err()
        .to_string()
        .contains("Data Center"));
    assert!(c
        .get_plans()
        .await
        .unwrap_err()
        .to_string()
        .contains("Cloud"));
    assert!(!c.is_premium().await);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[test]
fn data_center_wiki_text_and_mentions_remain_readable() {
    let text = json!("Hello [~alice], [~bob], [~alice]. Empty [~] and broken [~user");
    assert_eq!(jira_core::adf::adf_to_text(&text), text.as_str().unwrap());
    assert_eq!(
        jira_core::adf::mentioned_account_ids(&text),
        ["alice", "bob"]
    );
}
