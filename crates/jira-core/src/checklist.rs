//! Native Smart Checklist Data Center operations using the public Railsware API.
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::{model::checklist::*, JiraClient, JiraError, Result};

fn invalid(message: impl Into<String>) -> JiraError {
    JiraError::Api {
        status: 0,
        message: message.into(),
    }
}

fn positive(id: u64, name: &str) -> Result<()> {
    if id == 0 {
        return Err(invalid(format!("{name} must be positive")));
    }
    Ok(())
}

fn decode<T: DeserializeOwned>(value: Value) -> Result<T> {
    Ok(serde_json::from_value(value)?)
}

impl JiraClient {
    async fn smart_checklist_request(
        &self,
        method: &str,
        suffix: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<Value> {
        self.ensure_smart_checklist_enabled()?;
        let path = format!("/rest/railsware/1.0{suffix}");
        let mut url = reqwest::Url::parse("http://localhost/").expect("static URL");
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        }
        let path = match url.query() {
            Some(query) => format!("{path}?{query}"),
            None => path,
        };
        Ok(self
            .raw_request(method, &path, body)
            .await?
            .unwrap_or(Value::Null))
    }

    /// Public API calls only address the Default Checklist tab.
    pub async fn get_smart_checklists(&self, issue_key: &str) -> Result<SmartChecklists> {
        if issue_key.trim().is_empty() {
            return Err(invalid("issue_key must not be empty"));
        }
        decode(
            self.smart_checklist_request(
                "GET",
                "/checklist",
                &[("issueKey", issue_key.into())],
                None,
            )
            .await?,
        )
    }

    /// Resolve an issue's checklist; never silently pick among multiple results.
    pub async fn resolve_smart_checklist(
        &self,
        issue_key: &str,
        checklist_id: Option<u64>,
    ) -> Result<u64> {
        let checklists = self.get_smart_checklists(issue_key).await?.checklists;
        if let Some(id) = checklist_id {
            if checklists.iter().any(|c| c.checklist_id == id) {
                return Ok(id);
            }
            return Err(invalid(format!(
                "Checklist {id} does not belong to {issue_key}"
            )));
        }
        match checklists.as_slice() {
            [checklist] => Ok(checklist.checklist_id),
            [] => Err(invalid(format!("No Smart Checklist found for {issue_key}"))),
            _ => Err(invalid(
                "Multiple checklists returned; provide checklist_id (--checklist-id)",
            )),
        }
    }

    pub async fn update_smart_checklist(
        &self,
        checklist_id: u64,
        updates: &[SmartChecklistItemUpdate],
    ) -> Result<SmartChecklists> {
        positive(checklist_id, "checklist_id")?;
        if updates.is_empty() {
            return Err(invalid("Provide at least one item update"));
        }
        for item in updates {
            positive(item.id, "item_id")?;
            if let Some(status) = &item.status {
                positive(status.id, "status_id")?;
            }
            if item.level == Some(0) {
                return Err(invalid("level must be positive"));
            }
            if item.rank.is_none()
                && item.label.is_none()
                && item.level.is_none()
                && item.mandatory.is_none()
                && item.status.is_none()
            {
                return Err(invalid("Provide at least one field for each item update"));
            }
        }
        decode(
            self.smart_checklist_request(
                "PUT",
                &format!("/checklist/{checklist_id}"),
                &[],
                Some(serde_json::to_value(updates)?),
            )
            .await?,
        )
    }

    /// Pass Smart Checklist text directly, without Jira Wiki/ADF conversion.
    pub async fn write_smart_checklist(
        &self,
        checklist_id: u64,
        text: &str,
        replace: bool,
    ) -> Result<SmartChecklists> {
        positive(checklist_id, "checklist_id")?;
        if text.trim().is_empty() {
            return Err(invalid(
                "Checklist text must not be empty; use clear to remove all items",
            ));
        }
        decode(
            self.smart_checklist_request(
                "PUT",
                &format!("/checklist/{checklist_id}/item"),
                &[],
                Some(json!({"stringValue": text, "isReplace": replace})),
            )
            .await?,
        )
    }

    pub async fn clear_smart_checklist(&self, checklist_id: u64) -> Result<SmartChecklists> {
        positive(checklist_id, "checklist_id")?;
        decode(
            self.smart_checklist_request(
                "DELETE",
                &format!("/checklist/{checklist_id}"),
                &[],
                None,
            )
            .await?,
        )
    }

    /// Requires Smart Checklist v6.5.0 or newer.
    pub async fn get_smart_checklist_history(
        &self,
        issue_key: &str,
    ) -> Result<Vec<SmartChecklistHistory>> {
        if issue_key.trim().is_empty() {
            return Err(invalid("issue_key must not be empty"));
        }
        decode(
            self.smart_checklist_request(
                "GET",
                "/history",
                &[("issueKey", issue_key.into())],
                None,
            )
            .await?,
        )
    }

    pub async fn list_smart_checklist_templates(
        &self,
        options: &SmartChecklistTemplateQuery,
    ) -> Result<SmartChecklistTemplatePage> {
        if options.page == Some(0) {
            return Err(invalid("page must be positive"));
        }
        if let Some(id) = options.project_id {
            positive(id, "project_id")?;
        }
        if let Some(order) = &options.order_by {
            if !["name", "enabled", "issueTypes", "projects"].contains(&order.as_str()) {
                return Err(invalid(
                    "order_by must be name, enabled, issueTypes, or projects",
                ));
            }
        }
        let suffix = options
            .project_id
            .map(|id| format!("/template/project/{id}"))
            .unwrap_or_else(|| "/template/global".into());
        let mut combined: Option<SmartChecklistTemplatePage> = None;
        let start = options.page.unwrap_or(1);
        for offset in 0..500 {
            let page = start
                .checked_add(offset)
                .ok_or_else(|| invalid("page is too large"))?;
            let mut query = vec![
                ("page", page.to_string()),
                ("reversed", options.reversed.to_string()),
            ];
            if options.project_id.is_some() {
                query.push(("global", options.global.to_string()));
            }
            if let Some(value) = &options.query {
                query.push(("query", value.clone()));
            }
            if let Some(value) = &options.order_by {
                query.push(("orderBy", value.clone()));
            }
            let result: SmartChecklistTemplatePage = decode(
                self.smart_checklist_request("GET", &suffix, &query, None)
                    .await?,
            )?;
            let last = options.page.is_some() || page >= result.total_pages;
            if let Some(combined) = &mut combined {
                combined.total_pages = result.total_pages;
                combined.templates.extend(result.templates);
            } else {
                combined = Some(result);
            }
            if last {
                return Ok(combined.expect("first page was fetched"));
            }
        }
        Err(invalid(
            "Smart Checklist template pagination exceeded 500 pages",
        ))
    }

    pub async fn get_smart_checklist_template(
        &self,
        template_id: u64,
    ) -> Result<SmartChecklistTemplate> {
        positive(template_id, "template_id")?;
        decode(
            self.smart_checklist_request("GET", &format!("/template/{template_id}"), &[], None)
                .await?,
        )
    }

    pub async fn get_smart_checklist_template_fields(&self) -> Result<Vec<Value>> {
        decode(
            self.smart_checklist_request("GET", "/template/fields", &[], None)
                .await?,
        )
    }

    pub async fn create_smart_checklist_template(
        &self,
        template: &SmartChecklistTemplateRequest,
    ) -> Result<SmartChecklistTemplate> {
        validate_template(template)?;
        decode(
            self.smart_checklist_request(
                "POST",
                "/template",
                &[],
                Some(serde_json::to_value(template)?),
            )
            .await?,
        )
    }

    pub async fn update_smart_checklist_template(
        &self,
        template_id: u64,
        template: &SmartChecklistTemplateRequest,
    ) -> Result<SmartChecklistTemplate> {
        positive(template_id, "template_id")?;
        validate_template(template)?;
        decode(
            self.smart_checklist_request(
                "PUT",
                &format!("/template/{template_id}"),
                &[],
                Some(serde_json::to_value(template)?),
            )
            .await?,
        )
    }

    /// Return the plugin response unchanged (some versions return a template list).
    pub async fn delete_smart_checklist_template(
        &self,
        template_id: u64,
        project_id: Option<u64>,
        page: u32,
    ) -> Result<Value> {
        positive(template_id, "template_id")?;
        if page == 0 {
            return Err(invalid("page must be positive"));
        }
        let mut query = vec![("page", page.to_string())];
        if let Some(id) = project_id {
            positive(id, "project_id")?;
            query.push(("projectId", id.to_string()));
        }
        self.smart_checklist_request("DELETE", &format!("/template/{template_id}"), &query, None)
            .await
    }

    pub async fn apply_smart_checklist_template(
        &self,
        checklist_id: u64,
        template_id: u64,
    ) -> Result<Value> {
        positive(checklist_id, "checklist_id")?;
        positive(template_id, "template_id")?;
        self.smart_checklist_request(
            "POST",
            &format!("/checklist/{checklist_id}/template/{template_id}"),
            &[],
            None,
        )
        .await
    }
}

fn validate_template(template: &SmartChecklistTemplateRequest) -> Result<()> {
    if template.name.trim().is_empty() {
        return Err(invalid("Template name must not be empty"));
    }
    if !template.scope.is_object()
        || template.trigger.as_ref().is_some_and(|v| !v.is_object())
        || template
            .conditions
            .as_ref()
            .is_some_and(|v| v.iter().any(|c| !c.is_object()))
    {
        return Err(invalid(
            "scope, trigger and conditions must contain JSON objects",
        ));
    }
    if let Some(id) = template.project_id {
        positive(id, "project_id")?;
    }
    Ok(())
}
