use super::*;
use crate::models::validation::valid_shortcut;
#[cfg(test)]
mod settings_and_keys {
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
        let prices = crate::database::pricing::catalog_rows(&db).unwrap();
        assert_eq!(prices.len(), 83);
        crate::database::pricing::initialize(&db).unwrap();
        assert_eq!(crate::database::pricing::catalog_rows(&db).unwrap(), prices);
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
        document["updateSource"] = serde_json::json!("github");
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
        assert!(matches!(
            parse(&saved).unwrap().update_source,
            Some(crate::models::update::UpdateSource::Github)
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
            None,
        )
    }
    #[test]
    fn deleting_project_unassigns_drafts_without_removing_messages_or_active_selection() {
        let db = open(Path::new(":memory:")).unwrap();
        let mut document = fixture();
        document.projects.retain(|project| project.id == "p");
        save(&db, &document).unwrap();
        let before: serde_json::Value = serde_json::from_str(&load(&db).unwrap().unwrap()).unwrap();
        patch(&db, json!({"projects":{"remove":["p"]}})).unwrap();
        let after: serde_json::Value = serde_json::from_str(&load(&db).unwrap().unwrap()).unwrap();
        assert_eq!(after["projects"], json!([]));
        parse(&after.to_string()).unwrap();
        assert!(after["drafts"][0]["projectId"].is_null());
        assert_eq!(
            after["drafts"][0]["messages"],
            before["drafts"][0]["messages"]
        );
        assert_eq!(after["drafts"][0]["result"], before["drafts"][0]["result"]);
        assert_eq!(after["activeDraft"], before["activeDraft"]);
    }

    #[test]
    fn provider_metadata_and_key_commit_or_rollback_together() {
        let db = open(Path::new(":memory:")).unwrap();
        let document = fixture();
        save(&db, &document).unwrap();
        let original = &document.profiles[0];
        save_key(&db, original, "fixture-old-key").unwrap();
        let before = load(&db).unwrap();
        let mut profile = original.clone();
        profile.endpoint = "https://new-provider.example/v1".into();
        profile.name = "修改后的供应商".into();
        let key: crate::models::provider::ProviderKeyUpdate = serde_json::from_value(json!({
            "providerId":profile.id, "endpoint":profile.endpoint, "key":"fixture-new-key"
        }))
        .unwrap();
        let invalid: WorkspaceChanges = serde_json::from_value(json!({
            "profiles":{"upsert":[profile]}, "settings":{"theme":"invalid"}
        }))
        .unwrap();
        let error = apply_changes(&db, &invalid, Some(&key)).unwrap_err();
        assert!(!error.contains("fixture-new-key"));
        assert_eq!(load(&db).unwrap(), before);
        assert_eq!(
            load_key(&db, original).unwrap().as_deref(),
            Some("fixture-old-key")
        );
        assert_eq!(load_key(&db, &profile).unwrap(), None);

        let changes: WorkspaceChanges =
            serde_json::from_value(json!({"profiles":{"upsert":[profile]}})).unwrap();
        let wrong_endpoint = serde_json::from_value(json!({"providerId":profile.id,"endpoint":"https://wrong.example","key":"fixture-new-key"})).unwrap();
        assert!(apply_changes(&db, &changes, Some(&wrong_endpoint)).is_err());
        assert_eq!(load(&db).unwrap(), before);

        apply_changes(&db, &changes, Some(&key)).unwrap();
        assert_eq!(load_key(&db, original).unwrap(), None);
        assert_eq!(
            load_key(&db, &profile).unwrap().as_deref(),
            Some("fixture-new-key")
        );
        let snapshot = load(&db).unwrap().unwrap();
        assert!(!snapshot.contains("fixture-new-key"));
        assert!(snapshot.contains("修改后的供应商"));

        let key_only = serde_json::from_value(json!({"providerId":profile.id,"endpoint":profile.endpoint,"key":"fixture-rotated-key"})).unwrap();
        apply_changes(&db, &WorkspaceChanges::default(), Some(&key_only)).unwrap();
        assert_eq!(
            load_key(&db, &profile).unwrap().as_deref(),
            Some("fixture-rotated-key")
        );
    }
    #[test]
    fn messages_roundtrip_incremental_backup_validation_and_cascade() {
        let db = open(Path::new(":memory:")).unwrap();
        let mut document = fixture();
        document.drafts[0].template_id = Some("basic".into());
        document.drafts[0].provider_id = Some("account".into());
        document.drafts[0].model = Some("custom-model".into());
        document.profiles[0].is_default = true;
        document.drafts[0].messages[1].input_tokens = Some(200000);
        document.drafts[0].messages[1].context_warning_shown = Some(true);
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
        let backup = crate::services::backup::export(&db).unwrap();
        let target = open(Path::new(":memory:")).unwrap();
        crate::services::backup::restore(&target, &backup).unwrap();
        assert_eq!(load(&target).unwrap(), load(&db).unwrap());
        let mut broken: serde_json::Value = serde_json::from_str(&backup).unwrap();
        broken["tables"]["draft_messages"][0]["data_json"] = json!("{}");
        assert!(crate::services::backup::restore(&target, &broken.to_string()).is_err());
        assert_eq!(load(&target).unwrap(), load(&db).unwrap());
        let mut invalid = document.drafts[0].clone();
        invalid.messages[0].role = crate::models::conversation::ConversationRole::Assistant;
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
        user.context.as_mut().unwrap().mode = crate::models::conversation::ModificationMode::Chat;
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
        crate::services::backup::restore(&target, &crate::services::backup::export(&db).unwrap())
            .unwrap();
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
        let mut usage = crate::models::usage::Usage::default();
        usage.capture(&json!({"usage":{"prompt_tokens":100,"completion_tokens":20}}));
        crate::database::usage::record(&db, &profile, "optimize", "success", 10, usage).unwrap();
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
