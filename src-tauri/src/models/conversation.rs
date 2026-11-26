use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub name: String,
    pub content: String,
    pub description: String,
    #[serde(
        default,
        rename = "userContent",
        skip_serializing_if = "Option::is_none"
    )]
    pub user_content: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Conversation {
    pub source: String,
    pub template_id: String,
    pub template_name: String,
    pub system_prompt: String,
    pub messages: Vec<ConversationMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_draft: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConversationRole {
    User,
    Assistant,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMessage {
    pub role: ConversationRole,
    pub content: String,
    #[serde(
        default,
        rename = "displayContent",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_content: Option<String>,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModificationMode {
    Draft,
    Chat,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
pub struct OptimizationResult {
    #[serde(
        default,
        rename = "inputTokens",
        skip_serializing_if = "Option::is_none"
    )]
    pub input_tokens: Option<i64>,
    pub content: String,
}

pub fn validate_conversation(conversation: &Conversation, complete: bool) -> Result<(), String> {
    let count = conversation.messages.len();
    if conversation
        .current_draft
        .as_ref()
        .is_some_and(|text| text.chars().count() > 60_000)
    {
        return Err("当前稿超过 60,000 字。".into());
    }
    if conversation.source.trim().is_empty()
        || conversation.source.chars().count() > 30_000
        || conversation.template_id.is_empty()
        || conversation.template_id.chars().count() > 200
        || conversation.template_name.chars().count() > 200
        || conversation.system_prompt.trim().is_empty()
        || conversation.system_prompt.chars().count() > 20_000
    {
        return Err("对话的原始需求或模板内容无效。".into());
    }
    if count == 0 || count.is_multiple_of(2) != complete {
        return Err("对话记录顺序无效或轮数过多，请重新优化开启新一轮。".into());
    }
    for (index, message) in conversation.messages.iter().enumerate() {
        let expected = if index % 2 == 0 {
            ConversationRole::User
        } else {
            ConversationRole::Assistant
        };
        let length = message.content.chars().count();
        if message.role != expected
            || message.content.trim().is_empty()
            || length > 120_000
            || message
                .display_content
                .as_ref()
                .is_some_and(|text| text.chars().count() > 120_000)
        {
            return Err("对话消息内容或角色顺序无效。".into());
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeRequest {
    pub(crate) profile: crate::models::provider::Profile,
    pub(crate) request_id: String,
    pub(crate) conversation: crate::models::conversation::Conversation,
    pub(crate) mode: crate::models::conversation::ModificationMode,
}
