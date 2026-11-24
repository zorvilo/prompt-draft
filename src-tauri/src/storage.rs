use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    id: String,
    name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatContext {
    source: String,
    template_id: String,
    template_name: String,
    system_prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    current_draft: Option<String>,
    mode: crate::optimizer::ModificationMode,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatMessage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    id: String,
    role: crate::optimizer::ConversationRole,
    content: String,
    created: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    display_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context: Option<ChatContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider_name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    #[serde(default)]
    template_id: Option<String>,
    #[serde(default)]
    provider_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    id: String,
    project_id: Option<String>,
    title: String,
    source: String,
    result: String,
    favorite: bool,
    #[serde(default)]
    pinned: bool,
    opened: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated: Option<u64>,
    messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dialogue: Option<DialogueState>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DialogueState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mode: Option<crate::optimizer::ModificationMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chat_input: Option<String>,
}

fn validate_dialogue(draft: &Draft) -> Result<(), String> {
    use crate::optimizer::ConversationRole;
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
            let request = crate::optimizer::Conversation {
                source: context.source.clone(),
                template_id: context.template_id.clone(),
                template_name: context.template_name.clone(),
                system_prompt: context.system_prompt.clone(),
                current_draft: context.current_draft.clone(),
                messages: vec![crate::optimizer::ConversationMessage {
                    role: message.role.clone(),
                    content: message.content.clone(),
                    display_content: message.display_content.clone(),
                }],
            };
            crate::optimizer::validate_conversation(&request, false)?;
        }
    }
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default)]
    pub is_default: bool,
    pub id: String,
    pub name: String,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    #[serde(default)]
    default_template_id: Option<String>,
    schema: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    templates: Option<Vec<crate::optimizer::PromptTemplate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template_categories: Option<Vec<Project>>,
    projects: Vec<Project>,
    drafts: Vec<Draft>,
    profiles: Vec<Profile>,
    active_profile: Option<String>,
    active_draft: Option<String>,
    theme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ui_state: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    close_behavior: Option<CloseBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shortcuts: Option<Shortcuts>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CloseBehavior {
    Ask,
    Hide,
    Exit,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Shortcuts {
    new_draft: String,
    optimize: String,
    #[serde(default = "shortcut_newline")]
    newline: String,
    #[serde(default = "shortcut_newline_alternate")]
    newline_alternate: String,
    #[serde(default = "shortcut_rename")]
    rename: String,
    #[serde(default = "shortcut_pin")]
    pin: String,
    #[serde(default = "shortcut_favorite")]
    favorite: String,
    #[serde(default = "shortcut_delete")]
    delete: String,
    #[serde(default = "shortcut_copy_result")]
    copy_result: String,
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
fn invalid_shortcuts(keys: &Shortcuts) -> bool {
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

fn valid_shortcut(value: &str) -> bool {
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

#[cfg(test)]
pub const MAX_BACKUP: usize = 32 * 1024 * 1024;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    connection.execute_batch("PRAGMA secure_delete=ON;")?;
    connection.execute_batch(include_str!("schema.sql"))?;
    crate::usage::initialize(&connection)?;
    Ok(connection)
}

pub fn load_key(connection: &Connection, profile: &Profile) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT nullif(api_key,'') FROM providers WHERE id=?1 AND endpoint=?2",
            params![profile.id, profile.endpoint],
            |r| r.get(0),
        )
        .optional()
        .map(Option::flatten)
}
pub fn save_key(connection: &Connection, profile: &Profile, key: &str) -> rusqlite::Result<()> {
    let tx = connection.unchecked_transaction()?;
    upsert_provider(&tx, profile)?;
    tx.execute(
        "UPDATE providers SET api_key=?1,updated_at=?4 WHERE id=?2 AND endpoint=?3 AND api_key IS NOT ?1",
        params![key, profile.id, profile.endpoint, now()],
    )?;
    tx.commit()
}

#[cfg(test)]
pub fn parse(input: &str) -> Result<Workspace, String> {
    if input.len() > MAX_BACKUP {
        return Err("工作区数据超过 32 MB，请先导出并归档部分稿件。".into());
    }
    let workspace: Workspace =
        serde_json::from_str(input).map_err(|_| "无法读取词稿备份，请选择有效的 JSON 文件。")?;
    let projects: HashSet<_> = workspace.projects.iter().map(|item| &item.id).collect();
    let drafts: HashSet<_> = workspace.drafts.iter().map(|item| &item.id).collect();
    let profiles: HashSet<_> = workspace.profiles.iter().map(|item| &item.id).collect();
    let mut message_ids = HashSet::new();
    for draft in &workspace.drafts {
        validate_dialogue(draft)?;
        for message in &draft.messages {
            if !message_ids.insert(&message.id) {
                return Err("聊天消息 ID 重复。".into());
            }
        }
    }
    let bad_project = workspace.drafts.iter().any(|draft| {
        draft
            .project_id
            .as_ref()
            .is_some_and(|id| !projects.contains(id))
    });
    if workspace.shortcuts.as_ref().is_some_and(invalid_shortcuts) {
        return Err("快捷键配置无效或存在冲突。".into());
    }
    if workspace.schema != 1
        || projects.is_empty()
        || bad_project
        || projects.len() != workspace.projects.len()
        || drafts.len() != workspace.drafts.len()
        || profiles.len() != workspace.profiles.len()
        || workspace
            .active_draft
            .as_ref()
            .is_some_and(|id| !drafts.contains(id))
        || workspace
            .active_profile
            .as_ref()
            .is_some_and(|id| !profiles.contains(id))
        || !matches!(workspace.theme.as_str(), "light" | "dark" | "system")
    {
        return Err("备份版本或数据关系无效。".into());
    }
    Ok(workspace)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    default_template_id: Option<String>,
    theme: String,
    close_behavior: Option<CloseBehavior>,
    shortcuts: Option<Shortcuts>,
    active_draft: Option<String>,
    active_profile: Option<String>,
    ui_state: Option<serde_json::Value>,
}
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct Changes<T> {
    #[serde(default)]
    upsert: Vec<T>,
    #[serde(default)]
    remove: Vec<String>,
}
impl<T> Default for Changes<T> {
    fn default() -> Self {
        Self {
            upsert: vec![],
            remove: vec![],
        }
    }
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceChanges {
    projects: Option<Changes<Project>>,
    drafts: Option<Changes<Draft>>,
    profiles: Option<Changes<Profile>>,
    templates: Option<Changes<crate::optimizer::PromptTemplate>>,
    template_categories: Option<Changes<Project>>,
    settings: Option<Settings>,
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn decode<T: serde::de::DeserializeOwned>(text: String) -> rusqlite::Result<T> {
    serde_json::from_str(&text).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}
fn json_rows<T: serde::de::DeserializeOwned>(
    db: &Connection,
    sql: &str,
) -> rusqlite::Result<Vec<T>> {
    db.prepare(sql)?
        .query_map([], |r| decode(r.get(0)?))?
        .collect()
}
pub fn load(connection: &Connection) -> rusqlite::Result<Option<String>> {
    let settings: Option<(String, String)> = connection
        .query_row(
            "SELECT preferences_json,ui_state_json FROM app_settings WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((preferences, ui)) = settings else {
        return Ok(None);
    };
    let preferences: serde_json::Value = decode(preferences)?;
    let ui: serde_json::Value = decode(ui)?;
    let mut drafts:Vec<Draft>=json_rows(connection,"SELECT json_object('id',id,'templateId',template_id,'providerId',provider_id,'model',model,'projectId',project_id,'title',title,'source',source,'result',result,'favorite',json(CASE WHEN is_favorite THEN 'true' ELSE 'false' END),'pinned',json(CASE WHEN is_pinned THEN 'true' ELSE 'false' END),'opened',created_at,'updated',updated_at,'messages',json('[]')) FROM drafts ORDER BY created_at,rowid")?;
    let mut messages: HashMap<String, Vec<ChatMessage>> = HashMap::new();
    for row in connection
        .prepare(
            "SELECT id,draft_id,data_json,position FROM draft_messages ORDER BY draft_id,position",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                decode::<ChatMessage>(row.get(2)?)?,
                row.get::<_, usize>(3)?,
            ))
        })?
    {
        let (id, draft_id, message, position) = row?;
        let entries = messages.entry(draft_id).or_default();
        if id != message.id || position != entries.len() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        entries.push(message);
    }
    for draft in &mut drafts {
        draft.messages = messages.remove(&draft.id).unwrap_or_default();
        draft.dialogue = connection
            .query_row(
                "SELECT state_json FROM draft_dialogues WHERE draft_id=?1",
                [&draft.id],
                |row| decode(row.get(0)?),
            )
            .optional()?;
        validate_dialogue(draft).map_err(|_| rusqlite::Error::InvalidQuery)?;
    }
    let workspace=Workspace {
        default_template_id:preferences["defaultTemplateId"].as_str().map(String::from),
        schema:1,
        projects:json_rows(connection,"SELECT json_object('id',id,'name',name) FROM projects ORDER BY created_at,rowid")?,
        drafts,
        profiles:json_rows(connection,"SELECT json_object('id',id,'name',name,'endpoint',endpoint,'presetId',preset_id,'website',website,'isDefault',json(CASE WHEN is_default THEN 'true' ELSE 'false' END),'model',default_model,'catalog',json(models_json)) FROM providers ORDER BY created_at,rowid")?,
        templates:Some(json_rows(connection,"SELECT json_object('id',id,'category',category_id,'name',name,'content',system_prompt,'userContent',user_prompt,'description',description) FROM templates ORDER BY created_at,rowid")?),
        template_categories:Some(json_rows(connection,"SELECT json_object('id',id,'name',name) FROM template_categories ORDER BY created_at,rowid")?),
        theme:preferences["theme"].as_str().unwrap_or("system").into(),
        close_behavior:serde_json::from_value(preferences["closeBehavior"].clone()).map_err(|_|rusqlite::Error::InvalidQuery)?,
        shortcuts:serde_json::from_value(preferences["shortcuts"].clone()).map_err(|_|rusqlite::Error::InvalidQuery)?,
        active_draft:ui["activeDraft"].as_str().map(String::from),
        active_profile:ui["activeProfile"].as_str().map(String::from),
        ui_state:ui.get("uiState").filter(|v|!v.is_null()).cloned(),
    };
    serde_json::to_string(&workspace)
        .map(Some)
        .map_err(|_| rusqlite::Error::InvalidQuery)
}
fn upsert_project(db: &Connection, table: &str, item: &Project) -> rusqlite::Result<()> {
    db.execute(&format!("INSERT INTO {table}(id,name,created_at,updated_at) VALUES(?1,?2,?3,?3) ON CONFLICT(id) DO UPDATE SET name=excluded.name,updated_at=excluded.updated_at WHERE name IS NOT excluded.name"),params![item.id,item.name,now()])?;
    Ok(())
}
fn upsert_provider(db: &Connection, item: &Profile) -> rusqlite::Result<()> {
    if item.is_default {
        db.execute(
            "UPDATE providers SET is_default=0 WHERE id<>?1 AND is_default=1",
            [&item.id],
        )?;
    }
    let models = item
        .catalog
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap());
    db.execute("INSERT INTO providers(id,name,preset_id,website,endpoint,default_model,models_json,created_at,updated_at,is_default) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8,?9)
        ON CONFLICT(id) DO UPDATE SET name=excluded.name,preset_id=excluded.preset_id,website=excluded.website,endpoint=excluded.endpoint,default_model=excluded.default_model,models_json=excluded.models_json,is_default=excluded.is_default,
        api_key=CASE WHEN providers.endpoint=excluded.endpoint THEN providers.api_key ELSE '' END,updated_at=excluded.updated_at
        WHERE (name,preset_id,website,endpoint,default_model,models_json,is_default) IS NOT (excluded.name,excluded.preset_id,excluded.website,excluded.endpoint,excluded.default_model,excluded.models_json,excluded.is_default)",params![item.id,item.name,item.preset_id,item.website,item.endpoint,item.model,models,now(),item.is_default])?;
    Ok(())
}
fn upsert_draft(db: &Connection, item: &Draft) -> rusqlite::Result<()> {
    validate_dialogue(item).map_err(|_| rusqlite::Error::InvalidQuery)?;
    db.execute("INSERT INTO drafts(id,project_id,title,source,result,is_pinned,is_favorite,created_at,updated_at,template_id,provider_id,model) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) ON CONFLICT(id) DO UPDATE SET template_id=excluded.template_id,provider_id=excluded.provider_id,model=excluded.model,project_id=excluded.project_id,title=excluded.title,source=excluded.source,result=excluded.result,is_pinned=excluded.is_pinned,is_favorite=excluded.is_favorite,updated_at=excluded.updated_at
        WHERE (template_id,provider_id,model,project_id,title,source,result,is_pinned,is_favorite,updated_at) IS NOT (excluded.template_id,excluded.provider_id,excluded.model,excluded.project_id,excluded.title,excluded.source,excluded.result,excluded.is_pinned,excluded.is_favorite,excluded.updated_at)",params![item.id,item.project_id,item.title,item.source,item.result,item.pinned,item.favorite,item.opened,item.updated.unwrap_or(item.opened),item.template_id,item.provider_id,item.model])?;
    if let Some(dialogue) = &item.dialogue {
        let state = serde_json::to_string(dialogue).map_err(|_| rusqlite::Error::InvalidQuery)?;
        db.execute("INSERT INTO draft_dialogues VALUES(?1,?2) ON CONFLICT(draft_id) DO UPDATE SET state_json=excluded.state_json WHERE state_json IS NOT excluded.state_json", params![item.id, state])?;
    } else {
        db.execute("DELETE FROM draft_dialogues WHERE draft_id=?1", [&item.id])?;
    }
    let ids: Vec<_> = item.messages.iter().map(|message| &message.id).collect();
    db.execute("DELETE FROM draft_messages WHERE draft_id=?1 AND id NOT IN(SELECT value FROM json_each(?2))",params![item.id,serde_json::to_string(&ids).unwrap()])?;
    for (position, message) in item.messages.iter().enumerate() {
        let occupied: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM draft_messages WHERE id=?1 AND draft_id<>?2)",
            params![message.id, item.id],
            |row| row.get(0),
        )?;
        if occupied {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let data = serde_json::to_string(message).map_err(|_| rusqlite::Error::InvalidQuery)?;
        db.execute("INSERT INTO draft_messages VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET position=excluded.position,data_json=excluded.data_json WHERE (position,data_json) IS NOT (excluded.position,excluded.data_json)",params![message.id,item.id,position,data])?;
    }
    Ok(())
}
fn upsert_template(
    db: &Connection,
    item: &crate::optimizer::PromptTemplate,
) -> rusqlite::Result<()> {
    db.execute("INSERT INTO templates VALUES(?1,?2,?3,?4,?5,?6,?7,?7) ON CONFLICT(id) DO UPDATE SET category_id=excluded.category_id,name=excluded.name,description=excluded.description,system_prompt=excluded.system_prompt,user_prompt=excluded.user_prompt,updated_at=excluded.updated_at
        WHERE (category_id,name,description,system_prompt,user_prompt) IS NOT (excluded.category_id,excluded.name,excluded.description,excluded.system_prompt,excluded.user_prompt)",params![item.id,item.category.as_deref().unwrap_or("basic"),item.name,item.description,item.content,item.user_content,now()])?;
    Ok(())
}
fn persist(db: &Connection, changes: &WorkspaceChanges) -> rusqlite::Result<()> {
    if let Some(items) = &changes.projects {
        for item in &items.upsert {
            upsert_project(db, "projects", item)?;
        }
    }
    if let Some(items) = &changes.profiles {
        for item in &items.upsert {
            upsert_provider(db, item)?;
        }
    }
    if let Some(items) = &changes.template_categories {
        for item in &items.upsert {
            upsert_project(db, "template_categories", item)?;
        }
    }
    if let Some(items) = &changes.drafts {
        for item in &items.upsert {
            upsert_draft(db, item)?;
        }
    }
    if let Some(items) = &changes.templates {
        for item in &items.upsert {
            upsert_template(db, item)?;
        }
    }
    for (table, removed) in [
        ("drafts", changes.drafts.as_ref().map(|v| &v.remove)),
        ("providers", changes.profiles.as_ref().map(|v| &v.remove)),
        ("projects", changes.projects.as_ref().map(|v| &v.remove)),
        ("templates", changes.templates.as_ref().map(|v| &v.remove)),
        (
            "template_categories",
            changes.template_categories.as_ref().map(|v| &v.remove),
        ),
    ] {
        if let Some(ids) = removed {
            for id in ids {
                db.execute(&format!("DELETE FROM {table} WHERE id=?1"), [id])?;
            }
        }
    }
    if let Some(settings) = &changes.settings {
        let preferences=serde_json::json!({"defaultTemplateId":settings.default_template_id,"theme":settings.theme,"closeBehavior":settings.close_behavior,"shortcuts":settings.shortcuts}).to_string();
        let ui=serde_json::json!({"activeDraft":settings.active_draft,"activeProfile":settings.active_profile,"uiState":settings.ui_state}).to_string();
        db.execute("INSERT INTO app_settings VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET preferences_json=excluded.preferences_json,ui_state_json=excluded.ui_state_json WHERE (preferences_json,ui_state_json) IS NOT (excluded.preferences_json,excluded.ui_state_json)",params![preferences,ui])?;
    }
    Ok(())
}

pub(crate) fn validate_state(db: &Connection) -> Result<(), String> {
    let (prefs, ui): (String, String) = db
        .query_row(
            "SELECT preferences_json,ui_state_json FROM app_settings WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "缺少应用设置。")?;
    let mut value: serde_json::Value =
        serde_json::from_str(&prefs).map_err(|_| "应用设置无效。")?;
    let ui: serde_json::Value = serde_json::from_str(&ui).map_err(|_| "界面状态无效。")?;
    for key in ["activeDraft", "activeProfile", "uiState"] {
        value[key] = ui[key].clone();
    }
    let settings: Settings = serde_json::from_value(value).map_err(|_| "设置内容无效。")?;
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark")
        || settings.shortcuts.as_ref().is_some_and(invalid_shortcuts)
    {
        return Err("主题或快捷键设置无效。".into());
    }
    for (table, id) in [
        ("drafts", settings.active_draft),
        ("providers", settings.active_profile),
    ] {
        if let Some(id) = id {
            let exists: bool = db
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
                    [id],
                    |r| r.get(0),
                )
                .map_err(|_| "无法检查界面状态。")?;
            if !exists {
                return Err("当前选中的数据不存在。".into());
            }
        }
    }
    let has_projects: bool = db
        .query_row("SELECT EXISTS(SELECT 1 FROM projects)", [], |r| r.get(0))
        .map_err(|_| "无法检查项目。")?;
    if !has_projects {
        return Err("至少保留一个项目。".into());
    }
    let invalid_categories:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM templates t WHERE category_id <> 'basic' AND NOT EXISTS(SELECT 1 FROM template_categories c WHERE c.id=t.category_id))",[],|r|r.get(0)).map_err(|_|"无法检查模板分类。")?;
    let builtin_categories: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM template_categories WHERE id = 'basic')",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "无法检查系统分类。")?;
    let builtin: serde_json::Value = serde_json::from_str(include_str!(
        "../../src/features/library/upstream/builtin-templates.json"
    ))
    .map_err(|_| "系统模板无效。")?;
    let ids: Vec<_> = builtin
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect();
    let builtin_templates: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM templates WHERE id IN(SELECT value FROM json_each(?1)))",
            [serde_json::to_string(&ids).unwrap()],
            |r| r.get(0),
        )
        .map_err(|_| "无法检查系统模板。")?;
    if invalid_categories || builtin_categories || builtin_templates {
        return Err("模板分类无效或试图覆盖系统内置内容。".into());
    }
    Ok(())
}
fn check_changes<T>(items: &Option<Changes<T>>, id: impl Fn(&T) -> &String) -> Result<(), String> {
    if let Some(items) = items {
        let mut ids = HashSet::new();
        for item in &items.upsert {
            let id = id(item);
            if id.is_empty() || !ids.insert(id) {
                return Err("记录 ID 为空或重复。".into());
            }
        }
        for removed in &items.remove {
            if removed.is_empty() || !ids.insert(removed) {
                return Err("删除的记录 ID 为空或重复。".into());
            }
        }
    }
    Ok(())
}
pub fn apply_changes(db: &Connection, changes: &WorkspaceChanges) -> Result<(), String> {
    check_changes(&changes.projects, |v| &v.id)?;
    check_changes(&changes.drafts, |v| &v.id)?;
    check_changes(&changes.profiles, |v| &v.id)?;
    check_changes(&changes.templates, |v| &v.id)?;
    check_changes(&changes.template_categories, |v| &v.id)?;
    let mut message_ids = HashSet::new();
    if let Some(items) = &changes.drafts {
        for draft in &items.upsert {
            validate_dialogue(draft)?;
            for message in &draft.messages {
                if !message_ids.insert(&message.id) {
                    return Err("聊天消息 ID 重复。".into());
                }
            }
        }
    }
    let tx = db.unchecked_transaction().map_err(|_| "无法开始保存。")?;
    persist(&tx, changes).map_err(|_| "保存失败，请检查数据关联和磁盘空间。")?;
    validate_state(&tx)?;
    tx.commit().map_err(|_| "无法完成保存。".into())
}
#[cfg(test)]
fn replace_changes<T: Clone>(
    db: &Connection,
    table: &str,
    items: &[T],
    id: impl Fn(&T) -> &String,
) -> rusqlite::Result<Changes<T>> {
    let ids: HashSet<_> = items.iter().map(id).collect();
    let removed = db
        .prepare(&format!("SELECT id FROM {table}"))?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|id| !ids.contains(id))
        .collect();
    Ok(Changes {
        upsert: items.to_vec(),
        remove: removed,
    })
}
#[cfg(test)]
pub fn save(db: &Connection, document: &Workspace) -> Result<(), String> {
    let tx = db.unchecked_transaction().map_err(|_| "无法开始保存。")?;
    let changes = (|| -> rusqlite::Result<_> {
        Ok(WorkspaceChanges {
            projects: Some(replace_changes(&tx, "projects", &document.projects, |v| {
                &v.id
            })?),
            drafts: Some(replace_changes(&tx, "drafts", &document.drafts, |v| &v.id)?),
            profiles: Some(replace_changes(
                &tx,
                "providers",
                &document.profiles,
                |v| &v.id,
            )?),
            templates: Some(replace_changes(
                &tx,
                "templates",
                document.templates.as_deref().unwrap_or(&[]),
                |v| &v.id,
            )?),
            template_categories: Some(replace_changes(
                &tx,
                "template_categories",
                document.template_categories.as_deref().unwrap_or(&[]),
                |v| &v.id,
            )?),
            settings: Some(Settings {
                default_template_id: document.default_template_id.clone(),
                theme: document.theme.clone(),
                close_behavior: document.close_behavior.clone(),
                shortcuts: document.shortcuts.clone(),
                active_draft: document.active_draft.clone(),
                active_profile: document.active_profile.clone(),
                ui_state: document.ui_state.clone(),
            }),
        })
    })()
    .map_err(|_| "无法准备工作区数据。")?;
    persist(&tx, &changes).map_err(|_| "保存失败，请检查数据关联和磁盘空间。")?;
    validate_state(&tx)?;
    tx.commit().map_err(|_| "无法完成保存。".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_bound_to_the_provider_and_excluded_from_workspace_backups() {
        let connection = open(Path::new(":memory:")).unwrap();
        let mut profile = Profile {
            is_default: false,
            id: "account".into(),
            name: "测试".into(),
            endpoint: "https://provider.example/v1".into(),
            model: "model".into(),
            catalog: None,
            preset_id: None,
            website: None,
        };
        save_key(&connection, &profile, "fixture-key").unwrap();
        assert_eq!(
            load_key(&connection, &profile).unwrap().as_deref(),
            Some("fixture-key")
        );
        assert_eq!(load(&connection).unwrap(), None);
        profile.endpoint = "https://other.example/v1".into();
        assert_eq!(load_key(&connection, &profile).unwrap(), None);
        profile.endpoint = "https://provider.example/v1".into();
        save_key(&connection, &profile, "replacement-fixture").unwrap();
        assert_eq!(
            load_key(&connection, &profile).unwrap().as_deref(),
            Some("replacement-fixture")
        );
        save_key(&connection, &profile, "").unwrap();
        assert_eq!(load_key(&connection, &profile).unwrap(), None);
    }

    #[test]
    fn provider_metadata_roundtrip() {
        let input = r#"{"schema":1,"projects":[{"id":"p","name":"测试"}],"drafts":[],"profiles":[{"id":"deepseek","name":"DeepSeek 2","endpoint":"https://api.deepseek.com","model":"deepseek-flash"}],"activeProfile":"deepseek","activeDraft":null,"theme":"system"}"#;
        let mut workspace = parse(input).unwrap();
        assert!(workspace.profiles[0].preset_id.is_none());
        assert!(workspace.profiles[0].website.is_none());
        workspace.profiles[0].preset_id = Some("preset-deepseek".into());
        workspace.profiles[0].website = Some("https://example.com".into());
        workspace.profiles[0].endpoint = "https://proxy.example/v1".into();
        let connection = open(Path::new(":memory:")).unwrap();
        save(&connection, &workspace).unwrap();
        let saved = load(&connection).unwrap().unwrap();
        let restored = parse(&saved).unwrap();
        assert_eq!(
            restored.profiles[0].preset_id.as_deref(),
            Some("preset-deepseek")
        );
        assert_eq!(
            restored.profiles[0].website.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(restored.profiles[0].endpoint, "https://proxy.example/v1");
        assert_eq!(restored.profiles[0].name, "DeepSeek 2");
    }

    #[test]
    fn fresh_database_has_only_current_schema_and_reopens_without_changes() {
        let db = open(Path::new(":memory:")).unwrap();
        let draft_columns: Vec<String> = db
            .prepare("PRAGMA table_info(drafts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            draft_columns,
            vec![
                "id",
                "project_id",
                "template_id",
                "provider_id",
                "model",
                "title",
                "source",
                "result",
                "is_pinned",
                "is_favorite",
                "created_at",
                "updated_at"
            ]
        );

        let names: Vec<String>=db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([],|r|r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(
            names,
            vec![
                "app_settings",
                "draft_dialogues",
                "draft_messages",
                "drafts",
                "model_prices",
                "models_dev_cache",
                "pricing_settings",
                "projects",
                "providers",
                "template_categories",
                "templates",
                "usage_records"
            ]
        );
        let prices = crate::usage::catalog_rows(&db).unwrap();
        assert_eq!(prices.len(), 83);
        crate::usage::initialize(&db).unwrap();
        assert_eq!(crate::usage::catalog_rows(&db).unwrap(), prices);
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn window_preferences_and_shortcuts_roundtrip() {
        let input = r#"{"schema":1,"projects":[{"id":"p","name":"测试"}],"drafts":[],"profiles":[],"activeProfile":null,"activeDraft":null,"theme":"system"}"#;
        let workspace = parse(input).unwrap();
        assert!(workspace.close_behavior.is_none());
        assert!(workspace.shortcuts.is_none());
        let mut document: serde_json::Value = serde_json::from_str(input).unwrap();
        document["closeBehavior"] = serde_json::json!("hide");
        document["shortcuts"] = serde_json::json!({"newDraft":"Ctrl+Shift+KeyN","optimize":"Alt+Enter","newlineAlternate":""});
        document["shortcuts"]["newline"] = serde_json::json!("Alt+Enter");
        assert!(parse(&document.to_string()).is_err());
        document["shortcuts"]["newline"] = serde_json::json!("Shift+Enter");
        let connection = open(Path::new(":memory:")).unwrap();
        save(&connection, &parse(&document.to_string()).unwrap()).unwrap();
        let saved = load(&connection).unwrap().unwrap();
        assert!(matches!(
            parse(&saved).unwrap().close_behavior,
            Some(CloseBehavior::Hide)
        ));
        assert_eq!(
            parse(&saved).unwrap().shortcuts.unwrap().optimize,
            "Alt+Enter"
        );
        assert_eq!(
            parse(&saved).unwrap().shortcuts.unwrap().rename,
            "Ctrl+Alt+KeyR"
        );
        document["shortcuts"]["pin"] = serde_json::json!("Ctrl+Alt+KeyR");
        assert!(parse(&document.to_string()).is_err());
        document["shortcuts"]["pin"] = serde_json::json!("Alt+F4");
        assert!(parse(&document.to_string()).is_err());
        document["shortcuts"]["pin"] = serde_json::json!("Ctrl+Alt+KeyP");
        document["shortcuts"]["newDraft"] = serde_json::json!("Alt+Enter");
        assert!(parse(&document.to_string()).is_err());
        document["shortcuts"]["newDraft"] = serde_json::json!("Alt+F4");
        assert!(parse(&document.to_string()).is_err());
        document["closeBehavior"] = serde_json::json!("unknown");
        assert!(parse(&document.to_string()).is_err());
        for value in [
            "Ctrl+KeyN",
            "Ctrl+Alt+Digit1",
            "Alt+Shift+Enter",
            "Ctrl+F12",
            "KeyN",
            "Shift+KeyN",
            "Enter",
            "F2",
        ] {
            assert!(valid_shortcut(value), "{value}");
        }
        for value in [
            "Meta+KeyN",
            "Ctrl+KeyC",
            "Alt+F4",
            "Ctrl+Ctrl+KeyN",
            "Ctrl+F13",
        ] {
            assert!(!valid_shortcut(value), "{value}");
        }
    }

    #[test]
    fn custom_template_categories_survive_save_and_restore() {
        let input = r#"{"schema":1,"projects":[{"id":"p","name":"测试"}],"drafts":[],"profiles":[],"activeProfile":null,"activeDraft":null,"theme":"system","templateCategories":[{"id":"custom","name":"写作"}],"templates":[{"id":"t","category":"custom","name":"测试","content":"改写","description":""}]}"#;
        let mut workspace = parse(input).unwrap();
        workspace
            .template_categories
            .as_mut()
            .unwrap()
            .push(Project {
                id: "daily".into(),
                name: "日常优化".into(),
            });
        let connection = open(Path::new(":memory:")).unwrap();
        save(&connection, &workspace).unwrap();
        let saved = load(&connection).unwrap().unwrap();
        let restored = parse(&saved).unwrap();
        let categories = restored.template_categories.unwrap();
        assert_eq!(categories[0].name, "写作");
        assert_eq!(categories[1].name, "日常优化");
        let document: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(document["templates"][0]["category"], "custom");
    }

    #[test]
    fn accepts_unassigned_drafts_and_validates_explicit_project_ids() {
        let input = r#"{"schema":1,"projects":[{"id":"p","name":"测试"}],"drafts":[{"id":"d","projectId":null,"title":"","source":"原稿","result":"","favorite":false,"opened":0,"messages":[]}],"profiles":[],"activeProfile":null,"activeDraft":"d","theme":"system"}"#;
        let workspace = parse(input).unwrap();
        assert_eq!(workspace.drafts[0].project_id, None);
        let connection = open(Path::new(":memory:")).unwrap();
        save(&connection, &workspace).unwrap();
        assert_eq!(
            parse(&load(&connection).unwrap().unwrap()).unwrap().drafts[0].project_id,
            None
        );
        let assigned = input.replace(r#""projectId":null"#, r#""projectId":"p""#);
        assert_eq!(
            parse(&assigned).unwrap().drafts[0].project_id.as_deref(),
            Some("p")
        );
        assert!(
            parse(&assigned.replace(r#""projectId":"p""#, r#""projectId":"missing""#)).is_err()
        );
    }

    #[test]
    fn validates_backup_and_keeps_existing_snapshot() {
        let connection = open(Path::new(":memory:")).unwrap();
        let input = r#"{"schema":1,"projects":[{"id":"p","name":"测试"}],"drafts":[],"profiles":[],"activeProfile":null,"activeDraft":null,"theme":"system"}"#;
        let workspace = parse(input).unwrap();
        save(&connection, &workspace).unwrap();
        let before = load(&connection).unwrap();
        assert!(parse(&input.replace("system", "unsupported")).is_err());
        assert_eq!(load(&connection).unwrap(), before);
    }
}

#[cfg(test)]
mod relational_tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> Workspace {
        parse(&json!({"schema":1,"projects":[{"id":"p","name":"项目"},{"id":"other","name":"其他"}],"drafts":[{"id":"d","projectId":"p","title":"草稿","source":"原稿","result":"优化稿","favorite":false,"pinned":true,"opened":100,"updated":200,"messages":[{"id":"u","role":"user","content":"原稿","created":150,"context":{"mode":"draft","source":"原稿","templateId":"basic","templateName":"基础优化","systemPrompt":"保持原意","currentDraft":"优化稿"}},{"id":"a","role":"assistant","content":"优化稿","created":150,"durationMs":1234,"model":"deepseek-flash","providerName":"测试供应商"}]}],"profiles":[{"id":"account","name":"测试供应商","presetId":"preset-custom","website":"https://example.com","endpoint":"https://example.com/v1","model":"deepseek-flash","catalog":["deepseek-flash"]}],"activeProfile":"account","activeDraft":"d","theme":"dark","closeBehavior":"hide","shortcuts":{"newDraft":"Ctrl+KeyN","optimize":"Ctrl+Enter"},"uiState":{"recentExpanded":false,"pinnedExpanded":true,"expandedProjects":["p"],"draftListMode":"recent"},"templateCategories":[{"id":"custom","name":"用户分类"}],"templates":[{"id":"t","category":"custom","name":"用户模板","content":"系统提示词","userContent":"用户提示词","description":"说明"}]}).to_string()).unwrap()
    }
    fn patch(db: &Connection, value: serde_json::Value) -> Result<(), String> {
        apply_changes(
            db,
            &serde_json::from_value::<WorkspaceChanges>(value).unwrap(),
        )
    }
    #[test]
    fn messages_roundtrip_incremental_backup_validation_and_cascade() {
        let db = open(Path::new(":memory:")).unwrap();
        let mut document = fixture();
        document.drafts[0].template_id = Some("basic".into());
        document.drafts[0].provider_id = Some("account".into());
        document.drafts[0].model = Some("custom-model".into());
        document.profiles[0].is_default = true;
        document.default_template_id = Some("prompt-general".into());
        let state = json!({"mode":"chat","chatInput":"下一条消息"});
        document.drafts[0].dialogue = Some(serde_json::from_value(state.clone()).unwrap());
        save(&db, &document).unwrap();
        let loaded: serde_json::Value = serde_json::from_str(&load(&db).unwrap().unwrap()).unwrap();
        assert_eq!(loaded["drafts"][0]["dialogue"], state);
        assert_eq!(loaded["drafts"][0]["templateId"], "basic");
        assert_eq!(loaded["drafts"][0]["providerId"], "account");
        assert_eq!(loaded["drafts"][0]["model"], "custom-model");
        assert_eq!(loaded["profiles"][0]["isDefault"], true);
        assert_eq!(loaded["defaultTemplateId"], "prompt-general");
        assert_eq!(
            loaded["drafts"][0]["messages"],
            json!(document.drafts[0].messages)
        );
        let before = db.total_changes();
        save(&db, &document).unwrap();
        assert_eq!(before, db.total_changes());
        let backup = crate::backup::export(&db).unwrap();
        let target = open(Path::new(":memory:")).unwrap();
        crate::backup::restore(&target, &backup).unwrap();
        assert_eq!(load(&target).unwrap(), load(&db).unwrap());
        let mut broken: serde_json::Value = serde_json::from_str(&backup).unwrap();
        broken["tables"]["draft_messages"][0]["data_json"] = json!("{}");
        assert!(crate::backup::restore(&target, &broken.to_string()).is_err());
        assert_eq!(load(&target).unwrap(), load(&db).unwrap());
        let mut invalid = document.drafts[0].clone();
        invalid.messages[0].role = crate::optimizer::ConversationRole::Assistant;
        assert!(patch(&db, json!({"drafts":{"upsert":[invalid]}})).is_err());
        let mut invalid = document.drafts[0].clone();
        invalid.messages[0]
            .context
            .as_mut()
            .unwrap()
            .system_prompt
            .clear();
        assert!(patch(&db, json!({"drafts":{"upsert":[invalid]}})).is_err());
        let mut other = document.drafts[0].clone();
        other.id = "other-draft".into();
        assert!(patch(&db, json!({"drafts":{"upsert":[other]}})).is_err());
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&load(&db).unwrap().unwrap()).unwrap(),
            loaded
        );
        let mut user = document.drafts[0].messages[0].clone();
        user.id = "u2".into();
        user.context.as_mut().unwrap().mode = crate::optimizer::ModificationMode::Chat;
        let mut assistant = document.drafts[0].messages[1].clone();
        assistant.id = "a2".into();
        document.drafts[0].messages.extend([user, assistant]);
        let before = db.total_changes();
        save(&db, &document).unwrap();
        assert_eq!(
            db.total_changes() - before,
            2,
            "append only writes the two new messages"
        );
        patch(&db, json!({"settings":{"theme":"dark","activeProfile":"account","activeDraft":null},"drafts":{"remove":["d"]}})).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM draft_messages", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM draft_dialogues", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn deleting_only_a_reply_preserves_prompt_and_persists_order() {
        let db = open(Path::new(":memory:")).unwrap();
        let mut document = fixture();
        save(&db, &document).unwrap();
        let mut user = document.drafts[0].messages[0].clone();
        let mut assistant = document.drafts[0].messages.remove(1);
        user.id = "next-user".into();
        assistant.id = "next-assistant".into();
        document.drafts[0].messages.extend([user, assistant]);
        save(&db, &document).unwrap();
        let loaded = parse(&load(&db).unwrap().unwrap()).unwrap();
        assert_eq!(loaded.drafts[0].messages.len(), 3);
        assert_eq!(loaded.drafts[0].messages[0].id, "u");
        assert_eq!(loaded.drafts[0].messages[2].id, "next-assistant");
        assert_eq!(loaded.drafts[0].result, "优化稿");
        let target = open(Path::new(":memory:")).unwrap();
        crate::backup::restore(&target, &crate::backup::export(&db).unwrap()).unwrap();
        assert_eq!(load(&target).unwrap(), load(&db).unwrap());
    }

    #[test]
    fn incremental_writes_and_invalid_change_rollback() {
        let db = open(Path::new(":memory:")).unwrap();
        let document = fixture();
        save(&db, &document).unwrap();
        save_key(&db, &document.profiles[0], "fixture-secret").unwrap();
        let exported = load(&db).unwrap().unwrap();
        assert!(!exported.contains("fixture-secret"));
        let mut restored = parse(&exported).unwrap();
        assert_eq!(
            restored.drafts[0].messages[1].provider_name.as_deref(),
            Some("测试供应商")
        );
        assert_eq!(
            restored.templates.as_ref().unwrap()[0]
                .user_content
                .as_deref(),
            Some("用户提示词")
        );
        assert_eq!(restored.ui_state.as_ref().unwrap()["recentExpanded"], false);
        restored.drafts[0].source = "只改这一条草稿".into();
        let before = db.total_changes();
        let change = json!({"drafts":{"upsert":[restored.drafts[0]],"remove":[]}});
        patch(&db, change.clone()).unwrap();
        assert_eq!(db.total_changes() - before, 1);
        let before = db.total_changes();
        patch(&db, change).unwrap();
        assert_eq!(db.total_changes(), before);
        assert_eq!(
            load_key(&db, &document.profiles[0]).unwrap().as_deref(),
            Some("fixture-secret")
        );
        let before = load(&db).unwrap();
        assert!(patch(&db,json!({"projects":{"upsert":[{"id":"p","name":"不应该保存"}]},"settings":{"theme":"invalid","activeDraft":"d","activeProfile":"account"}})).is_err());
        assert_eq!(load(&db).unwrap(), before);
        assert!(patch(
            &db,
            json!({"drafts":{"upsert":[restored.drafts[0],restored.drafts[0]]}})
        )
        .is_err());
        assert_eq!(
            db.query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .unwrap(),
            None
        );
    }
    #[test]
    fn deleting_project_unassigns_draft_and_deleting_draft_removes_messages() {
        let db = open(Path::new(":memory:")).unwrap();
        save(&db, &fixture()).unwrap();
        patch(&db, json!({"projects":{"remove":["p"]}})).unwrap();
        assert_eq!(
            parse(&load(&db).unwrap().unwrap()).unwrap().drafts[0].project_id,
            None
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM draft_messages", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        patch(&db,json!({"drafts":{"remove":["d"]},"settings":{"theme":"dark","activeDraft":null,"activeProfile":"account"}})).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM draft_messages", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn provider_key_lifecycle_preserves_usage_snapshots() {
        let db = open(Path::new(":memory:")).unwrap();
        let document = fixture();
        save(&db, &document).unwrap();
        let mut profile = document.profiles[0].clone();
        save_key(&db, &profile, "fixture-secret").unwrap();
        profile.name = "新配置名称".into();
        patch(&db, json!({"profiles":{"upsert":[profile]}})).unwrap();
        assert_eq!(
            load_key(&db, &profile).unwrap().as_deref(),
            Some("fixture-secret")
        );
        profile.endpoint = "https://other.example/v1".into();
        patch(&db, json!({"profiles":{"upsert":[profile]}})).unwrap();
        assert_eq!(load_key(&db, &profile).unwrap(), None);
        save_key(&db, &profile, "new-fixture-secret").unwrap();
        let mut usage = crate::usage::Usage::default();
        usage.capture(&json!({"usage":{"prompt_tokens":100,"completion_tokens":20}}));
        crate::usage::record(&db, &profile, "optimize", "success", 10, usage).unwrap();
        let before: String = db
            .query_row("SELECT cost_details_json FROM usage_records", [], |r| {
                r.get(0)
            })
            .unwrap();
        patch(&db,json!({"profiles":{"remove":["account"]},"settings":{"theme":"dark","activeDraft":"d","activeProfile":null}})).unwrap();
        assert_eq!(load_key(&db, &profile).unwrap(), None);
        assert_eq!(
            db.query_row("SELECT provider_name FROM usage_records", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "新配置名称"
        );
        assert_eq!(
            db.query_row("SELECT cost_details_json FROM usage_records", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            before
        );
    }
    #[test]
    fn user_templates_cannot_orphan_or_override_builtin_categories() {
        let db = open(Path::new(":memory:")).unwrap();
        save(&db, &fixture()).unwrap();
        let before = load(&db).unwrap();
        assert!(patch(&db, json!({"templateCategories":{"remove":["custom"]}})).is_err());
        assert_eq!(load(&db).unwrap(), before);
        assert!(patch(
            &db,
            json!({"templateCategories":{"upsert":[{"id":"basic","name":"覆盖"}]}})
        )
        .is_err());
        assert_eq!(load(&db).unwrap(), before);
        for category in ["context", "iteration"] {
            let mut template = serde_json::to_value(&fixture().templates.unwrap()[0]).unwrap();
            template["category"] = json!(category);
            assert!(patch(&db, json!({"templates":{"upsert":[template.clone()]}})).is_err());
            assert_eq!(load(&db).unwrap(), before);
            patch(
                &db,
                json!({
                    "templateCategories":{"upsert":[{"id":category,"name":"用户分类"}]},
                    "templates":{"upsert":[template]}
                }),
            )
            .unwrap();
            patch(
                &db,
                json!({
                    "templates":{"upsert":fixture().templates.unwrap()},
                    "templateCategories":{"remove":[category]}
                }),
            )
            .unwrap();
        }
        patch(
            &db,
            json!({"templates":{"remove":["t"]},"templateCategories":{"remove":["custom"]}}),
        )
        .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM templates", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
