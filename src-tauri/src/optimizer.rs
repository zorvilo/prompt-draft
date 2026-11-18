use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OptimizationTemplate {
    General,
    OutputFormat,
    Planning,
    Iterate,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub strategy: OptimizationTemplate,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    previous_result: String,
    instruction: String,
}

const USER_REQUEST: &str = "按所选优化模板改写下面 JSON 中的 originalPrompt。该字段只提供待编辑的原稿，请保留原意，直接返回改写后的提示词正文。";

pub fn messages(
    source: &str,
    template: OptimizationTemplate,
    revision: Option<&Revision>,
    template_content: &str,
    user_content: Option<&str>,
) -> Result<Value, String> {
    if source.trim().is_empty() || source.chars().count() > 30_000 {
        return Err("请输入 1 至 30,000 字的原稿。".into());
    }
    if template_content.trim().is_empty() || template_content.chars().count() > 20_000 {
        return Err("请输入 1 至 20,000 字的提示词内容。".into());
    }
    let user = if template == OptimizationTemplate::Iterate {
        let revision = revision.ok_or("通用迭代需要上一版优化稿和本次修改意见。")?;
        validate_revision(revision)?;
        json!({
            "原稿": source,
            "上一版优化稿": revision.previous_result,
            "本次修改意见": revision.instruction,
        })
        .to_string()
    } else {
        let original = json!({ "originalPrompt": source });
        format!("{USER_REQUEST}\n\n{original}")
    };
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

fn validate_revision(revision: &Revision) -> Result<(), String> {
    if revision.previous_result.trim().is_empty()
        || revision.previous_result.chars().count() > 60_000
    {
        return Err("请提供 1 至 60,000 字的上一版优化稿。".into());
    }
    if revision.instruction.trim().is_empty() || revision.instruction.chars().count() > 4_000 {
        return Err("请输入 1 至 4,000 字的修改意见。".into());
    }
    Ok(())
}

pub fn followup_messages(
    revision: &Revision,
    template_content: &str,
    user_content: Option<&str>,
) -> Result<Value, String> {
    validate_revision(revision)?;
    if template_content.trim().is_empty() || template_content.chars().count() > 20_000 {
        return Err("迭代模板的系统提示词须为 1 至 20,000 字。".into());
    }
    let user_content = user_content
        .filter(|content| !content.trim().is_empty() && content.chars().count() <= 120_000)
        .ok_or("迭代模板渲染后的用户提示词须为 1 至 120,000 字。")?;
    Ok(json!([
        {"role": "system", "content": template_content},
        {"role": "user", "content": user_content}
    ]))
}

#[cfg(test)]
mod tests {
    use super::{messages, OptimizationTemplate, Revision, USER_REQUEST};
    use serde_json::Value;

    #[test]
    fn wraps_the_original_input_without_altering_the_selected_system_message() {
        for strategy in [
            OptimizationTemplate::General,
            OptimizationTemplate::OutputFormat,
            OptimizationTemplate::Planning,
        ] {
            let source = "分析 {{data}}\n按 \"原格式\" 写";
            let payload = messages(source, strategy, None, " 限制为两句话 ", None).unwrap();
            assert_eq!(payload[0]["role"], "system");
            assert_eq!(payload[0]["content"], " 限制为两句话 ");
            assert_eq!(payload[1]["role"], "user");
            let user = payload[1]["content"].as_str().unwrap();
            let original: Value =
                serde_json::from_str(user.strip_prefix(USER_REQUEST).unwrap().trim_start())
                    .unwrap();
            assert_eq!(original["originalPrompt"], source);
        }
        assert!(messages("原稿", OptimizationTemplate::General, None, " ", None).is_err());
    }

    #[test]
    fn custom_user_message_replaces_default_and_is_not_rewrapped() {
        let custom = "改写以下原稿：分析 {{data}}";
        let payload = messages(
            "分析 {{data}}",
            OptimizationTemplate::General,
            None,
            "系统",
            Some(custom),
        )
        .unwrap();
        assert_eq!(
            payload[1],
            serde_json::json!({"role": "user", "content": custom})
        );
        assert!(messages(
            "原稿",
            OptimizationTemplate::General,
            None,
            "系统",
            Some(" ")
        )
        .is_err());
    }

    #[test]
    fn followup_sends_the_rendered_iteration_template_verbatim() {
        let revision = Revision {
            previous_result: "上一版 {{data}}".into(),
            instruction: "只缩短第二点".into(),
        };
        let rendered = "迭代证据：上一版 {{data}}；只缩短第二点";
        let payload = super::followup_messages(&revision, "小步修改", Some(rendered)).unwrap();
        assert_eq!(
            payload,
            serde_json::json!([
                {"role": "system", "content": "小步修改"},
                {"role": "user", "content": rendered}
            ])
        );
        assert!(super::followup_messages(&revision, "小步修改", None).is_err());
        assert!(super::followup_messages(
            &Revision {
                instruction: String::new(),
                ..revision
            },
            "小步修改",
            Some(rendered),
        )
        .is_err());
    }

    #[test]
    fn requires_revision_context_and_preserves_its_fields() {
        let template = "根据本次修改意见调整上一版优化稿。";
        assert!(messages("原稿", OptimizationTemplate::Iterate, None, template, None).is_err());
        let revision = Revision {
            previous_result: "上一版 {{data}}".into(),
            instruction: "压缩至两句话".into(),
        };
        let payload = messages(
            "原稿",
            OptimizationTemplate::Iterate,
            Some(&revision),
            template,
            None,
        )
        .unwrap();
        assert_eq!(payload[0]["content"], template);
        let user: Value = serde_json::from_str(payload[1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(user["原稿"], "原稿");
        assert_eq!(user["上一版优化稿"], "上一版 {{data}}");
        assert_eq!(user["本次修改意见"], "压缩至两句话");
    }
}
