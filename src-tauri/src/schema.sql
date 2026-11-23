PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS projects(id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS drafts(
    id TEXT PRIMARY KEY, project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    title TEXT NOT NULL, source TEXT NOT NULL, result TEXT NOT NULL,
    is_pinned INTEGER NOT NULL CHECK(is_pinned IN (0,1)), is_favorite INTEGER NOT NULL CHECK(is_favorite IN (0,1)),
    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS drafts_project ON drafts(project_id);
CREATE INDEX IF NOT EXISTS drafts_updated ON drafts(updated_at DESC);
CREATE TABLE IF NOT EXISTS draft_messages(
    id TEXT PRIMARY KEY, draft_id TEXT NOT NULL REFERENCES drafts(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position>=0), data_json TEXT NOT NULL CHECK(json_valid(data_json)),
    UNIQUE(draft_id,position)
);
CREATE TABLE IF NOT EXISTS draft_dialogues(
    draft_id TEXT PRIMARY KEY REFERENCES drafts(id) ON DELETE CASCADE,
    state_json TEXT NOT NULL CHECK(json_valid(state_json))
);
CREATE TABLE IF NOT EXISTS template_categories(id TEXT PRIMARY KEY,name TEXT NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS templates(
    id TEXT PRIMARY KEY,category_id TEXT NOT NULL,name TEXT NOT NULL,description TEXT NOT NULL,
    system_prompt TEXT NOT NULL,user_prompt TEXT,strategy TEXT NOT NULL,
    created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS templates_category ON templates(category_id);
CREATE TABLE IF NOT EXISTS providers(
    id TEXT PRIMARY KEY,name TEXT NOT NULL,preset_id TEXT,website TEXT,endpoint TEXT NOT NULL,
    default_model TEXT NOT NULL,api_key TEXT NOT NULL DEFAULT '',models_json TEXT,
    created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS usage_records(
    id INTEGER PRIMARY KEY,created_at INTEGER NOT NULL,provider_id TEXT NOT NULL,provider_name TEXT NOT NULL,
    requested_model TEXT NOT NULL,billing_model TEXT NOT NULL,operation TEXT NOT NULL,status TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,input_tokens INTEGER,output_tokens INTEGER,cache_read_tokens INTEGER,cache_write_tokens INTEGER,
    endpoint_scope TEXT NOT NULL,cost_cny TEXT,cost_details_json TEXT
);
CREATE INDEX IF NOT EXISTS usage_records_created ON usage_records(created_at);
CREATE TABLE IF NOT EXISTS model_prices(
    endpoint_scope TEXT NOT NULL,model_id TEXT NOT NULL,layer TEXT NOT NULL CHECK(layer IN ('base','override')),
    input_price TEXT NOT NULL,output_price TEXT NOT NULL,cache_read_price TEXT NOT NULL,cache_write_price TEXT NOT NULL,
    source TEXT NOT NULL,synced_at INTEGER,PRIMARY KEY(endpoint_scope,model_id,layer)
);
CREATE TABLE IF NOT EXISTS pricing_settings(id INTEGER PRIMARY KEY CHECK(id=1),usd_to_cny_rate TEXT NOT NULL,
    rate_date TEXT NOT NULL,last_synced_at INTEGER,last_sync_error TEXT);
CREATE TABLE IF NOT EXISTS app_settings(id INTEGER PRIMARY KEY CHECK(id=1),preferences_json TEXT NOT NULL,ui_state_json TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS models_dev_cache(id INTEGER PRIMARY KEY CHECK(id=1),content_json TEXT NOT NULL,fetched_at INTEGER NOT NULL);
CREATE VIEW IF NOT EXISTS usage_effective_prices AS
    SELECT u.id,
        coalesce(o.input_price,g.input_price,m.input_price,b.input_price) AS input_price,
        coalesce(o.output_price,g.output_price,m.output_price,b.output_price) AS output_price,
        coalesce(o.cache_read_price,g.cache_read_price,m.cache_read_price,b.cache_read_price) AS cache_read_price,
        coalesce(o.cache_write_price,g.cache_write_price,m.cache_write_price,b.cache_write_price) AS cache_write_price
    FROM usage_records u
    LEFT JOIN model_prices o ON o.endpoint_scope<>'' AND o.endpoint_scope=u.endpoint_scope AND o.model_id=lower(u.billing_model) AND o.layer='override'
    LEFT JOIN model_prices g ON g.endpoint_scope='' AND g.model_id=lower(u.billing_model) AND g.layer='override'
    LEFT JOIN model_prices m ON m.endpoint_scope<>'' AND m.endpoint_scope=u.endpoint_scope AND m.model_id=lower(u.billing_model) AND m.layer='base'
    LEFT JOIN model_prices b ON b.endpoint_scope='' AND b.model_id=lower(u.billing_model) AND b.layer='base';
