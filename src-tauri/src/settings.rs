use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{Map, Value as JsonValue};

use crate::{
    backup, db, entries, images,
    models::{
        CapsuleConfigResponse, CapsuleConfigValue, ConfigMutationResponse, Entry, EntryFilters,
        ExportEntriesRequest, ExportEntriesResponse, ExportFormat, LibraryListResponse,
        LibraryPrompt, LibraryPromptInput, LibraryPromptMutationResponse, LibraryPromptUpdate,
        LibraryTemplate, LibraryTemplateInput, LibraryTemplateMutationResponse,
        LibraryTemplateUpdate, LocationConfigUpdateRequest, MoodCatalogResponse, MoodDeleteRequest,
        MoodMutationResponse, MoodRenameRequest, MoodUsage, PathSettingsResponse,
        PathSettingsUpdateRequest, TagCatalogResponse, TagDeleteRequest, TagMergeRequest,
        TagMutationResponse, TagRenameRequest, TagUsage,
    },
    search,
};

const CONFIG_BACKUP_PREFIX: &str = "config_backup_";
const EXPORT_PREFIX: &str = "capsule_export_";

pub fn get_capsule_config() -> Result<CapsuleConfigResponse> {
    get_capsule_config_for_database(&db::resolve_database_path())
}

pub(crate) fn get_capsule_config_for_database(db_path: &Path) -> Result<CapsuleConfigResponse> {
    let config_path = config_path_for_database(db_path);
    let exists = config_path.exists();
    let mut warnings = Vec::new();
    let mut values = Vec::new();

    if exists {
        match read_config_object(&config_path) {
            Ok(object) => {
                values = object
                    .into_iter()
                    .map(|(key, value)| CapsuleConfigValue {
                        key,
                        value: config_value_to_string(&value),
                    })
                    .collect();
                values.sort_by(|left, right| left.key.cmp(&right.key));
            }
            Err(error) => warnings.push(format!("Unable to read config JSON: {error}")),
        }
    } else {
        warnings.push("No config.json was found next to the active database.".to_string());
    }

    Ok(CapsuleConfigResponse {
        config_path: db::path_to_string(&config_path),
        exists,
        values,
        warnings,
    })
}

pub fn set_capsule_config_value(key: String, value: String) -> Result<ConfigMutationResponse> {
    mutate_capsule_config("config.set", |object| {
        let key = normalize_config_key(&key)?;
        object.insert(key, JsonValue::String(value));
        Ok(())
    })
}

pub fn delete_capsule_config_value(key: String) -> Result<ConfigMutationResponse> {
    mutate_capsule_config("config.delete", |object| {
        let key = normalize_config_key(&key)?;
        object.remove(&key);
        Ok(())
    })
}

pub fn set_location_config(input: LocationConfigUpdateRequest) -> Result<ConfigMutationResponse> {
    mutate_capsule_config("config.location.set", |object| {
        apply_location_config(object, input)?;
        Ok(())
    })
}

pub fn get_path_settings() -> Result<PathSettingsResponse> {
    let mut warnings = Vec::new();
    if let Err(error) = db::try_read_local_path_settings() {
        warnings.push(format!("Unable to read saved path settings: {error}"));
    }
    if env::var("CAPSULE_DB_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
    {
        warnings.push("CAPSULE_DB_PATH is set and overrides the saved database path.".to_string());
    }
    if env::var("CAPSULE_IMAGES_MEDIA_ROOT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
    {
        warnings.push(
            "CAPSULE_IMAGES_MEDIA_ROOT is set and overrides the saved image path.".to_string(),
        );
    }
    if env::var("CAPSULE_BACKUP_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
    {
        warnings.push("CAPSULE_BACKUP_DIR is set and overrides the saved backup path.".to_string());
    }
    if env::var("CAPSULE_SYNC_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
    {
        warnings.push("CAPSULE_SYNC_PATH is set and overrides the saved sync path.".to_string());
    }

    let db_path = db::resolve_database_path();
    let backup_directory = db::backup_directory_for_database(&db_path);
    let local_settings = db::read_local_path_settings();
    let sync_path = env::var("CAPSULE_SYNC_PATH")
        .ok()
        .and_then(|value| normalize_string(Some(&value)))
        .or_else(|| local_settings.sync_path.clone());

    Ok(PathSettingsResponse {
        database_path: db::path_to_string(&db_path),
        image_media_root: images::get_image_media_root()?,
        backup_directory: db::path_to_string(&backup_directory),
        sync_path,
        auto_sync_enabled: local_settings.auto_sync_enabled.unwrap_or(false),
        auto_sync_interval_minutes: local_settings
            .auto_sync_interval_minutes
            .unwrap_or(15)
            .clamp(1, 24 * 60),
        settings_path: db::path_to_string(&db::local_path_settings_path()),
        warnings,
    })
}

pub fn set_path_settings(input: PathSettingsUpdateRequest) -> Result<PathSettingsResponse> {
    let mut settings = db::read_local_path_settings();
    settings.database_path = normalize_string(input.database_path.as_deref());
    settings.image_media_root = normalize_string(input.image_media_root.as_deref());
    settings.backup_directory = normalize_string(input.backup_directory.as_deref());
    settings.sync_path = normalize_string(input.sync_path.as_deref());
    settings.auto_sync_enabled = input.auto_sync_enabled;
    settings.auto_sync_interval_minutes = input
        .auto_sync_interval_minutes
        .map(|minutes| minutes.clamp(1, 24 * 60));

    if let Some(path) = settings.image_media_root.as_deref() {
        fs::create_dir_all(path).with_context(|| format!("failed to create image path {path}"))?;
    }
    if let Some(path) = settings.backup_directory.as_deref() {
        fs::create_dir_all(path).with_context(|| format!("failed to create backup path {path}"))?;
    }
    if let Some(path) = settings.sync_path.as_deref() {
        fs::create_dir_all(path).with_context(|| format!("failed to create sync path {path}"))?;
    }

    db::write_local_path_settings(&settings)?;
    get_path_settings()
}

pub fn browse_database_path(current_path: Option<String>) -> Result<Option<String>> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Select Capsule database")
        .add_filter("SQLite databases", &["db", "sqlite", "sqlite3"])
        .add_filter("All files", &["*"]);
    if let Some(directory) = dialog_start_directory(current_path.as_deref(), true) {
        dialog = dialog.set_directory(directory);
    }
    Ok(dialog.pick_file().map(|path| db::path_to_string(&path)))
}

pub fn browse_directory_path(current_path: Option<String>) -> Result<Option<String>> {
    let mut dialog = rfd::FileDialog::new().set_title("Select folder");
    if let Some(directory) = dialog_start_directory(current_path.as_deref(), false) {
        dialog = dialog.set_directory(directory);
    }
    Ok(dialog.pick_folder().map(|path| db::path_to_string(&path)))
}

pub fn browse_image_path(current_path: Option<String>) -> Result<Option<String>> {
    Ok(image_file_dialog(current_path.as_deref())
        .pick_file()
        .map(|path| db::path_to_string(&path)))
}

pub fn browse_image_paths(current_path: Option<String>) -> Result<Vec<String>> {
    Ok(image_file_dialog(current_path.as_deref())
        .pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|path| db::path_to_string(&path))
        .collect())
}

pub fn list_tags() -> Result<TagCatalogResponse> {
    list_tags_for_database(&db::resolve_database_path())
}

pub(crate) fn list_tags_for_database(db_path: &Path) -> Result<TagCatalogResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    if !tables.contains("tags") {
        return Ok(TagCatalogResponse {
            tags: Vec::new(),
            warnings: vec!["The tags table was not found.".to_string()],
        });
    }

    let sql = if tables.contains("entry_tags") {
        "SELECT t.id, t.name, COUNT(et.entry_id) AS entry_count
         FROM tags t
         LEFT JOIN entry_tags et ON et.tag_id = t.id
         GROUP BY t.id, t.name
         ORDER BY lower(t.name) ASC"
    } else {
        "SELECT t.id, t.name, 0 AS entry_count
         FROM tags t
         ORDER BY lower(t.name) ASC"
    };
    let mut statement = connection.prepare(sql)?;
    let tags = statement
        .query_map([], |row| {
            Ok(TagUsage {
                id: row.get(0)?,
                name: row.get(1)?,
                entry_count: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(TagCatalogResponse {
        tags,
        warnings: Vec::new(),
    })
}

pub fn rename_tag(input: TagRenameRequest) -> Result<TagMutationResponse> {
    let guarded = backup::with_database_backup("tag.rename", move |db_path| {
        rename_tag_inner(db_path, input)?;
        list_tags_for_database(db_path).map(|response| response.tags)
    })?;
    Ok(TagMutationResponse {
        tags: guarded.value,
        audit: guarded.audit,
    })
}

pub fn merge_tag(input: TagMergeRequest) -> Result<TagMutationResponse> {
    let guarded = backup::with_database_backup("tag.merge", move |db_path| {
        merge_tag_inner(db_path, input)?;
        list_tags_for_database(db_path).map(|response| response.tags)
    })?;
    Ok(TagMutationResponse {
        tags: guarded.value,
        audit: guarded.audit,
    })
}

pub fn delete_tag(input: TagDeleteRequest) -> Result<TagMutationResponse> {
    let guarded = backup::with_database_backup("tag.delete", move |db_path| {
        delete_tag_inner(db_path, input)?;
        list_tags_for_database(db_path).map(|response| response.tags)
    })?;
    Ok(TagMutationResponse {
        tags: guarded.value,
        audit: guarded.audit,
    })
}

pub fn list_moods() -> Result<MoodCatalogResponse> {
    list_moods_for_database(&db::resolve_database_path())
}

pub(crate) fn list_moods_for_database(db_path: &Path) -> Result<MoodCatalogResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    if !tables.contains("entries") {
        return Ok(MoodCatalogResponse {
            moods: Vec::new(),
            warnings: vec!["The entries table was not found.".to_string()],
        });
    }

    let mut statement = connection.prepare(
        "SELECT mood, COUNT(*) AS entry_count
         FROM entries
         WHERE mood IS NOT NULL AND trim(mood) != ''
         GROUP BY mood
         ORDER BY lower(mood) ASC",
    )?;
    let moods = statement
        .query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(MoodUsage {
                label: labelize(&name),
                name,
                entry_count: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(MoodCatalogResponse {
        moods,
        warnings: Vec::new(),
    })
}

pub fn rename_mood(input: MoodRenameRequest) -> Result<MoodMutationResponse> {
    let guarded = backup::with_database_backup("mood.rename", move |db_path| {
        rename_mood_inner(db_path, input)?;
        list_moods_for_database(db_path).map(|response| response.moods)
    })?;
    Ok(MoodMutationResponse {
        moods: guarded.value,
        audit: guarded.audit,
    })
}

pub fn delete_mood(input: MoodDeleteRequest) -> Result<MoodMutationResponse> {
    let guarded = backup::with_database_backup("mood.delete", move |db_path| {
        delete_mood_inner(db_path, input)?;
        list_moods_for_database(db_path).map(|response| response.moods)
    })?;
    Ok(MoodMutationResponse {
        moods: guarded.value,
        audit: guarded.audit,
    })
}

pub fn list_library_items() -> Result<LibraryListResponse> {
    list_library_items_for_database(&db::resolve_database_path())
}

pub(crate) fn list_library_items_for_database(db_path: &Path) -> Result<LibraryListResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    let mut warnings = Vec::new();

    let templates = if tables.contains("library_templates") {
        list_templates(&connection)?
    } else {
        warnings.push("The template library table was not found.".to_string());
        Vec::new()
    };
    let prompts = if tables.contains("library_prompts") {
        list_prompts(&connection)?
    } else {
        warnings.push("The prompt library table was not found.".to_string());
        Vec::new()
    };

    Ok(LibraryListResponse {
        templates,
        prompts,
        warnings,
    })
}

pub fn create_template(input: LibraryTemplateInput) -> Result<LibraryTemplateMutationResponse> {
    let guarded = backup::with_database_backup("library.template.create", move |db_path| {
        create_template_inner(db_path, input)
    })?;
    Ok(LibraryTemplateMutationResponse {
        template: Some(guarded.value),
        audit: guarded.audit,
    })
}

pub fn update_template(
    slug: String,
    input: LibraryTemplateUpdate,
) -> Result<LibraryTemplateMutationResponse> {
    let guarded = backup::with_database_backup("library.template.update", move |db_path| {
        update_template_inner(db_path, &slug, input)
    })?;
    Ok(LibraryTemplateMutationResponse {
        template: Some(guarded.value),
        audit: guarded.audit,
    })
}

pub fn delete_template(slug: String) -> Result<LibraryTemplateMutationResponse> {
    let guarded = backup::with_database_backup("library.template.delete", move |db_path| {
        delete_template_inner(db_path, &slug)
    })?;
    Ok(LibraryTemplateMutationResponse {
        template: None,
        audit: guarded.audit,
    })
}

pub fn create_prompt(input: LibraryPromptInput) -> Result<LibraryPromptMutationResponse> {
    let guarded = backup::with_database_backup("library.prompt.create", move |db_path| {
        create_prompt_inner(db_path, input)
    })?;
    Ok(LibraryPromptMutationResponse {
        prompt: Some(guarded.value),
        audit: guarded.audit,
    })
}

pub fn update_prompt(
    slug: String,
    input: LibraryPromptUpdate,
) -> Result<LibraryPromptMutationResponse> {
    let guarded = backup::with_database_backup("library.prompt.update", move |db_path| {
        update_prompt_inner(db_path, &slug, input)
    })?;
    Ok(LibraryPromptMutationResponse {
        prompt: Some(guarded.value),
        audit: guarded.audit,
    })
}

pub fn delete_prompt(slug: String) -> Result<LibraryPromptMutationResponse> {
    let guarded = backup::with_database_backup("library.prompt.delete", move |db_path| {
        delete_prompt_inner(db_path, &slug)
    })?;
    Ok(LibraryPromptMutationResponse {
        prompt: None,
        audit: guarded.audit,
    })
}

pub fn export_entries(input: ExportEntriesRequest) -> Result<ExportEntriesResponse> {
    let db_path = db::resolve_database_path();
    export_entries_for_database(&db_path, input)
}

pub(crate) fn export_entries_for_database(
    db_path: &Path,
    input: ExportEntriesRequest,
) -> Result<ExportEntriesResponse> {
    let entries = load_entries_for_export(db_path, input.uuids, input.search, input.filters)?;
    let created_at = Utc::now().to_rfc3339();
    let export_directory = db::backup_directory_for_database(db_path).join("exports");
    fs::create_dir_all(&export_directory)
        .with_context(|| format!("failed to create {}", export_directory.display()))?;

    let extension = match input.format {
        ExportFormat::Markdown => "md",
        ExportFormat::Json => "json",
    };
    let stem = input
        .file_name
        .as_deref()
        .and_then(normalize_file_stem)
        .unwrap_or_else(|| format!("{EXPORT_PREFIX}{}", Utc::now().format("%Y%m%d_%H%M%S")));
    let path = next_available_export_path(&export_directory, &stem, extension);
    let content = match input.format {
        ExportFormat::Markdown => entries_to_markdown(&entries),
        ExportFormat::Json => serde_json::to_string_pretty(&entries)?,
    };
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(ExportEntriesResponse {
        path: db::path_to_string(&path),
        format: input.format,
        entry_count: entries.len(),
        created_at,
    })
}

fn mutate_capsule_config(
    operation: &str,
    mutate: impl FnOnce(&mut Map<String, JsonValue>) -> Result<()>,
) -> Result<ConfigMutationResponse> {
    let db_path = db::resolve_database_path();
    let config_path = config_path_for_database(&db_path);
    let mut object = if config_path.exists() {
        read_config_object(&config_path)?
    } else {
        Map::new()
    };
    let backup_path = create_config_backup(&config_path)?;

    mutate(&mut object)?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(
        &config_path,
        serde_json::to_vec_pretty(&JsonValue::Object(object))?,
    )
    .with_context(|| format!("failed to write {}", config_path.display()))?;

    Ok(ConfigMutationResponse {
        config: get_capsule_config_for_database(&db_path)?,
        backup_path: backup_path.map(|path| db::path_to_string(&path)),
        operation: operation.to_string(),
        completed_at: Utc::now().to_rfc3339(),
    })
}

fn config_path_for_database(db_path: &Path) -> PathBuf {
    if let Ok(path) = env::var("CAPSULE_CONFIG_PATH") {
        let path = path.trim();
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    db::database_directory_for_database(db_path).join("config.json")
}

fn image_file_dialog(current_path: Option<&str>) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Select images")
        .add_filter("Images", &["jpg", "jpeg", "png", "webp"])
        .add_filter("All files", &["*"]);
    if let Some(directory) = dialog_start_directory(current_path, true) {
        dialog = dialog.set_directory(directory);
    }
    dialog
}

fn dialog_start_directory(value: Option<&str>, file_path: bool) -> Option<PathBuf> {
    let path = value
        .and_then(|value| normalize_string(Some(value)))
        .map(PathBuf::from)?;
    if file_path {
        path.parent().map(Path::to_path_buf).or(Some(path))
    } else {
        Some(path)
    }
}

fn read_config_object(path: &Path) -> Result<Map<String, JsonValue>> {
    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    match serde_json::from_slice::<JsonValue>(&raw)? {
        JsonValue::Object(object) => Ok(object),
        _ => Err(anyhow!("config.json must contain a JSON object")),
    }
}

fn create_config_backup(config_path: &Path) -> Result<Option<PathBuf>> {
    if !config_path.exists() {
        return Ok(None);
    }

    let directory = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let now = Utc::now();
    for offset in 0..3600 {
        let candidate = directory.join(format!(
            "{CONFIG_BACKUP_PREFIX}{}.json",
            (now + chrono::Duration::seconds(offset)).format("%Y%m%d_%H%M%S")
        ));
        if !candidate.exists() {
            fs::copy(config_path, &candidate).with_context(|| {
                format!(
                    "failed to back up config {} to {}",
                    config_path.display(),
                    candidate.display()
                )
            })?;
            return Ok(Some(candidate));
        }
    }

    Err(anyhow!("unable to choose a config backup filename"))
}

fn config_value_to_string(value: &JsonValue) -> String {
    match value {
        JsonValue::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn apply_location_config(
    object: &mut Map<String, JsonValue>,
    input: LocationConfigUpdateRequest,
) -> Result<()> {
    let default_location_name = normalize_string(input.default_location_name.as_deref());
    if input.use_default_location && default_location_name.is_none() {
        return Err(anyhow!(
            "Default location is required when fixed location is enabled."
        ));
    }

    object.insert(
        "location.auto_capture".to_string(),
        JsonValue::Bool(input.auto_capture),
    );
    object.insert(
        "location.use_default_location".to_string(),
        JsonValue::Bool(input.use_default_location),
    );

    if input.use_default_location {
        object.insert(
            "location.default_location_name".to_string(),
            JsonValue::String(default_location_name.unwrap_or_default()),
        );
    } else {
        object.remove("location.default_location_name");
    }

    Ok(())
}

fn normalize_config_key(value: &str) -> Result<String> {
    normalize_string(Some(value)).ok_or_else(|| anyhow!("Config key is required."))
}

fn rename_tag_inner(db_path: &Path, input: TagRenameRequest) -> Result<()> {
    let from = normalize_catalog_name(&input.from)?;
    let to = normalize_catalog_name(&input.to)?;
    if from == to {
        return Ok(());
    }

    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_tag_tables(&tx)?;
    let source_id = tag_id(&tx, &from)?.ok_or_else(|| anyhow!("Tag '{from}' was not found."))?;
    if tag_id(&tx, &to)?.is_some() {
        return Err(anyhow!(
            "Tag '{to}' already exists. Use merge to combine existing tags."
        ));
    }
    tx.execute(
        "UPDATE tags SET name = ?1 WHERE id = ?2",
        params![to, source_id],
    )?;
    tx.commit()?;
    Ok(())
}

fn merge_tag_inner(db_path: &Path, input: TagMergeRequest) -> Result<()> {
    let source = normalize_catalog_name(&input.source)?;
    let target = normalize_catalog_name(&input.target)?;
    if source == target {
        return Ok(());
    }

    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_tag_tables(&tx)?;
    let source_id =
        tag_id(&tx, &source)?.ok_or_else(|| anyhow!("Tag '{source}' was not found."))?;
    let target_id = match tag_id(&tx, &target)? {
        Some(id) => id,
        None => {
            tx.execute("INSERT INTO tags (name) VALUES (?1)", [target.as_str()])?;
            tx.last_insert_rowid()
        }
    };
    tx.execute(
        "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id)
         SELECT entry_id, ?1 FROM entry_tags WHERE tag_id = ?2",
        params![target_id, source_id],
    )?;
    tx.execute("DELETE FROM entry_tags WHERE tag_id = ?1", [source_id])?;
    tx.execute("DELETE FROM tags WHERE id = ?1", [source_id])?;
    tx.commit()?;
    Ok(())
}

fn delete_tag_inner(db_path: &Path, input: TagDeleteRequest) -> Result<()> {
    let name = normalize_catalog_name(&input.name)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_tag_tables(&tx)?;
    let tag_id = tag_id(&tx, &name)?.ok_or_else(|| anyhow!("Tag '{name}' was not found."))?;
    tx.execute("DELETE FROM entry_tags WHERE tag_id = ?1", [tag_id])?;
    tx.execute("DELETE FROM tags WHERE id = ?1", [tag_id])?;
    tx.commit()?;
    Ok(())
}

fn rename_mood_inner(db_path: &Path, input: MoodRenameRequest) -> Result<()> {
    let from = normalize_catalog_name(&input.from)?;
    let to = normalize_catalog_name(&input.to)?;
    if from == to {
        return Ok(());
    }
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_entries_table(&tx)?;
    let changed = tx.execute(
        "UPDATE entries SET mood = ?1, updated_at = ?2 WHERE lower(mood) = lower(?3)",
        params![to, current_timestamp_seconds(), from],
    )?;
    if changed == 0 {
        return Err(anyhow!("Mood '{from}' was not found."));
    }
    tx.commit()?;
    Ok(())
}

fn delete_mood_inner(db_path: &Path, input: MoodDeleteRequest) -> Result<()> {
    let name = normalize_catalog_name(&input.name)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_entries_table(&tx)?;
    let changed = tx.execute(
        "UPDATE entries SET mood = NULL, updated_at = ?1 WHERE lower(mood) = lower(?2)",
        params![current_timestamp_seconds(), name],
    )?;
    if changed == 0 {
        return Err(anyhow!("Mood '{name}' was not found."));
    }
    tx.commit()?;
    Ok(())
}

fn create_template_inner(db_path: &Path, input: LibraryTemplateInput) -> Result<LibraryTemplate> {
    let slug = slugify(&input.slug)?;
    let name = normalize_required(&input.name, "Template name")?;
    let description = normalize_string(input.description.as_deref()).unwrap_or_default();
    let intro_text = input.intro_text.unwrap_or_default();
    let sections = validate_string_list(input.sections.unwrap_or_default(), "Template sections")?;
    let now = current_timestamp_minutes();

    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_library_schema(&tx)?;
    if template_exists(&tx, &slug)? {
        return Err(anyhow!("Template slug '{slug}' already exists."));
    }
    tx.execute(
        "INSERT INTO library_templates
            (slug, name, description, intro_text, sections_json, is_builtin, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8)",
        params![
            slug,
            name,
            description,
            intro_text,
            serde_json::to_string(&sections)?,
            bool_to_int(input.is_active.unwrap_or(true)),
            now,
            now,
        ],
    )?;
    tx.execute(
        "DELETE FROM sync_template_tombstones WHERE slug = ?1",
        [slug.as_str()],
    )?;
    let template = get_template_by_slug(&tx, &slug)?.context("Could not create template.")?;
    tx.commit()?;
    Ok(template)
}

fn update_template_inner(
    db_path: &Path,
    slug: &str,
    input: LibraryTemplateUpdate,
) -> Result<LibraryTemplate> {
    let slug = slugify(slug)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_library_schema(&tx)?;
    let current =
        get_template_by_slug(&tx, &slug)?.ok_or_else(|| anyhow!("Template '{slug}' not found."))?;

    if current.is_builtin
        && (input.name.is_some()
            || input.description.is_some()
            || input.intro_text.is_some()
            || input.sections.is_some())
    {
        return Err(anyhow!(
            "Built-in templates are immutable. You can only enable or disable them."
        ));
    }

    let name = match input.name {
        Some(value) => normalize_required(&value, "Template name")?,
        None => current.name,
    };
    let description = input
        .description
        .map(|value| normalize_string(Some(&value)).unwrap_or_default())
        .unwrap_or(current.description);
    let intro_text = input.intro_text.unwrap_or(current.intro_text);
    let sections = match input.sections {
        Some(value) => validate_string_list(value, "Template sections")?,
        None => current.sections,
    };
    let is_active = input.is_active.unwrap_or(current.is_active);
    tx.execute(
        "UPDATE library_templates
         SET name = ?1,
             description = ?2,
             intro_text = ?3,
             sections_json = ?4,
             is_active = ?5,
             updated_at = ?6
         WHERE slug = ?7",
        params![
            name,
            description,
            intro_text,
            serde_json::to_string(&sections)?,
            bool_to_int(is_active),
            current_timestamp_minutes(),
            slug,
        ],
    )?;
    let template = get_template_by_slug(&tx, &slug)?.context("Could not update template.")?;
    tx.commit()?;
    Ok(template)
}

fn delete_template_inner(db_path: &Path, slug: &str) -> Result<()> {
    let slug = slugify(slug)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_library_schema(&tx)?;
    let current =
        get_template_by_slug(&tx, &slug)?.ok_or_else(|| anyhow!("Template '{slug}' not found."))?;
    if current.is_builtin {
        return Err(anyhow!("Built-in templates cannot be deleted."));
    }
    tx.execute(
        "DELETE FROM library_templates WHERE slug = ?1",
        [slug.as_str()],
    )?;
    tx.execute(
        "INSERT OR REPLACE INTO sync_template_tombstones (slug, deleted_at) VALUES (?1, ?2)",
        params![slug, current_timestamp_minutes()],
    )?;
    tx.commit()?;
    Ok(())
}

fn create_prompt_inner(db_path: &Path, input: LibraryPromptInput) -> Result<LibraryPrompt> {
    let slug = slugify(&input.slug)?;
    let prompt_text = normalize_required(&input.prompt_text, "Prompt text")?;
    let category =
        normalize_string(input.category.as_deref()).unwrap_or_else(|| "general".to_string());
    let tags = validate_optional_list(input.tags.unwrap_or_default());
    let now = current_timestamp_minutes();

    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_library_schema(&tx)?;
    if prompt_exists(&tx, &slug)? {
        return Err(anyhow!("Prompt slug '{slug}' already exists."));
    }
    tx.execute(
        "INSERT INTO library_prompts
            (slug, prompt_text, category, tags_json, is_builtin, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7)",
        params![
            slug,
            prompt_text,
            category,
            serde_json::to_string(&tags)?,
            bool_to_int(input.is_active.unwrap_or(true)),
            now,
            now,
        ],
    )?;
    tx.execute(
        "DELETE FROM sync_prompt_tombstones WHERE slug = ?1",
        [slug.as_str()],
    )?;
    let prompt = get_prompt_by_slug(&tx, &slug)?.context("Could not create prompt.")?;
    tx.commit()?;
    Ok(prompt)
}

fn update_prompt_inner(
    db_path: &Path,
    slug: &str,
    input: LibraryPromptUpdate,
) -> Result<LibraryPrompt> {
    let slug = slugify(slug)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_library_schema(&tx)?;
    let current =
        get_prompt_by_slug(&tx, &slug)?.ok_or_else(|| anyhow!("Prompt '{slug}' not found."))?;

    if current.is_builtin
        && (input.prompt_text.is_some() || input.category.is_some() || input.tags.is_some())
    {
        return Err(anyhow!(
            "Built-in prompts are immutable. You can only enable or disable them."
        ));
    }

    let prompt_text = match input.prompt_text {
        Some(value) => normalize_required(&value, "Prompt text")?,
        None => current.prompt_text,
    };
    let category = input
        .category
        .map(|value| normalize_string(Some(&value)).unwrap_or_else(|| "general".to_string()))
        .unwrap_or(current.category);
    let tags = input
        .tags
        .map(validate_optional_list)
        .unwrap_or(current.tags);
    let is_active = input.is_active.unwrap_or(current.is_active);
    tx.execute(
        "UPDATE library_prompts
         SET prompt_text = ?1,
             category = ?2,
             tags_json = ?3,
             is_active = ?4,
             updated_at = ?5
         WHERE slug = ?6",
        params![
            prompt_text,
            category,
            serde_json::to_string(&tags)?,
            bool_to_int(is_active),
            current_timestamp_minutes(),
            slug,
        ],
    )?;
    let prompt = get_prompt_by_slug(&tx, &slug)?.context("Could not update prompt.")?;
    tx.commit()?;
    Ok(prompt)
}

fn delete_prompt_inner(db_path: &Path, slug: &str) -> Result<()> {
    let slug = slugify(slug)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_library_schema(&tx)?;
    let current =
        get_prompt_by_slug(&tx, &slug)?.ok_or_else(|| anyhow!("Prompt '{slug}' not found."))?;
    if current.is_builtin {
        return Err(anyhow!("Built-in prompts cannot be deleted."));
    }
    tx.execute(
        "DELETE FROM library_prompts WHERE slug = ?1",
        [slug.as_str()],
    )?;
    tx.execute(
        "INSERT OR REPLACE INTO sync_prompt_tombstones (slug, deleted_at) VALUES (?1, ?2)",
        params![slug, current_timestamp_minutes()],
    )?;
    tx.commit()?;
    Ok(())
}

fn load_entries_for_export(
    db_path: &Path,
    uuids: Option<Vec<String>>,
    search_request: Option<crate::models::SearchRequest>,
    filters: Option<EntryFilters>,
) -> Result<Vec<Entry>> {
    if let Some(uuids) = uuids {
        let uuids = uuids
            .into_iter()
            .filter_map(|uuid| normalize_string(Some(&uuid)))
            .collect::<Vec<_>>();
        return entries::list_entries_by_uuids_for_database(db_path, &uuids);
    }

    if let Some(search_request) = search_request {
        return Ok(search::search_entries_for_database(db_path, search_request)?.entries);
    }

    let mut filters = filters.unwrap_or_default();
    filters.limit = Some(filters.limit.unwrap_or(200).clamp(1, 200));
    Ok(entries::list_entries_for_database(db_path, filters)?.entries)
}

fn entries_to_markdown(entries: &[Entry]) -> String {
    let mut output = String::new();
    output.push_str("# Capsule Export\n\n");
    for entry in entries {
        output.push_str("## ");
        output.push_str(entry.title.as_deref().unwrap_or("Untitled entry"));
        output.push_str("\n\n");
        output.push_str(&format!("- UUID: {}\n", entry.uuid));
        output.push_str(&format!("- Created: {}\n", entry.created_at));
        if let Some(mood) = entry.mood.as_deref() {
            output.push_str(&format!("- Mood: {mood}\n"));
        }
        if !entry.tags.is_empty() {
            let tags = entry
                .tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!("- Tags: {tags}\n"));
        }
        output.push('\n');
        output.push_str(&entry.text);
        output.push_str("\n\n---\n\n");
    }
    output
}

fn next_available_export_path(directory: &Path, stem: &str, extension: &str) -> PathBuf {
    let first = directory.join(format!("{stem}.{extension}"));
    if !first.exists() {
        return first;
    }

    for index in 2..10_000 {
        let candidate = directory.join(format!("{stem}-{index}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    first
}

fn normalize_file_stem(value: &str) -> Option<String> {
    let stem = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else if ch.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect::<String>();
    normalize_string(Some(&stem))
}

fn ensure_tag_tables(connection: &Connection) -> Result<()> {
    let tables = detected_tables(connection)?;
    if !tables.contains("tags") || !tables.contains("entry_tags") {
        return Err(anyhow!(
            "The active database must contain tags and entry_tags tables."
        ));
    }
    Ok(())
}

fn ensure_entries_table(connection: &Connection) -> Result<()> {
    if detected_tables(connection)?.contains("entries") {
        Ok(())
    } else {
        Err(anyhow!(
            "The active database does not contain an entries table."
        ))
    }
}

fn ensure_library_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS library_templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            description TEXT,
            intro_text TEXT,
            sections_json TEXT NOT NULL DEFAULT '[]',
            is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
            is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS library_prompts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL UNIQUE,
            prompt_text TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'general',
            tags_json TEXT NOT NULL DEFAULT '[]',
            is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
            is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_template_tombstones (
            slug TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_prompt_tombstones (
            slug TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_library_templates_active ON library_templates(is_active, slug);
        CREATE INDEX IF NOT EXISTS idx_library_prompts_active ON library_prompts(is_active, slug);
        ",
    )?;
    Ok(())
}

fn list_templates(connection: &Connection) -> Result<Vec<LibraryTemplate>> {
    let mut statement = connection.prepare(
        "SELECT id, slug, name, description, intro_text, sections_json,
                is_builtin, is_active, created_at, updated_at
         FROM library_templates
         ORDER BY is_builtin DESC, lower(slug) ASC",
    )?;
    let templates = statement
        .query_map([], template_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(templates)
}

fn list_prompts(connection: &Connection) -> Result<Vec<LibraryPrompt>> {
    let mut statement = connection.prepare(
        "SELECT id, slug, prompt_text, category, tags_json,
                is_builtin, is_active, created_at, updated_at
         FROM library_prompts
         ORDER BY is_builtin DESC, lower(slug) ASC",
    )?;
    let prompts = statement
        .query_map([], prompt_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(prompts)
}

fn get_template_by_slug(connection: &Connection, slug: &str) -> Result<Option<LibraryTemplate>> {
    connection
        .query_row(
            "SELECT id, slug, name, description, intro_text, sections_json,
                    is_builtin, is_active, created_at, updated_at
             FROM library_templates
             WHERE slug = ?1",
            [slug],
            template_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn get_prompt_by_slug(connection: &Connection, slug: &str) -> Result<Option<LibraryPrompt>> {
    connection
        .query_row(
            "SELECT id, slug, prompt_text, category, tags_json,
                    is_builtin, is_active, created_at, updated_at
             FROM library_prompts
             WHERE slug = ?1",
            [slug],
            prompt_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn template_exists(connection: &Connection, slug: &str) -> Result<bool> {
    Ok(get_template_by_slug(connection, slug)?.is_some())
}

fn prompt_exists(connection: &Connection, slug: &str) -> Result<bool> {
    Ok(get_prompt_by_slug(connection, slug)?.is_some())
}

fn template_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryTemplate> {
    let sections_json: String = row.get(5)?;
    Ok(LibraryTemplate {
        id: row.get(0)?,
        slug: row.get(1)?,
        name: row.get(2)?,
        description: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        intro_text: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        sections: parse_string_list(&sections_json),
        is_builtin: row.get::<_, i64>(6)? != 0,
        is_active: row.get::<_, i64>(7)? != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn prompt_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryPrompt> {
    let tags_json: String = row.get(4)?;
    Ok(LibraryPrompt {
        id: row.get(0)?,
        slug: row.get(1)?,
        prompt_text: row.get(2)?,
        category: row
            .get::<_, Option<String>>(3)?
            .unwrap_or_else(|| "general".to_string()),
        tags: parse_string_list(&tags_json),
        is_builtin: row.get::<_, i64>(5)? != 0,
        is_active: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn parse_string_list(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<JsonValue>>(raw)
        .ok()
        .map(|values| {
            values
                .into_iter()
                .filter_map(|value| match value {
                    JsonValue::String(text) => Some(text),
                    other => Some(other.to_string()),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn validate_string_list(values: Vec<String>, label: &str) -> Result<Vec<String>> {
    let normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .collect::<Vec<_>>();
    if normalized.iter().any(|value| value.is_empty()) {
        return Err(anyhow!("{label} cannot contain empty values."));
    }
    Ok(normalized)
}

fn validate_optional_list(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for value in values {
        if let Some(value) = normalize_string(Some(&value)) {
            if seen.insert(value.to_lowercase()) {
                output.push(value);
            }
        }
    }
    output
}

fn tag_id(connection: &Connection, name: &str) -> Result<Option<i64>> {
    connection
        .query_row(
            "SELECT id FROM tags WHERE lower(name) = lower(?1) LIMIT 1",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn detected_tables(connection: &Connection) -> Result<HashSet<String>> {
    Ok(db::inspect_schema(connection)?
        .detected_tables
        .into_iter()
        .collect())
}

fn slugify(value: &str) -> Result<String> {
    let mut previous_dash = false;
    let mut slug = String::new();
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            slug.push(ch);
            previous_dash = false;
        } else if ch.is_whitespace() || ch == '-' {
            if !previous_dash && !slug.is_empty() {
                slug.push('-');
                previous_dash = true;
            }
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        return Err(anyhow!("Slug cannot be empty."));
    }
    if slug.len() > 64 {
        return Err(anyhow!("Slug must be 64 characters or fewer."));
    }
    if !slug
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphanumeric())
        .unwrap_or(false)
    {
        return Err(anyhow!("Slug must start with a letter or number."));
    }
    Ok(slug)
}

fn normalize_required(value: &str, label: &str) -> Result<String> {
    normalize_string(Some(value)).ok_or_else(|| anyhow!("{label} cannot be empty."))
}

fn normalize_catalog_name(value: &str) -> Result<String> {
    normalize_required(value, "Name").map(|value| value.to_lowercase())
}

fn normalize_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn labelize(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn current_timestamp_minutes() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

fn current_timestamp_seconds() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        LibraryPromptInput, LibraryPromptUpdate, LibraryTemplateInput, LibraryTemplateUpdate,
        MoodDeleteRequest, MoodRenameRequest, TagMergeRequest, TagRenameRequest,
    };
    use rusqlite::Connection;

    #[test]
    fn location_config_update_writes_capsule_default_location_keys() {
        let mut object = Map::new();

        apply_location_config(
            &mut object,
            LocationConfigUpdateRequest {
                auto_capture: true,
                use_default_location: true,
                default_location_name: Some(" Tromso, Norway ".to_string()),
            },
        )
        .expect("fixed location config");

        assert_eq!(
            object.get("location.auto_capture"),
            Some(&JsonValue::Bool(true))
        );
        assert_eq!(
            object.get("location.use_default_location"),
            Some(&JsonValue::Bool(true))
        );
        assert_eq!(
            object
                .get("location.default_location_name")
                .and_then(JsonValue::as_str),
            Some("Tromso, Norway")
        );

        apply_location_config(
            &mut object,
            LocationConfigUpdateRequest {
                auto_capture: true,
                use_default_location: false,
                default_location_name: Some("Tromso, Norway".to_string()),
            },
        )
        .expect("ip lookup config");

        assert_eq!(
            object.get("location.use_default_location"),
            Some(&JsonValue::Bool(false))
        );
        assert!(object.get("location.default_location_name").is_none());

        let error = apply_location_config(
            &mut object,
            LocationConfigUpdateRequest {
                auto_capture: true,
                use_default_location: true,
                default_location_name: Some(" ".to_string()),
            },
        )
        .expect_err("empty fixed location");
        assert!(error.to_string().contains("Default location is required"));
    }

    #[test]
    fn tag_merge_combines_links_and_keeps_backup() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_tools_fixture(temp_dir.path());

        let response = backup::with_database_backup_for_database(&db_path, "tag.merge", |path| {
            merge_tag_inner(
                path,
                TagMergeRequest {
                    source: "alpha".to_string(),
                    target: "beta".to_string(),
                },
            )?;
            list_tags_for_database(path).map(|response| response.tags)
        })
        .expect("merge tag");

        assert!(PathBuf::from(response.audit.backup_path).exists());
        let beta = response
            .value
            .iter()
            .find(|tag| tag.name == "beta")
            .expect("beta tag");
        assert_eq!(beta.entry_count, 2);
        assert!(!response.value.iter().any(|tag| tag.name == "alpha"));
    }

    #[test]
    fn mood_rename_and_delete_update_entry_values() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_tools_fixture(temp_dir.path());

        rename_mood_inner(
            &db_path,
            MoodRenameRequest {
                from: "calm".to_string(),
                to: "steady".to_string(),
            },
        )
        .expect("rename mood");
        delete_mood_inner(
            &db_path,
            MoodDeleteRequest {
                name: "focused".to_string(),
            },
        )
        .expect("delete mood");

        let moods = list_moods_for_database(&db_path).expect("moods").moods;
        assert!(moods.iter().any(|mood| mood.name == "steady"));
        assert!(!moods.iter().any(|mood| mood.name == "focused"));
    }

    #[test]
    fn library_crud_preserves_schema_and_tombstones() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_tools_fixture(temp_dir.path());

        let template = create_template_inner(
            &db_path,
            LibraryTemplateInput {
                slug: "Morning Pages".to_string(),
                name: "Morning Pages".to_string(),
                description: Some("Start the day".to_string()),
                intro_text: Some("Begin".to_string()),
                sections: Some(vec!["## Today".to_string()]),
                is_active: Some(true),
            },
        )
        .expect("create template");
        assert_eq!(template.slug, "morning-pages");

        let updated = update_template_inner(
            &db_path,
            &template.slug,
            LibraryTemplateUpdate {
                is_active: Some(false),
                ..LibraryTemplateUpdate::default()
            },
        )
        .expect("update template");
        assert!(!updated.is_active);

        let prompt = create_prompt_inner(
            &db_path,
            LibraryPromptInput {
                slug: "Daily Question".to_string(),
                prompt_text: "What mattered today?".to_string(),
                category: Some("reflection".to_string()),
                tags: Some(vec!["daily".to_string(), "daily".to_string()]),
                is_active: Some(true),
            },
        )
        .expect("create prompt");
        assert_eq!(prompt.tags, vec!["daily"]);

        let prompt = update_prompt_inner(
            &db_path,
            &prompt.slug,
            LibraryPromptUpdate {
                is_active: Some(false),
                ..LibraryPromptUpdate::default()
            },
        )
        .expect("update prompt");
        assert!(!prompt.is_active);

        delete_template_inner(&db_path, &template.slug).expect("delete template");
        delete_prompt_inner(&db_path, &prompt.slug).expect("delete prompt");

        let connection = Connection::open(&db_path).expect("open db");
        let template_tombstone = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_template_tombstones WHERE slug = ?1",
                [template.slug],
                |row| row.get::<_, i64>(0),
            )
            .expect("template tombstone");
        let prompt_tombstone = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_prompt_tombstones WHERE slug = ?1",
                [prompt.slug],
                |row| row.get::<_, i64>(0),
            )
            .expect("prompt tombstone");
        assert_eq!(template_tombstone, 1);
        assert_eq!(prompt_tombstone, 1);
    }

    #[test]
    fn export_entries_writes_markdown_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_tools_fixture(temp_dir.path());

        let response = export_entries_for_database(
            &db_path,
            ExportEntriesRequest {
                format: ExportFormat::Markdown,
                uuids: Some(vec!["entry_one".to_string()]),
                search: None,
                filters: None,
                file_name: Some("single entry".to_string()),
            },
        )
        .expect("export");

        let path = PathBuf::from(response.path);
        assert!(path.exists());
        let content = fs::read_to_string(path).expect("read export");
        assert!(content.contains("First entry"));
    }

    #[test]
    fn rename_tag_rejects_existing_target() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_tools_fixture(temp_dir.path());

        let result = rename_tag_inner(
            &db_path,
            TagRenameRequest {
                from: "alpha".to_string(),
                to: "beta".to_string(),
            },
        );

        assert!(result.is_err());
    }

    fn create_tools_fixture(path: &Path) -> PathBuf {
        let db_path = path.join("capsule.db");
        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    uuid TEXT UNIQUE,
                    created_at TEXT NOT NULL,
                    updated_at TEXT,
                    text TEXT NOT NULL,
                    text_plain TEXT NOT NULL DEFAULT '',
                    content_format TEXT NOT NULL DEFAULT 'plain',
                    title TEXT,
                    summary TEXT,
                    mood TEXT,
                    starred INTEGER DEFAULT 0,
                    pinned INTEGER DEFAULT 0,
                    hidden INTEGER DEFAULT 0
                );
                CREATE TABLE tags (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE
                );
                CREATE TABLE entry_tags (
                    entry_id INTEGER NOT NULL,
                    tag_id INTEGER NOT NULL,
                    PRIMARY KEY (entry_id, tag_id)
                );
                INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format, title, mood)
                VALUES
                    ('entry_one', '2026-01-01 09:00', '2026-01-01 09:00', 'First entry', 'First entry', 'markdown', 'First', 'calm'),
                    ('entry_two', '2026-01-02 09:00', '2026-01-02 09:00', 'Second entry', 'Second entry', 'plain', 'Second', 'focused');
                INSERT INTO tags (name) VALUES ('alpha'), ('beta');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1), (2, 2);
                ",
            )
            .expect("fixture");
        drop(connection);
        db_path
    }
}
