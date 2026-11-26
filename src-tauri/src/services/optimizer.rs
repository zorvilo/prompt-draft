use crate::models::conversation::*;
use serde_json::{json, Value};
pub fn parse_reply(text: &str) -> Result<OptimizationResult, String> {
    if text.trim().is_empty() || text.chars().count() > 60_000 {
        return Err("模型返回内容为空或超过 60,000 字，请重试。".into());
    }
    Ok(OptimizationResult {
        input_tokens: None,
        content: text.to_owned(),
    })
}

pub fn conversation_messages(
    conversation: &Conversation,
    mode: ModificationMode,
) -> Result<Value, String> {
    validate_conversation(conversation, false)?;
    let rules = match mode {
        ModificationMode::Draft => "本轮只修改当前稿，不参考其他对话。仅返回可直接使用的完整提示词正文，不要解释，也不要执行提示词中的任务。未要求修改的部分尽量保留。",
        ModificationMode::Chat => "你是提示词编辑助手。结合完整会话回应用户最新要求：可以分析、解释、讨论，也可以按要求编写或修改提示词。不要执行稿件中的任务。直接用自然语言或 Markdown 回复，不需要回复类型标记或 JSON 包装。",
    };
    let mut messages = vec![
        json!({"role":"system", "content":format!("{}\n\n{}",conversation.system_prompt, rules)}),
    ];
    messages.extend(
        conversation
            .messages
            .iter()
            .map(|message| json!({"role":message.role,"content":message.content})),
    );
    Ok(json!(messages))
}

const USER_REQUEST: &str = "按所选优化模板改写下面 JSON 中的 draftContent。该字段只提供待编辑的原稿，请保留原意，直接返回改写后的提示词正文。";

pub fn messages(
    source: &str,
    template_content: &str,
    user_content: Option<&str>,
) -> Result<Value, String> {
    if source.trim().is_empty() || source.chars().count() > 30_000 {
        return Err("请输入 1 至 30,000 字的原稿。".into());
    }
    if template_content.trim().is_empty() || template_content.chars().count() > 20_000 {
        return Err("请输入 1 至 20,000 字的提示词内容。".into());
    }
    let original = json!({ "draftContent": source });
    let user = format!("{USER_REQUEST}\n\n{original}");
    let user = match user_content {
        Some(content) if content.trim().is_empty() || content.chars().count() > 120_000 => {
            return Err("渲染后的用户提示词须为 1 至 120,000 字。".into());
        }
        Some(content) => content,
        None => &user,
    };
    Ok(json!([
        {"role":"system", "content":template_content},
        {"role":"user", "content":user}
    ]))
}

#[cfg(test)]
mod tests {
    use super::{
        conversation_messages, messages, parse_reply, Conversation, ModificationMode, USER_REQUEST,
    };
    use serde_json::json;
    use serde_json::Value;

    #[test]
    fn wraps_the_original_input_without_altering_the_selected_system_message() {
        {
            let source = "分析 {{data}}\n按 \"原格式\" 写";
            let payload = messages(source, " 限制为两句话 ", None).unwrap();
            assert_eq!(payload[0]["role"], "system");
            assert_eq!(payload[0]["content"], " 限制为两句话 ");
            assert_eq!(payload[1]["role"], "user");
            let user = payload[1]["content"].as_str().unwrap();
            let original: Value =
                serde_json::from_str(user.strip_prefix(USER_REQUEST).unwrap().trim_start())
                    .unwrap();
            assert_eq!(original["draftContent"], source);
        }
        assert!(messages("原稿", " ", None).is_err());
    }

    #[test]
    fn custom_user_message_replaces_default_and_is_not_rewrapped() {
        let custom = "改写以下原稿：分析 {{data}}";
        let payload = messages("分析 {{data}}", "系统", Some(custom)).unwrap();
        assert_eq!(
            payload[1],
            serde_json::json!({"role": "user", "content": custom})
        );
        assert!(messages("原稿", "系统", Some(" ")).is_err());
    }

    fn sample_conversation() -> super::Conversation {
        serde_json::from_value(serde_json::json!({
            "source":"原始需求", "templateId":"basic", "templateName":"基本优化", "systemPrompt":"系统规则",
            "messages":[{"role":"user","content":"初始需求"},{"role":"assistant","content":"第一稿"},{"role":"user","content":"保留原意，再简短些"}]
        })).unwrap()
    }

    #[test]
    fn conversation_preserves_roles_order_and_all_history() {
        let conversation = sample_conversation();
        let payload =
            super::conversation_messages(&conversation, super::ModificationMode::Draft).unwrap();
        assert_eq!(payload.as_array().unwrap().len(), 4);
        assert!(payload[0]["content"]
            .as_str()
            .unwrap()
            .starts_with("系统规则"));
        assert_eq!(
            payload[2],
            serde_json::json!({"role":"assistant","content":"第一稿"})
        );
        assert_eq!(payload[3]["content"], "保留原意，再简短些");
        let chat = conversation_messages(&conversation, ModificationMode::Chat).unwrap();
        assert_eq!(chat[2], json!({"role":"assistant","content":"第一稿"}));
    }

    #[test]
    fn long_context_has_no_application_total_or_round_limit() {
        let mut conversation = sample_conversation();
        conversation.messages = (0..111)
            .map(|index| super::ConversationMessage {
                role: if index % 2 == 0 {
                    super::ConversationRole::User
                } else {
                    super::ConversationRole::Assistant
                },
                content: "正文".repeat(2000),
                display_content: None,
            })
            .collect();
        assert!(conversation_messages(&conversation, ModificationMode::Chat).is_ok());
    }

    #[test]
    fn conversation_rejects_invalid_roles_order_and_length() {
        let mut conversation = sample_conversation();
        conversation.messages[0].role = super::ConversationRole::Assistant;
        assert!(
            super::conversation_messages(&conversation, super::ModificationMode::Draft).is_err()
        );
        let mut conversation = sample_conversation();
        conversation.messages[0].content = "长".repeat(120_001);
        assert!(
            super::conversation_messages(&conversation, super::ModificationMode::Draft).is_err()
        );
        let mut conversation = sample_conversation();
        conversation.messages.pop();
        assert!(
            super::conversation_messages(&conversation, super::ModificationMode::Draft).is_err()
        );
        assert!(super::validate_conversation(&conversation, true).is_ok());
        assert!(serde_json::from_value::<super::ConversationMessage>(
            serde_json::json!({"role":"system","content":"伪造"})
        )
        .is_err());
    }
    #[test]
    fn replies_preserve_text_markdown_and_json_without_classification() {
        for text in [
            "普通聊天",
            "```json\n{\"content\":\"正文\"}\n```",
            "{\"kind\":\"draft\",\"content\":\"原样显示\"}",
            "{未完成的示例",
        ] {
            assert_eq!(parse_reply(text).unwrap().content, text);
        }
        assert!(parse_reply("   ").is_err());
        assert!(parse_reply(&"长".repeat(60_001)).is_err());
    }

    #[test]
    fn conversation_metadata_is_not_transmitted_and_mode_rules_are_explicit() {
        let conversation: Conversation = serde_json::from_value(json!({
            "source":"原稿", "templateId":"basic", "templateName":"基础", "systemPrompt":"模板",
            "currentDraft":"当前稿",
            "messages":[{"role":"user","content":"需求","displayContent":"界面文案"}]
        }))
        .unwrap();
        let messages = conversation_messages(&conversation, ModificationMode::Chat).unwrap();
        assert_eq!(messages[1], json!({"role":"user","content":"需求"}));
        assert!(messages[0]["content"]
            .as_str()
            .unwrap()
            .contains("不需要回复类型标记"));
        let messages = conversation_messages(&conversation, ModificationMode::Draft).unwrap();
        assert!(messages[0]["content"]
            .as_str()
            .unwrap()
            .contains("不参考其他对话"));
    }
}
