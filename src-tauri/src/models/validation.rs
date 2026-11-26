use crate::models::workspace::{Draft, Shortcuts};
use std::collections::HashSet;
pub(crate) fn validate_dialogue(draft: &Draft) -> Result<(), String> {
    use crate::models::conversation::ConversationRole;
    if draft
        .dialogue
        .as_ref()
        .and_then(|state| state.chat_input.as_ref())
        .is_some_and(|text| text.chars().count() > 30_000)
    {
        return Err("对话输入最多 30,000 字。".into());
    }
    let mut ids = HashSet::new();
    for (index, message) in draft.messages.iter().enumerate() {
        let user = message.role == ConversationRole::User;
        if message.id.is_empty()
            || message.id.len() > 200
            || !ids.insert(&message.id)
            || (!user && (index == 0 || draft.messages[index - 1].role != ConversationRole::User))
            || message.content.trim().is_empty()
            || message.content.chars().count() > 120_000
            || message
                .display_content
                .as_ref()
                .is_some_and(|text| text.chars().count() > 120_000)
            || message
                .model
                .as_ref()
                .is_some_and(|text| text.chars().count() > 200)
            || message
                .provider_name
                .as_ref()
                .is_some_and(|text| text.chars().count() > 200)
            || user != message.context.is_some()
        {
            return Err("聊天消息的 ID、顺序、内容或请求信息无效。".into());
        }
        if let Some(context) = &message.context {
            let request = crate::models::conversation::Conversation {
                source: context.source.clone(),
                template_id: context.template_id.clone(),
                template_name: context.template_name.clone(),
                system_prompt: context.system_prompt.clone(),
                current_draft: context.current_draft.clone(),
                messages: vec![crate::models::conversation::ConversationMessage {
                    role: message.role.clone(),
                    content: message.content.clone(),
                    display_content: message.display_content.clone(),
                }],
            };
            crate::models::conversation::validate_conversation(&request, false)?;
        }
    }
    Ok(())
}

pub(crate) fn invalid_shortcuts(keys: &Shortcuts) -> bool {
    let values = [
        &keys.optimize,
        &keys.newline,
        &keys.new_draft,
        &keys.rename,
        &keys.pin,
        &keys.favorite,
        &keys.delete,
        &keys.copy_result,
    ];
    (!keys.newline_alternate.is_empty()
        && (!valid_shortcut(&keys.newline_alternate) || values.contains(&&keys.newline_alternate)))
        || values.iter().any(|value| !valid_shortcut(value))
        || values
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != values.len()
}

pub(crate) fn valid_shortcut(value: &str) -> bool {
    let mut code = value;
    let ctrl = code.strip_prefix("Ctrl+");
    if let Some(rest) = ctrl {
        code = rest;
    }
    let alt = code.strip_prefix("Alt+");
    if let Some(rest) = alt {
        code = rest;
    }
    if let Some(rest) = code.strip_prefix("Shift+") {
        code = rest;
    }
    let valid_code = code
        .strip_prefix("Key")
        .is_some_and(|s| s.len() == 1 && s.as_bytes()[0].is_ascii_uppercase())
        || code
            .strip_prefix("Digit")
            .is_some_and(|s| s.len() == 1 && s.as_bytes()[0].is_ascii_digit())
        || (1..=12).any(|n| code == format!("F{n}"))
        || [
            "Enter",
            "Space",
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "Insert",
            "Delete",
            "Backquote",
            "Minus",
            "Equal",
            "BracketLeft",
            "BracketRight",
            "Backslash",
            "Semicolon",
            "Quote",
            "Comma",
            "Period",
            "Slash",
        ]
        .contains(&code);
    valid_code
        && ![
            "Alt+F4",
            "Alt+Space",
            "Ctrl+Alt+Delete",
            "Ctrl+KeyW",
            "Ctrl+KeyR",
            "Ctrl+Shift+KeyR",
            "Ctrl+KeyC",
            "Ctrl+KeyV",
            "Ctrl+KeyX",
            "Ctrl+KeyA",
            "Ctrl+KeyZ",
            "Ctrl+KeyY",
            "Ctrl+KeyF",
            "Ctrl+KeyP",
            "Ctrl+KeyS",
            "Ctrl+KeyL",
            "Ctrl+KeyT",
        ]
        .contains(&value)
}
