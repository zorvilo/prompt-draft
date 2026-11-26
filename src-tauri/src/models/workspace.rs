use crate::models::provider::Profile;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatContext {
    pub(crate) source: String,
    pub(crate) template_id: String,
    pub(crate) template_name: String,
    pub(crate) system_prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) current_draft: Option<String>,
    pub(crate) mode: crate::models::conversation::ModificationMode,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatMessage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) context_warning_shown: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) context_limit_warning_shown: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
    pub(crate) id: String,
    pub(crate) role: crate::models::conversation::ConversationRole,
    pub(crate) content: String,
    pub(crate) created: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) display_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) context: Option<ChatContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) provider_name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    #[serde(default)]
    pub(crate) template_id: Option<String>,
    #[serde(default)]
    pub(crate) provider_id: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
    pub(crate) id: String,
    pub(crate) project_id: Option<String>,
    pub(crate) title: String,
    pub(crate) source: String,
    pub(crate) result: String,
    pub(crate) favorite: bool,
    #[serde(default)]
    pub(crate) pinned: bool,
    pub(crate) opened: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) updated: Option<u64>,
    pub(crate) messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) dialogue: Option<DialogueState>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DialogueState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mode: Option<crate::models::conversation::ModificationMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) chat_input: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    #[serde(default)]
    pub(crate) default_template_id: Option<String>,
    pub(crate) schema: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) templates: Option<Vec<crate::models::conversation::PromptTemplate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) template_categories: Option<Vec<Project>>,
    pub(crate) projects: Vec<Project>,
    pub(crate) drafts: Vec<Draft>,
    pub(crate) profiles: Vec<Profile>,
    pub(crate) active_profile: Option<String>,
    pub(crate) active_draft: Option<String>,
    pub(crate) theme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) ui_state: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) close_behavior: Option<CloseBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) update_source: Option<crate::models::update::UpdateSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) shortcuts: Option<Shortcuts>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CloseBehavior {
    Ask,
    Hide,
    Exit,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Shortcuts {
    pub(crate) new_draft: String,
    pub(crate) optimize: String,
    #[serde(default = "shortcut_newline")]
    pub(crate) newline: String,
    #[serde(default = "shortcut_newline_alternate")]
    pub(crate) newline_alternate: String,
    #[serde(default = "shortcut_rename")]
    pub(crate) rename: String,
    #[serde(default = "shortcut_pin")]
    pub(crate) pin: String,
    #[serde(default = "shortcut_favorite")]
    pub(crate) favorite: String,
    #[serde(default = "shortcut_delete")]
    pub(crate) delete: String,
    #[serde(default = "shortcut_copy_result")]
    pub(crate) copy_result: String,
}

fn shortcut_newline() -> String {
    "Shift+Enter".into()
}

fn shortcut_newline_alternate() -> String {
    "Alt+Enter".into()
}

fn shortcut_rename() -> String {
    "Ctrl+Alt+KeyR".into()
}

fn shortcut_pin() -> String {
    "Ctrl+Alt+KeyP".into()
}

fn shortcut_favorite() -> String {
    "Ctrl+Alt+KeyF".into()
}

fn shortcut_delete() -> String {
    "Ctrl+Shift+Delete".into()
}

fn shortcut_copy_result() -> String {
    "Ctrl+Alt+KeyC".into()
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub(crate) default_template_id: Option<String>,
    pub(crate) theme: String,
    pub(crate) close_behavior: Option<CloseBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) update_source: Option<crate::models::update::UpdateSource>,
    pub(crate) shortcuts: Option<Shortcuts>,
    pub(crate) active_draft: Option<String>,
    pub(crate) active_profile: Option<String>,
    pub(crate) ui_state: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct Changes<T> {
    #[serde(default)]
    pub(crate) upsert: Vec<T>,
    #[serde(default)]
    pub(crate) remove: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceChanges {
    pub(crate) projects: Option<Changes<Project>>,
    pub(crate) drafts: Option<Changes<Draft>>,
    pub(crate) profiles: Option<Changes<Profile>>,
    pub(crate) templates: Option<Changes<crate::models::conversation::PromptTemplate>>,
    pub(crate) template_categories: Option<Changes<Project>>,
    pub(crate) settings: Option<Settings>,
}

impl<T> Default for Changes<T> {
    fn default() -> Self {
        Self {
            upsert: vec![],
            remove: vec![],
        }
    }
}
