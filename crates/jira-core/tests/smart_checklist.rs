use jira_core::{
    config::{JiraAuthType, JiraConfig, JiraDeployment},
    model::checklist::*,
    JiraClient, JiraError,
};
use serde_json::{json, Value};
use wiremock::{
    matchers::{body_json, header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/smart-checklist.json")).unwrap()
}

fn client(server: &MockServer) -> JiraClient {
    JiraClient::new(JiraConfig {
        base_url: format!("{}/jira/", server.uri()),
        token: Some("pat-test".into()),
        deployment: JiraDeployment::DataCenter,
        auth_type: JiraAuthType::DataCenterPat,
        api_version: 2,
        smart_checklist_enabled: true,
        ..JiraConfig::default()
    })
}

#[tokio::test]
async fn disabled_feature_blocks_read_write_history_and_templates_without_http() {
    let server = MockServer::start().await;
    let config = JiraConfig {
        base_url: server.uri(),
        token: Some("pat-test".into()),
        ..Default::default()
    };
    assert!(!config.smart_checklist_enabled);
    let c = JiraClient::new(config);
    for error in [
        c.get_smart_checklists("PROJ-123").await.unwrap_err(),
        c.write_smart_checklist(42, "- Review", false)
            .await
            .unwrap_err(),
        c.clear_smart_checklist(42).await.unwrap_err(),
        c.get_smart_checklist_history("PROJ-123").await.unwrap_err(),
        c.get_smart_checklist_template(2).await.unwrap_err(),
        c.apply_smart_checklist_template(42, 2).await.unwrap_err(),
    ] {
        assert!(error.to_string().contains("smart_checklist_enabled true"));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn read_preserves_plugin_fields_and_encodes_issue_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/jira/rest/railsware/1.0/checklist"))
        .and(query_param("issueKey", "PROJ-1&other=value"))
        .and(header("authorization", "Bearer pat-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture()))
        .expect(1)
        .mount(&server)
        .await;
    let result = client(&server)
        .get_smart_checklists("PROJ-1&other=value")
        .await
        .unwrap();
    let c = &result.checklists[0];
    assert_eq!(c.items[0].item_type, "HEADING");
    assert!(c.items[0].status.is_none());
    assert_eq!(c.items[1].item_type, "item");
    assert_eq!(c.items[1].status.as_ref().unwrap().id, 1);
    assert_eq!(c.items[1].assignees[0].user_name, "developer");
    let output = serde_json::to_value(result).unwrap();
    assert_eq!(output["checklists"][0]["futureField"], json!({"keep":true}));
    assert_eq!(output["checklists"][0]["items"][1]["history"]["id"], 7);
    assert_eq!(
        output["checklists"][0]["items"][1]["status"]["color"],
        "GRAY"
    );
    assert_eq!(
        output["checklists"][0]["items"][1]["quotes"][0]["value"],
        "Explanation"
    );
}

#[tokio::test]
async fn resolve_rejects_missing_ambiguous_and_foreign_checklists() {
    let server = MockServer::start().await;
    for (key, data) in [
        ("ONE", fixture()),
        ("NONE", json!({"checklists":[]})),
        ("MANY", {
            let mut data = fixture();
            let mut other = data["checklists"][0].clone();
            other["checklistId"] = json!(7777);
            data["checklists"].as_array_mut().unwrap().push(other);
            data
        }),
    ] {
        Mock::given(method("GET"))
            .and(path("/jira/rest/railsware/1.0/checklist"))
            .and(query_param("issueKey", key))
            .respond_with(ResponseTemplate::new(200).set_body_json(data))
            .mount(&server)
            .await;
    }
    let c = client(&server);
    assert_eq!(c.resolve_smart_checklist("ONE", None).await.unwrap(), 42);
    assert_eq!(
        c.resolve_smart_checklist("MANY", Some(7777)).await.unwrap(),
        7777
    );
    assert!(c
        .resolve_smart_checklist("NONE", None)
        .await
        .unwrap_err()
        .to_string()
        .contains("No Smart Checklist"));
    assert!(c
        .resolve_smart_checklist("MANY", None)
        .await
        .unwrap_err()
        .to_string()
        .contains("Multiple"));
    assert!(c
        .resolve_smart_checklist("ONE", Some(7777))
        .await
        .unwrap_err()
        .to_string()
        .contains("does not belong"));
}

#[tokio::test]
async fn updates_are_sparse_and_text_is_passed_unchanged() {
    let server = MockServer::start().await;
    Mock::given(method("PUT")).and(path("/jira/rest/railsware/1.0/checklist/42"))
        .and(body_json(json!([{ "id":102, "status":{"id":37}, "mandatory":false }, {"id":101,"label":"New heading","rank":2,"level":2}])))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture())).expect(1).mount(&server).await;
    let c = client(&server);
    let updates = serde_json::from_value::<Vec<SmartChecklistItemUpdate>>(json!([
        {"id":102,"status":{"id":37},"mandatory":false}, {"id":101,"label":"New heading","rank":2,"level":2}
    ])).unwrap();
    c.update_smart_checklist(42, &updates).await.unwrap();
    let text = "# Heading\n- [Document](https://example.com) @developer\n> * explanation";
    for replace in [false, true] {
        Mock::given(method("PUT"))
            .and(path("/jira/rest/railsware/1.0/checklist/42/item"))
            .and(body_json(json!({"stringValue":text,"isReplace":replace})))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture()))
            .expect(1)
            .mount(&server)
            .await;
        c.write_smart_checklist(42, text, replace).await.unwrap();
    }
    Mock::given(method("DELETE"))
        .and(path("/jira/rest/railsware/1.0/checklist/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"checklists":[]})))
        .expect(1)
        .mount(&server)
        .await;
    assert!(c
        .clear_smart_checklist(42)
        .await
        .unwrap()
        .checklists
        .is_empty());
}

#[tokio::test]
async fn invalid_updates_and_template_inputs_never_send_requests() {
    let server = MockServer::start().await;
    let c = client(&server);
    assert!(c.update_smart_checklist(42, &[]).await.is_err());
    assert!(c
        .update_smart_checklist(
            42,
            &[SmartChecklistItemUpdate {
                id: 102,
                ..Default::default()
            }]
        )
        .await
        .is_err());
    assert!(c
        .update_smart_checklist(
            42,
            &[SmartChecklistItemUpdate {
                id: 102,
                status: Some(SmartChecklistStatusId { id: 0 }),
                ..Default::default()
            }]
        )
        .await
        .is_err());
    assert!(c.write_smart_checklist(42, " \n", true).await.is_err());
    assert!(c.clear_smart_checklist(0).await.is_err());
    assert!(
        serde_json::from_value::<SmartChecklistItemUpdate>(json!({"id":102,"typo":true})).is_err()
    );
    let request = serde_json::from_value(json!({"name":"","scope":{}})).unwrap();
    assert!(c.create_smart_checklist_template(&request).await.is_err());
    let request = serde_json::from_value(json!({"name":"Test","scope":null})).unwrap();
    assert!(c.create_smart_checklist_template(&request).await.is_err());
    assert!(c
        .list_smart_checklist_templates(&SmartChecklistTemplateQuery {
            page: Some(0),
            ..Default::default()
        })
        .await
        .is_err());
    assert!(c
        .list_smart_checklist_templates(&SmartChecklistTemplateQuery {
            order_by: Some("unknown".into()),
            ..Default::default()
        })
        .await
        .is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn history_and_template_operations_use_public_endpoints() {
    let server = MockServer::start().await;
    let c = client(&server);
    Mock::given(method("GET")).and(path("/jira/rest/railsware/1.0/history")).and(query_param("issueKey","PROJ-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":7,"category":"edited","createdAt":1700000000000_i64,"from":[],"to":[{"text":"Updated","mandatory":true}],"performerName":"Developer","future":42}]))).expect(1).mount(&server).await;
    let history = c.get_smart_checklist_history("PROJ-1").await.unwrap();
    assert_eq!(history[0].to[0]["mandatory"], true);
    assert_eq!(history[0].extra["future"], 42);
    let template = json!({"id":2,"name":"Release","value":"- Check\n","enabled":true,"scope":{"type":0,"values":[]}});
    Mock::given(method("GET"))
        .and(path("/jira/rest/railsware/1.0/template/2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&template))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        c.get_smart_checklist_template(2).await.unwrap().extra["scope"]["type"],
        0
    );
    Mock::given(method("GET"))
        .and(path("/jira/rest/railsware/1.0/template/fields"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!([{"id":"customfield_54321","type":"SINGLE_LINE_TEXT","canBeEmpty":true}]),
        ))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        c.get_smart_checklist_template_fields().await.unwrap()[0]["id"],
        "customfield_54321"
    );
    let body = json!({"name":"Release","value":"- Check\n","enabled":false,"scope":{"type":2,"values":["10000"]},"projectId":10000,"conditions":[{"field":{"id":"priority","type":"PRIORITY"},"option":"GREATER_THAN","values":["2"]}],"trigger":{"type":2,"from":["4"],"to":["5"],"preventDuplicates":true},"modificationType":"APPEND"});
    let request: SmartChecklistTemplateRequest = serde_json::from_value(body.clone()).unwrap();
    for (verb, suffix) in [("POST", "/template"), ("PUT", "/template/2")] {
        Mock::given(method(verb))
            .and(path(format!("/jira/rest/railsware/1.0{suffix}")))
            .and(body_json(body.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(&template))
            .expect(1)
            .mount(&server)
            .await;
    }
    assert_eq!(
        c.create_smart_checklist_template(&request)
            .await
            .unwrap()
            .id,
        2
    );
    assert_eq!(
        c.update_smart_checklist_template(2, &request)
            .await
            .unwrap()
            .id,
        2
    );
    Mock::given(method("DELETE"))
        .and(path("/jira/rest/railsware/1.0/template/2"))
        .and(query_param("projectId", "10000"))
        .and(query_param("page", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"templates":[]})))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        c.delete_smart_checklist_template(2, Some(10000), 3)
            .await
            .unwrap()["templates"],
        json!([])
    );
    Mock::given(method("POST"))
        .and(path("/jira/rest/railsware/1.0/checklist/42/template/2"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        c.apply_smart_checklist_template(42, 2).await.unwrap(),
        Value::Null
    );
}

#[tokio::test]
async fn template_pagination_and_project_query() {
    let server = MockServer::start().await;
    for page in 1..=2 {
        Mock::given(method("GET")).and(path("/jira/rest/railsware/1.0/template/global"))
            .and(query_param("page",page.to_string())).and(query_param("query","Release & deploy"))
            .and(query_param("orderBy","name")).and(query_param("reversed","true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"totalPages":2,"templates":[{"id":page,"name":"Release","value":"- Check"}]}))).expect(1).mount(&server).await;
    }
    let c = client(&server);
    let result = c
        .list_smart_checklist_templates(&SmartChecklistTemplateQuery {
            query: Some("Release & deploy".into()),
            order_by: Some("name".into()),
            reversed: true,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        result.templates.iter().map(|t| t.id).collect::<Vec<_>>(),
        vec![1, 2]
    );
    for global in [true, false] {
        Mock::given(method("GET"))
            .and(path("/jira/rest/railsware/1.0/template/project/10000"))
            .and(query_param("global", global.to_string()))
            .and(query_param("page", "3"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"totalPages":8,"templates":[]})),
            )
            .expect(1)
            .mount(&server)
            .await;
        c.list_smart_checklist_templates(&SmartChecklistTemplateQuery {
            project_id: Some(10000),
            global,
            page: Some(3),
            ..Default::default()
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn plugin_http_errors_keep_status_and_body() {
    let server = MockServer::start().await;
    for status in [400, 401, 403, 404, 500] {
        Mock::given(method("GET"))
            .and(path(format!("/jira/rest/railsware/1.0/template/{status}")))
            .respond_with(ResponseTemplate::new(status).set_body_string("Plugin error details"))
            .expect(1)
            .mount(&server)
            .await;
        let err = client(&server)
            .get_smart_checklist_template(status.into())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Plugin error details"));
        match status {
            401 | 403 => assert!(matches!(err, JiraError::Auth(_))),
            404 => assert!(matches!(err, JiraError::NotFound(_))),
            _ => assert!(matches!(err,JiraError::Api {status:s,..} if s == status)),
        }
    }
}

#[tokio::test]
async fn plugin_rate_limit_uses_existing_retry_transport() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/jira/rest/railsware/1.0/checklist"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jira/rest/railsware/1.0/checklist"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture()))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .get_smart_checklists("PROJ-1")
        .await
        .unwrap();
}
