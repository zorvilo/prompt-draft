use super::now;
use super::providers::upsert_provider;
#[cfg(test)]
use super::{
    connection::open,
    providers::{load_key, save_key},
};
#[cfg(test)]
use crate::models::provider::Profile;
use crate::models::provider::ProviderKeyUpdate;
use crate::models::{
    validation::{invalid_shortcuts, validate_dialogue},
    workspace::*,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
pub const MAX_BACKUP: usize = 32 * 1024 * 1024;

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
        update_source:serde_json::from_value(preferences["updateSource"].clone()).map_err(|_|rusqlite::Error::InvalidQuery)?,
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
    item: &crate::models::conversation::PromptTemplate,
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
        let preferences=serde_json::json!({"defaultTemplateId":settings.default_template_id,"theme":settings.theme,"closeBehavior":settings.close_behavior,"updateSource":settings.update_source,"shortcuts":settings.shortcuts}).to_string();
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
    let invalid_categories:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM templates t WHERE category_id <> 'basic' AND NOT EXISTS(SELECT 1 FROM template_categories c WHERE c.id=t.category_id))",[],|r|r.get(0)).map_err(|_|"无法检查模板分类。")?;
    let builtin_categories: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM template_categories WHERE id = 'basic')",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "无法检查系统分类。")?;
    let builtin: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../src/config/upstream/builtin-templates.json"
    )))
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

pub fn apply_changes(
    db: &Connection,
    changes: &WorkspaceChanges,
    provider_key: Option<&ProviderKeyUpdate>,
) -> Result<(), String> {
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
    if let Some(update) = provider_key {
        super::providers::update_key(&tx, update)?;
    }
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
                update_source: document.update_source,
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
#[path = "tests/workspace.rs"]
mod tests;
