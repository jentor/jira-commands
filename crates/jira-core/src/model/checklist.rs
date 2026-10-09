//! Smart Checklist Data Center public REST API models.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklists {
    pub checklists: Vec<SmartChecklist>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklist {
    pub checklist_id: u64,
    pub issue_id: u64,
    #[serde(default)]
    pub items: Vec<SmartChecklistItem>,
    #[serde(default)]
    pub mentioned_users: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklistItem {
    pub id: u64,
    pub label: String,
    pub rank: u32,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<SmartChecklistStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    #[serde(default)]
    pub mandatory: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_checkbox: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default)]
    pub quotes: Vec<SmartChecklistQuote>,
    #[serde(default)]
    pub assignees: Vec<SmartChecklistAssignee>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklistStatus {
    pub id: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_state: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartChecklistQuote {
    pub id: u64,
    pub rank: u32,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub label: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklistAssignee {
    #[serde(default)]
    pub user_name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklistHistory {
    pub id: u64,
    pub category: String,
    pub created_at: i64,
    #[serde(default)]
    pub from: Vec<Value>,
    #[serde(default)]
    pub to: Vec<Value>,
    #[serde(default)]
    pub performer_name: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmartChecklistItemUpdate {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mandatory: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SmartChecklistStatusId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmartChecklistStatusId {
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklistTemplate {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklistTemplatePage {
    pub total_pages: u32,
    pub templates: Vec<SmartChecklistTemplate>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Nested scope, condition and trigger objects follow the plugin's JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartChecklistTemplateRequest {
    pub name: String,
    pub scope: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Value>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default)]
pub struct SmartChecklistTemplateQuery {
    pub project_id: Option<u64>,
    /// Include global templates applicable to the project instead of local templates.
    pub global: bool,
    pub query: Option<String>,
    pub order_by: Option<String>,
    pub reversed: bool,
    /// None collects all pages (maximum 500); Some fetches just that page.
    pub page: Option<u32>,
}
