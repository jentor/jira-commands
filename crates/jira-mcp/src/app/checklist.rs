use jira_core::model::checklist::*;
use serde_json::Value;

use super::{shared::to_value, JiraApp};
use crate::{
    error::{AppError, AppResult},
    models::{
        ChecklistAppendArgs, ChecklistClearArgs, ChecklistHistoryArgs, ChecklistReplaceArgs,
        ChecklistTargetArgs, ChecklistTemplateApplyArgs, ChecklistTemplateCreateArgs,
        ChecklistTemplateDeleteArgs, ChecklistTemplateIdArgs, ChecklistTemplateListArgs,
        ChecklistTemplateUpdateArgs, ChecklistUpdateArgs,
    },
};

fn require_force(force: Option<bool>) -> AppResult<()> {
    if force != Some(true) {
        return Err(AppError::unsafe_operation(
            "This operation requires force=true",
        ));
    }
    Ok(())
}

impl JiraApp {
    pub async fn checklist_view(&self, args: ChecklistTargetArgs) -> AppResult<Value> {
        let mut result = self
            .build_client()?
            .get_smart_checklists(&args.issue_key)
            .await?;
        if let Some(id) = args.checklist_id {
            result.checklists.retain(|c| c.checklist_id == id);
            if result.checklists.is_empty() {
                return Err(AppError::validation(format!(
                    "Checklist {id} does not belong to {}",
                    args.issue_key
                )));
            }
        }
        to_value(result)
    }

    pub async fn checklist_history(&self, args: ChecklistHistoryArgs) -> AppResult<Value> {
        to_value(
            self.build_client()?
                .get_smart_checklist_history(&args.issue_key)
                .await?,
        )
    }

    pub async fn checklist_append(&self, args: ChecklistAppendArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let id = client
            .resolve_smart_checklist(&args.target.issue_key, args.target.checklist_id)
            .await?;
        to_value(client.write_smart_checklist(id, &args.text, false).await?)
    }

    pub async fn checklist_replace(&self, args: ChecklistReplaceArgs) -> AppResult<Value> {
        require_force(args.force)?;
        let client = self.build_client()?;
        let id = client
            .resolve_smart_checklist(&args.input.target.issue_key, args.input.target.checklist_id)
            .await?;
        to_value(
            client
                .write_smart_checklist(id, &args.input.text, true)
                .await?,
        )
    }

    pub async fn checklist_update(&self, args: ChecklistUpdateArgs) -> AppResult<Value> {
        let updates: Vec<_> = args
            .updates
            .into_iter()
            .map(|item| SmartChecklistItemUpdate {
                id: item.id,
                label: item.label,
                rank: item.rank,
                level: item.level,
                mandatory: item.mandatory,
                status: item.status_id.map(|id| SmartChecklistStatusId { id }),
            })
            .collect();
        let client = self.build_client()?;
        let id = client
            .resolve_smart_checklist(&args.target.issue_key, args.target.checklist_id)
            .await?;
        to_value(client.update_smart_checklist(id, &updates).await?)
    }

    pub async fn checklist_clear(&self, args: ChecklistClearArgs) -> AppResult<Value> {
        require_force(args.force)?;
        let client = self.build_client()?;
        let id = client
            .resolve_smart_checklist(&args.target.issue_key, args.target.checklist_id)
            .await?;
        to_value(client.clear_smart_checklist(id).await?)
    }

    pub async fn checklist_template_list(
        &self,
        args: ChecklistTemplateListArgs,
    ) -> AppResult<Value> {
        to_value(
            self.build_client()?
                .list_smart_checklist_templates(&SmartChecklistTemplateQuery {
                    project_id: args.project_id,
                    global: args.global.unwrap_or(false),
                    query: args.query,
                    order_by: args.order_by,
                    reversed: args.reversed.unwrap_or(false),
                    page: args.page,
                })
                .await?,
        )
    }

    pub async fn checklist_template_view(&self, args: ChecklistTemplateIdArgs) -> AppResult<Value> {
        to_value(
            self.build_client()?
                .get_smart_checklist_template(args.template_id)
                .await?,
        )
    }

    pub async fn checklist_template_fields(&self) -> AppResult<Value> {
        to_value(
            self.build_client()?
                .get_smart_checklist_template_fields()
                .await?,
        )
    }

    pub async fn checklist_template_create(
        &self,
        args: ChecklistTemplateCreateArgs,
    ) -> AppResult<Value> {
        let request = serde_json::from_value::<SmartChecklistTemplateRequest>(args.template)
            .map_err(|e| AppError::validation(format!("Invalid template JSON: {e}")))?;
        to_value(
            self.build_client()?
                .create_smart_checklist_template(&request)
                .await?,
        )
    }

    pub async fn checklist_template_update(
        &self,
        args: ChecklistTemplateUpdateArgs,
    ) -> AppResult<Value> {
        let request = serde_json::from_value::<SmartChecklistTemplateRequest>(args.template)
            .map_err(|e| AppError::validation(format!("Invalid template JSON: {e}")))?;
        to_value(
            self.build_client()?
                .update_smart_checklist_template(args.template_id, &request)
                .await?,
        )
    }

    pub async fn checklist_template_delete(
        &self,
        args: ChecklistTemplateDeleteArgs,
    ) -> AppResult<Value> {
        require_force(args.force)?;
        Ok(self
            .build_client()?
            .delete_smart_checklist_template(
                args.template_id,
                args.project_id,
                args.page.unwrap_or(1),
            )
            .await?)
    }

    pub async fn checklist_template_apply(
        &self,
        args: ChecklistTemplateApplyArgs,
    ) -> AppResult<Value> {
        let client = self.build_client()?;
        let id = client
            .resolve_smart_checklist(&args.target.issue_key, args.target.checklist_id)
            .await?;
        Ok(client
            .apply_smart_checklist_template(id, args.template_id)
            .await?)
    }
}
