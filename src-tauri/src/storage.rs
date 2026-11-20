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
pub struct Version {
    id: String,
    source: String,
    result: String,
    question: Option<String>,
    model: String,
    #[serde(
        default,
        rename = "providerName",
        skip_serializing_if = "Option::is_none"
    )]
    provider_name: Option<String>,
    created: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instruction: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    id: String,
    project_id: Option<String>,
    title: String,
    source: String,
    result: String,
    question: Option<String>,
    favorite: bool,
    #[serde(default)]
    pinned: bool,
    opened: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated: Option<u64>,
    versions: Vec<Version>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
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
    #[serde(default = "shortcut_history")]
    history: String,
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
fn shortcut_history() -> String {
    "Ctrl+Alt+KeyH".into()
}
fn invalid_shortcuts(keys: &Shortcuts) -> bool {
    let values = [
        &keys.new_draft,
        &keys.optimize,
        &keys.rename,
        &keys.pin,
        &keys.favorite,
        &keys.delete,
        &keys.copy_result,
        &keys.history,
    ];
    values.iter().any(|value| !valid_shortcut(value))
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
    if ctrl.is_none() && alt.is_none() {
        return false;
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

pub fn parse(input: &str) -> Result<Workspace, String> {
    if input.len() > MAX_BACKUP {
        return Err("工作区数据超过 32 MB，请先导出并归档部分稿件。".into());
    }
    let workspace: Workspace =
        serde_json::from_str(input).map_err(|_| "无法读取词稿备份，请选择有效的 JSON 文件。")?;
    let projects: HashSet<_> = workspace.projects.iter().map(|item| &item.id).collect();
    let drafts: HashSet<_> = workspace.drafts.iter().map(|item| &item.id).collect();
    let profiles: HashSet<_> = workspace.profiles.iter().map(|item| &item.id).collect();
    let mut version_ids = HashSet::new();
    if workspace
        .drafts
        .iter()
        .flat_map(|draft| &draft.versions)
        .any(|version| version.id.is_empty() || !version_ids.insert(&version.id))
    {
        return Err("版本 ID 为空或重复。".into());
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
    let mut drafts:Vec<Draft>=json_rows(connection,"SELECT json_object('id',id,'projectId',project_id,'title',title,'source',source,'result',result,'question',question,'favorite',json(CASE WHEN is_favorite THEN 'true' ELSE 'false' END),'pinned',json(CASE WHEN is_pinned THEN 'true' ELSE 'false' END),'opened',created_at,'updated',updated_at,'versions',json('[]')) FROM drafts ORDER BY created_at,rowid")?;
    let mut versions: HashMap<String, Vec<Version>> = HashMap::new();
    for row in connection.prepare("SELECT draft_id,json_object('id',id,'source',source,'result',result,'question',question,'instruction',instruction,'model',model_id,'providerName',provider_name,'created',created_at) FROM draft_versions ORDER BY created_at DESC,rowid DESC")?.query_map([],|r|Ok((r.get::<_,String>(0)?,decode::<Version>(r.get(1)?)?)))? {
        let (draft_id,version)=row?;
        versions.entry(draft_id).or_default().push(version);
    }
    for draft in &mut drafts {
        draft.versions = versions.remove(&draft.id).unwrap_or_default();
    }
    let workspace=Workspace {
        schema:1,
        projects:json_rows(connection,"SELECT json_object('id',id,'name',name) FROM projects ORDER BY created_at,rowid")?,
        drafts,
        profiles:json_rows(connection,"SELECT json_object('id',id,'name',name,'endpoint',endpoint,'presetId',preset_id,'website',website,'model',default_model,'catalog',json(models_json)) FROM providers ORDER BY created_at,rowid")?,
        templates:Some(json_rows(connection,"SELECT json_object('id',id,'category',category_id,'strategy',strategy,'name',name,'content',system_prompt,'userContent',user_prompt,'description',description) FROM templates ORDER BY created_at,rowid")?),
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
    let models = item
        .catalog
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap());
    db.execute("INSERT INTO providers(id,name,preset_id,website,endpoint,default_model,models_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8)
        ON CONFLICT(id) DO UPDATE SET name=excluded.name,preset_id=excluded.preset_id,website=excluded.website,endpoint=excluded.endpoint,default_model=excluded.default_model,models_json=excluded.models_json,
        api_key=CASE WHEN providers.endpoint=excluded.endpoint THEN providers.api_key ELSE '' END,updated_at=excluded.updated_at
        WHERE (name,preset_id,website,endpoint,default_model,models_json) IS NOT (excluded.name,excluded.preset_id,excluded.website,excluded.endpoint,excluded.default_model,excluded.models_json)",params![item.id,item.name,item.preset_id,item.website,item.endpoint,item.model,models,now()])?;
    Ok(())
}
fn upsert_draft(db: &Connection, item: &Draft) -> rusqlite::Result<()> {
    db.execute("INSERT INTO drafts VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET project_id=excluded.project_id,title=excluded.title,source=excluded.source,result=excluded.result,question=excluded.question,is_pinned=excluded.is_pinned,is_favorite=excluded.is_favorite,updated_at=excluded.updated_at
        WHERE (project_id,title,source,result,question,is_pinned,is_favorite,updated_at) IS NOT (excluded.project_id,excluded.title,excluded.source,excluded.result,excluded.question,excluded.is_pinned,excluded.is_favorite,excluded.updated_at)",params![item.id,item.project_id,item.title,item.source,item.result,item.question,item.pinned,item.favorite,item.opened,item.updated.unwrap_or(item.opened)])?;
    let ids: Vec<_> = item.versions.iter().map(|v| &v.id).collect();
    db.execute("DELETE FROM draft_versions WHERE draft_id=?1 AND id NOT IN(SELECT value FROM json_each(?2))",params![item.id,serde_json::to_string(&ids).unwrap()])?;
    for version in item.versions.iter().rev() {
        let occupied: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM draft_versions WHERE id=?1 AND draft_id<>?2)",
            params![version.id, item.id],
            |r| r.get(0),
        )?;
        if occupied {
            return Err(rusqlite::Error::InvalidQuery);
        }
        db.execute("INSERT INTO draft_versions VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET source=excluded.source,result=excluded.result,question=excluded.question,instruction=excluded.instruction,provider_name=excluded.provider_name,model_id=excluded.model_id,created_at=excluded.created_at
            WHERE draft_id=excluded.draft_id AND (source,result,question,instruction,provider_name,model_id,created_at) IS NOT (excluded.source,excluded.result,excluded.question,excluded.instruction,excluded.provider_name,excluded.model_id,excluded.created_at)",params![version.id,item.id,version.source,version.result,version.question,version.instruction,version.provider_name.as_deref().unwrap_or(""),version.model,version.created])?;
    }
    Ok(())
}
fn upsert_template(
    db: &Connection,
    item: &crate::optimizer::PromptTemplate,
) -> rusqlite::Result<()> {
    db.execute("INSERT INTO templates VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8) ON CONFLICT(id) DO UPDATE SET category_id=excluded.category_id,name=excluded.name,description=excluded.description,system_prompt=excluded.system_prompt,user_prompt=excluded.user_prompt,strategy=excluded.strategy,updated_at=excluded.updated_at
        WHERE (category_id,name,description,system_prompt,user_prompt,strategy) IS NOT (excluded.category_id,excluded.name,excluded.description,excluded.system_prompt,excluded.user_prompt,excluded.strategy)",params![item.id,item.category.as_deref().unwrap_or("basic"),item.name,item.description,item.content,item.user_content,serde_json::to_value(&item.strategy).unwrap().as_str().unwrap(),now()])?;
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
        let preferences=serde_json::json!({"theme":settings.theme,"closeBehavior":settings.close_behavior,"shortcuts":settings.shortcuts}).to_string();
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
    let invalid_categories:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM templates t WHERE category_id NOT IN ('basic','context','iteration') AND NOT EXISTS(SELECT 1 FROM template_categories c WHERE c.id=t.category_id))",[],|r|r.get(0)).map_err(|_|"无法检查模板分类。")?;
    let builtin_categories:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM template_categories WHERE id IN ('basic','context','iteration'))",[],|r|r.get(0)).map_err(|_|"无法检查系统分类。")?;
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
    let mut version_ids = HashSet::new();
    if let Some(items) = &changes.drafts {
        for draft in &items.upsert {
            for version in &draft.versions {
                if version.id.is_empty() || !version_ids.insert(&version.id) {
                    return Err("版本 ID 为空或重复。".into());
                }
            }
        }
    }
    let tx = db.unchecked_transaction().map_err(|_| "无法开始保存。")?;
    persist(&tx, changes).map_err(|_| "保存失败，请检查数据关联和磁盘空间。")?;
    validate_state(&tx)?;
    tx.commit().map_err(|_| "无法完成保存。".into())
}
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
        let names: Vec<String>=db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([],|r|r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(
            names,
            vec![
                "app_settings",
                "draft_versions",
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
        document["shortcuts"] =
            serde_json::json!({"newDraft":"Ctrl+Shift+KeyN","optimize":"Alt+Enter"});
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
        ] {
            assert!(valid_shortcut(value), "{value}");
        }
        for value in [
            "KeyN",
            "Shift+KeyN",
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
        let input = r#"{"schema":1,"projects":[{"id":"p","name":"测试"}],"drafts":[],"profiles":[],"activeProfile":null,"activeDraft":null,"theme":"system","templateCategories":[{"id":"custom","name":"写作"}],"templates":[{"id":"t","category":"custom","strategy":"general","name":"测试","content":"改写","description":""}]}"#;
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
        let input = r#"{"schema":1,"projects":[{"id":"p","name":"测试"}],"drafts":[{"id":"d","projectId":null,"title":"","source":"原稿","result":"","favorite":false,"opened":0,"versions":[]}],"profiles":[],"activeProfile":null,"activeDraft":"d","theme":"system"}"#;
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
        parse(&json!({"schema":1,"projects":[{"id":"p","name":"项目"},{"id":"other","name":"其他"}],"drafts":[{"id":"d","projectId":"p","title":"草稿","source":"原稿","result":"优化稿","question":null,"favorite":false,"pinned":true,"opened":100,"updated":200,"versions":[{"id":"v","source":"原稿","result":"版本正文","question":null,"instruction":"精简","model":"deepseek-flash","providerName":"测试供应商","created":150}]}],"profiles":[{"id":"account","name":"测试供应商","presetId":"preset-custom","website":"https://example.com","endpoint":"https://example.com/v1","model":"deepseek-flash","catalog":["deepseek-flash"]}],"activeProfile":"account","activeDraft":"d","theme":"dark","closeBehavior":"hide","shortcuts":{"newDraft":"Ctrl+KeyN","optimize":"Ctrl+Enter"},"uiState":{"recentExpanded":false,"pinnedExpanded":true,"expandedProjects":["p"],"draftListMode":"recent"},"templateCategories":[{"id":"custom","name":"用户分类"}],"templates":[{"id":"t","category":"custom","strategy":"general","name":"用户模板","content":"系统提示词","userContent":"用户提示词","description":"说明"}]}).to_string()).unwrap()
    }
    fn patch(db: &Connection, value: serde_json::Value) -> Result<(), String> {
        apply_changes(
            db,
            &serde_json::from_value::<WorkspaceChanges>(value).unwrap(),
        )
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
            restored.drafts[0].versions[0].provider_name.as_deref(),
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
    fn deleting_project_unassigns_draft_and_deleting_draft_removes_versions() {
        let db = open(Path::new(":memory:")).unwrap();
        save(&db, &fixture()).unwrap();
        patch(&db, json!({"projects":{"remove":["p"]}})).unwrap();
        assert_eq!(
            parse(&load(&db).unwrap().unwrap()).unwrap().drafts[0].project_id,
            None
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM draft_versions", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        patch(&db,json!({"drafts":{"remove":["d"]},"settings":{"theme":"dark","activeDraft":null,"activeProfile":"account"}})).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM draft_versions", [], |r| r
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
