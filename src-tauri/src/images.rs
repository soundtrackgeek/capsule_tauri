use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use image::GenericImageView;
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::{
    backup, db, entries,
    models::{
        CoverEntrySummary, CoverWallRequest, CoverWallResponse, EntryCover, ImageAsset,
        ImageAttachRequest, ImageAttachment, ImageEntriesItem, ImageEntriesListResponse,
        ImageEntryListResponse, ImageMutationResponse, ImageUploadResponse, ImageVariant,
    },
};

const DEFAULT_MEDIA_ROOT: &str = r"C:\Users\jtill\OneDrive\_capsule\images";
const MAX_UPLOAD_BYTES: u64 = 10 * 1024 * 1024;
const COVER_ROOT_ENV: &str = "CAPSULE_COVERS_ROOT";
const MEDIA_ROOT_ENV: &str = "CAPSULE_IMAGES_MEDIA_ROOT";
const COVER_THUMB_SIZE: u32 = 840;
const IMAGE_THUMB_SIZE: u32 = 480;
const DEFAULT_LIMIT: i64 = 80;
const MAX_LIMIT: i64 = 240;

#[derive(Debug, Clone)]
struct RawAttachment {
    attachment_id: i64,
    entry_uuid: String,
    media_id: i64,
    position: i64,
    caption: Option<String>,
    alt_text: Option<String>,
    created_at: String,
    hash: String,
    mime_type: String,
    bytes: i64,
    width: i64,
    height: i64,
    storage_backend: String,
    storage_key: String,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone)]
struct CoverFile {
    filename: String,
    cover_type: String,
    entry_uuid: String,
    path: PathBuf,
    bytes: u64,
    modified_at: Option<String>,
}

pub fn list_entry_images(identifier: String) -> Result<ImageEntryListResponse> {
    list_entry_images_for_database(&db::resolve_database_path(), &identifier)
}

pub fn get_image_media_root() -> Result<String> {
    let db_path = db::resolve_database_path();
    let roots = media_roots_for_database(&db_path, None);
    let root =
        first_existing_or_default_root(&roots).unwrap_or_else(|| PathBuf::from(DEFAULT_MEDIA_ROOT));
    Ok(db::path_to_string(&root))
}

pub(crate) fn list_entry_images_for_database(
    db_path: &Path,
    identifier: &str,
) -> Result<ImageEntryListResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let entry_uuid = resolve_entry_uuid(&connection, identifier)?;
    let tables = detected_tables(&connection)?;
    if !tables.contains("plugin_entry_media") || !tables.contains("plugin_media_assets") {
        return Ok(ImageEntryListResponse {
            entry_uuid,
            images: Vec::new(),
            warnings: vec!["Image attachment tables were not found.".to_string()],
        });
    }

    let roots = media_roots_for_database(db_path, None);
    let raws = query_attachments_for_uuids(&connection, &[entry_uuid.clone()])?;
    let images = raws
        .into_iter()
        .map(|raw| attachment_from_raw(raw, &roots))
        .collect::<Vec<_>>();

    Ok(ImageEntryListResponse {
        entry_uuid,
        images,
        warnings: Vec::new(),
    })
}

pub fn list_images_for_entries(uuids: Vec<String>) -> Result<ImageEntriesListResponse> {
    list_images_for_entries_for_database(&db::resolve_database_path(), uuids)
}

pub(crate) fn list_images_for_entries_for_database(
    db_path: &Path,
    uuids: Vec<String>,
) -> Result<ImageEntriesListResponse> {
    let normalized = uuids
        .into_iter()
        .filter_map(|uuid| normalize_string(Some(&uuid)))
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return Ok(ImageEntriesListResponse {
            entries: Vec::new(),
            warnings: Vec::new(),
        });
    }

    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    if !tables.contains("plugin_entry_media") || !tables.contains("plugin_media_assets") {
        return Ok(ImageEntriesListResponse {
            entries: normalized
                .into_iter()
                .map(|entry_uuid| ImageEntriesItem {
                    entry_uuid,
                    images: Vec::new(),
                })
                .collect(),
            warnings: vec!["Image attachment tables were not found.".to_string()],
        });
    }

    let roots = media_roots_for_database(db_path, None);
    let mut grouped: HashMap<String, Vec<ImageAttachment>> = HashMap::new();
    for raw in query_attachments_for_uuids(&connection, &normalized)? {
        grouped
            .entry(raw.entry_uuid.clone())
            .or_default()
            .push(attachment_from_raw(raw, &roots));
    }

    Ok(ImageEntriesListResponse {
        entries: normalized
            .into_iter()
            .map(|entry_uuid| ImageEntriesItem {
                images: grouped.remove(&entry_uuid).unwrap_or_default(),
                entry_uuid,
            })
            .collect(),
        warnings: Vec::new(),
    })
}

pub fn get_image_data_url(attachment_id: i64, variant: ImageVariant) -> Result<String> {
    get_image_data_url_for_database(&db::resolve_database_path(), attachment_id, variant)
}

pub(crate) fn get_image_data_url_for_database(
    db_path: &Path,
    attachment_id: i64,
    variant: ImageVariant,
) -> Result<String> {
    let connection = db::open_read_only_connection(db_path)?;
    let raw = get_raw_attachment(&connection, attachment_id)?;
    if raw.storage_backend != "local_fs" {
        return Err(anyhow!(
            "Only local_fs image assets can be served directly by Capsule Tauri."
        ));
    }

    let roots = media_roots_for_database(db_path, None);
    let (bytes, mime_type) = match variant {
        ImageVariant::Full => (
            read_media_bytes(&roots, &raw.storage_key)?,
            raw.mime_type.clone(),
        ),
        ImageVariant::Thumb => {
            let thumb_key = thumbnail_key(&raw.hash);
            match read_media_bytes(&roots, &thumb_key) {
                Ok(bytes) => (bytes, "image/jpeg".to_string()),
                Err(_) => {
                    let original = read_media_bytes(&roots, &raw.storage_key)?;
                    let thumb = build_thumbnail_bytes(&original, IMAGE_THUMB_SIZE)?;
                    if let Some(root) = first_existing_or_default_root(&roots) {
                        let _ = write_media_bytes(&root, &thumb_key, &thumb);
                    }
                    (thumb, "image/jpeg".to_string())
                }
            }
        }
    };

    Ok(data_url(&mime_type, &bytes))
}

pub fn get_local_image_preview_data_url(file_path: String) -> Result<String> {
    let source_path = PathBuf::from(file_path);
    let metadata = fs::metadata(&source_path)
        .with_context(|| format!("image file does not exist: {}", source_path.display()))?;
    if !metadata.is_file() {
        return Err(anyhow!(
            "image path is not a file: {}",
            source_path.display()
        ));
    }
    if metadata.len() == 0 {
        return Err(anyhow!("image file is empty: {}", source_path.display()));
    }
    if metadata.len() > MAX_UPLOAD_BYTES {
        return Err(anyhow!("image exceeds the 10 MB upload limit"));
    }

    let bytes = fs::read(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    detect_image_mime(&source_path, &bytes)?;
    let preview = build_thumbnail_bytes(&bytes, IMAGE_THUMB_SIZE)?;
    Ok(data_url("image/jpeg", &preview))
}

pub fn upload_image(file_path: String) -> Result<ImageUploadResponse> {
    upload_image_with_root(file_path, None)
}

fn upload_image_with_root(
    file_path: String,
    media_root_override: Option<PathBuf>,
) -> Result<ImageUploadResponse> {
    let guarded = backup::with_database_backup("image.upload", move |db_path| {
        upload_image_inner(db_path, &file_path, media_root_override)
    })?;
    Ok(ImageUploadResponse {
        asset: guarded.value,
        audit: guarded.audit,
    })
}

pub fn attach_image(input: ImageAttachRequest) -> Result<ImageMutationResponse> {
    attach_image_with_root(input, None)
}

fn attach_image_with_root(
    input: ImageAttachRequest,
    media_root_override: Option<PathBuf>,
) -> Result<ImageMutationResponse> {
    let guarded = backup::with_database_backup("image.attach", move |db_path| {
        attach_image_inner(db_path, input, media_root_override)
    })?;
    Ok(ImageMutationResponse {
        entry_uuid: guarded.value.0,
        images: guarded.value.1,
        audit: guarded.audit,
    })
}

pub fn remove_image(
    attachment_id: i64,
    identifier: Option<String>,
) -> Result<ImageMutationResponse> {
    let guarded = backup::with_database_backup("image.remove", move |db_path| {
        remove_image_inner(db_path, attachment_id, identifier)
    })?;
    Ok(ImageMutationResponse {
        entry_uuid: guarded.value.0,
        images: guarded.value.1,
        audit: guarded.audit,
    })
}

pub fn list_cover_wall(input: Option<CoverWallRequest>) -> Result<CoverWallResponse> {
    list_cover_wall_for_database(&db::resolve_database_path(), input.unwrap_or_default())
}

pub(crate) fn list_cover_wall_for_database(
    db_path: &Path,
    input: CoverWallRequest,
) -> Result<CoverWallResponse> {
    let covers_root = resolve_covers_root();
    let all_covers = iter_cover_files(&covers_root)?;
    let entry_uuids = all_covers
        .iter()
        .map(|cover| cover.entry_uuid.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let entries = entries::list_entries_by_uuids_for_database(db_path, &entry_uuids)?;
    let entries_by_uuid = entries
        .into_iter()
        .filter(|entry| !entry.hidden)
        .map(|entry| {
            let summary = CoverEntrySummary {
                id: entry.id,
                uuid: entry.uuid.clone(),
                created_at: entry.created_at,
                title: entry.title,
                mood: entry.mood,
                tags: entry.tags.into_iter().map(|tag| tag.name).collect(),
            };
            (summary.uuid.clone(), summary)
        })
        .collect::<HashMap<_, _>>();

    let available_types = {
        let mut values = all_covers
            .iter()
            .filter(|cover| entries_by_uuid.contains_key(&cover.entry_uuid))
            .map(|cover| cover.cover_type.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        values.sort();
        values
    };
    let orphaned_cover_count = all_covers
        .iter()
        .filter(|cover| !entries_by_uuid.contains_key(&cover.entry_uuid))
        .count() as i64;

    let cover_type =
        normalize_string(input.cover_type.as_deref()).map(|value| value.to_lowercase());
    let tags = normalized_set(input.tags.as_deref());
    let moods = normalized_set(input.moods.as_deref());
    let mut covers = all_covers
        .into_iter()
        .filter_map(|cover| {
            let entry = entries_by_uuid.get(&cover.entry_uuid)?;
            if !cover_matches(
                &cover,
                entry,
                cover_type.as_deref(),
                input.since.as_deref(),
                input.until.as_deref(),
                &tags,
                &moods,
            ) {
                return None;
            }
            Some(EntryCover {
                filename: cover.filename,
                cover_type: cover.cover_type,
                entry_uuid: cover.entry_uuid,
                bytes: cover.bytes,
                modified_at: cover.modified_at,
                entry: entry.clone(),
            })
        })
        .collect::<Vec<_>>();
    covers.sort_by(|left, right| right.entry.created_at.cmp(&left.entry.created_at));

    let total = covers.len() as i64;
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = input.offset.unwrap_or(0).max(0);
    let paged = covers
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(CoverWallResponse {
        covers: paged,
        total,
        limit,
        offset,
        available_types,
        orphaned_cover_count,
        covers_root: db::path_to_string(&covers_root),
    })
}

pub fn get_cover_data_url(filename: String, variant: ImageVariant) -> Result<String> {
    let covers_root = resolve_covers_root();
    let cover = resolve_cover_file(&covers_root, &filename)?;
    let bytes = match variant {
        ImageVariant::Full => fs::read(&cover.path)
            .with_context(|| format!("failed to read {}", cover.path.display()))?,
        ImageVariant::Thumb => {
            let cache_path = cover_thumbnail_cache_path(&cover)?;
            if !cache_path.exists() {
                let original = fs::read(&cover.path)
                    .with_context(|| format!("failed to read {}", cover.path.display()))?;
                let thumbnail = build_thumbnail_bytes(&original, COVER_THUMB_SIZE)?;
                if let Some(parent) = cache_path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("failed to create {}", parent.display()))?;
                }
                fs::write(&cache_path, &thumbnail)
                    .with_context(|| format!("failed to write {}", cache_path.display()))?;
                thumbnail
            } else {
                fs::read(&cache_path)
                    .with_context(|| format!("failed to read {}", cache_path.display()))?
            }
        }
    };
    let mime_type = if variant == ImageVariant::Thumb {
        "image/jpeg".to_string()
    } else {
        mime_from_path(&cover.path).unwrap_or_else(|| "image/jpeg".to_string())
    };
    Ok(data_url(&mime_type, &bytes))
}

fn upload_image_inner(
    db_path: &Path,
    file_path: &str,
    media_root_override: Option<PathBuf>,
) -> Result<ImageAsset> {
    let source_path = PathBuf::from(file_path);
    let metadata = fs::metadata(&source_path)
        .with_context(|| format!("image file does not exist: {}", source_path.display()))?;
    if !metadata.is_file() {
        return Err(anyhow!(
            "image path is not a file: {}",
            source_path.display()
        ));
    }
    if metadata.len() == 0 {
        return Err(anyhow!("image file is empty: {}", source_path.display()));
    }
    if metadata.len() > MAX_UPLOAD_BYTES {
        return Err(anyhow!("image exceeds the 10 MB upload limit"));
    }

    let bytes = fs::read(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let mime_type = detect_image_mime(&source_path, &bytes)?;
    let extension = extension_for_mime(&mime_type)?;
    let image = image::load_from_memory(&bytes).context("failed to decode image dimensions")?;
    let (width, height) = image.dimensions();
    let hash = sha256_hex(&bytes);
    let storage_key = format!("{}/{}.{}", &hash[..2], hash, extension);
    let roots = media_roots_for_database(db_path, media_root_override);
    let media_root = roots
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("could not resolve a media root for image upload"))?;
    write_media_bytes(&media_root, &storage_key, &bytes)?;
    let thumbnail = build_thumbnail_bytes(&bytes, IMAGE_THUMB_SIZE)?;
    write_media_bytes(&media_root, &thumbnail_key(&hash), &thumbnail)?;

    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_image_schema(&tx)?;
    tx.execute(
        "INSERT INTO plugin_media_assets
            (hash, mime_type, bytes, width, height, storage_backend, storage_key, created_at, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'local_fs', ?6, ?7, NULL)
         ON CONFLICT(hash) DO UPDATE SET deleted_at = NULL",
        params![
            hash,
            mime_type,
            metadata.len() as i64,
            width as i64,
            height as i64,
            storage_key,
            current_timestamp_minutes(),
        ],
    )?;
    let asset = get_asset_by_hash(&tx, &hash)?.context("failed to resolve uploaded image asset")?;
    tx.commit()?;
    Ok(asset)
}

fn attach_image_inner(
    db_path: &Path,
    input: ImageAttachRequest,
    media_root_override: Option<PathBuf>,
) -> Result<(String, Vec<ImageAttachment>)> {
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_image_schema(&tx)?;
    let entry_uuid = resolve_entry_uuid(&tx, &input.identifier)?;
    let media_exists = tx
        .query_row(
            "SELECT 1 FROM plugin_media_assets WHERE id = ?1 AND deleted_at IS NULL",
            [input.media_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !media_exists {
        return Err(anyhow!("Media asset #{} does not exist.", input.media_id));
    }

    let position = match input.position {
        Some(value) => value.max(0),
        None => tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM plugin_entry_media WHERE entry_uuid = ?1",
            [entry_uuid.as_str()],
            |row| row.get::<_, i64>(0),
        )?,
    };
    tx.execute(
        "INSERT INTO plugin_entry_media (entry_uuid, media_id, position, caption, alt_text, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entry_uuid,
            input.media_id,
            position,
            normalize_string(input.caption.as_deref()),
            normalize_string(input.alt_text.as_deref()),
            current_timestamp_minutes(),
        ],
    )?;
    tx.commit()?;

    let roots = media_roots_for_database(db_path, media_root_override);
    let images = list_attachments_for_entry_with_roots(db_path, &entry_uuid, &roots)?;
    Ok((entry_uuid, images))
}

fn remove_image_inner(
    db_path: &Path,
    attachment_id: i64,
    identifier: Option<String>,
) -> Result<(String, Vec<ImageAttachment>)> {
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_image_schema(&tx)?;
    let row = get_raw_attachment(&tx, attachment_id)?;
    if let Some(identifier) = identifier.as_deref() {
        let expected_uuid = resolve_entry_uuid(&tx, identifier)?;
        if expected_uuid != row.entry_uuid {
            return Err(anyhow!(
                "Attachment #{} does not belong to entry {}.",
                attachment_id,
                expected_uuid
            ));
        }
    }

    tx.execute(
        "CREATE TABLE IF NOT EXISTS sync_image_tombstones (
            entry_uuid TEXT NOT NULL,
            asset_hash TEXT NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            caption TEXT,
            alt_text TEXT,
            deleted_at TEXT NOT NULL,
            PRIMARY KEY (entry_uuid, asset_hash, position, caption, alt_text)
        )",
        [],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO sync_image_tombstones
            (entry_uuid, asset_hash, position, caption, alt_text, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            row.entry_uuid,
            row.hash,
            row.position,
            row.caption.clone().unwrap_or_default(),
            row.alt_text.clone().unwrap_or_default(),
            current_timestamp_minutes(),
        ],
    )?;
    tx.execute(
        "DELETE FROM plugin_entry_media WHERE id = ?1",
        [attachment_id],
    )?;
    let remaining = tx.query_row(
        "SELECT COUNT(*) FROM plugin_entry_media WHERE media_id = ?1",
        [row.media_id],
        |row| row.get::<_, i64>(0),
    )?;
    if remaining == 0 {
        tx.execute(
            "UPDATE plugin_media_assets SET deleted_at = ?1 WHERE id = ?2",
            params![current_timestamp_minutes(), row.media_id],
        )?;
    }
    tx.commit()?;

    let roots = media_roots_for_database(db_path, None);
    let images = list_attachments_for_entry_with_roots(db_path, &row.entry_uuid, &roots)?;
    Ok((row.entry_uuid, images))
}

fn list_attachments_for_entry_with_roots(
    db_path: &Path,
    entry_uuid: &str,
    roots: &[PathBuf],
) -> Result<Vec<ImageAttachment>> {
    let connection = db::open_read_only_connection(db_path)?;
    let raws = query_attachments_for_uuids(&connection, &[entry_uuid.to_string()])?;
    Ok(raws
        .into_iter()
        .map(|raw| attachment_from_raw(raw, roots))
        .collect())
}

fn query_attachments_for_uuids(
    connection: &Connection,
    entry_uuids: &[String],
) -> Result<Vec<RawAttachment>> {
    if entry_uuids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = placeholders(entry_uuids.len());
    let sql = format!(
        "SELECT
            em.id AS attachment_id,
            em.entry_uuid,
            em.media_id,
            em.position,
            em.caption,
            em.alt_text,
            em.created_at,
            ma.hash,
            ma.mime_type,
            ma.bytes,
            ma.width,
            ma.height,
            ma.storage_backend,
            ma.storage_key,
            ma.deleted_at
         FROM plugin_entry_media em
         JOIN plugin_media_assets ma ON ma.id = em.media_id
         WHERE em.entry_uuid IN ({placeholders})
           AND ma.deleted_at IS NULL
         ORDER BY em.entry_uuid, em.position ASC, em.id ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params_from_iter(entry_uuids.iter().cloned().map(Value::Text)),
        raw_attachment_from_row,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list image attachments")
}

fn get_raw_attachment(connection: &Connection, attachment_id: i64) -> Result<RawAttachment> {
    connection
        .query_row(
            "SELECT
                em.id AS attachment_id,
                em.entry_uuid,
                em.media_id,
                em.position,
                em.caption,
                em.alt_text,
                em.created_at,
                ma.hash,
                ma.mime_type,
                ma.bytes,
                ma.width,
                ma.height,
                ma.storage_backend,
                ma.storage_key,
                ma.deleted_at
             FROM plugin_entry_media em
             JOIN plugin_media_assets ma ON ma.id = em.media_id
             WHERE em.id = ?1",
            [attachment_id],
            raw_attachment_from_row,
        )
        .with_context(|| format!("Attachment #{attachment_id} does not exist."))
}

fn raw_attachment_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawAttachment> {
    Ok(RawAttachment {
        attachment_id: row.get(0)?,
        entry_uuid: row.get(1)?,
        media_id: row.get(2)?,
        position: row.get(3)?,
        caption: row.get(4)?,
        alt_text: row.get(5)?,
        created_at: row.get(6)?,
        hash: row.get(7)?,
        mime_type: row.get(8)?,
        bytes: row.get(9)?,
        width: row.get(10)?,
        height: row.get(11)?,
        storage_backend: row.get(12)?,
        storage_key: row.get(13)?,
        deleted_at: row.get(14)?,
    })
}

fn attachment_from_raw(raw: RawAttachment, roots: &[PathBuf]) -> ImageAttachment {
    let original_available = media_exists(roots, &raw.storage_key);
    let thumbnail_available = media_exists(roots, &thumbnail_key(&raw.hash));
    ImageAttachment {
        attachment_id: raw.attachment_id,
        entry_uuid: raw.entry_uuid,
        media_id: raw.media_id,
        position: raw.position,
        caption: raw.caption,
        alt_text: raw.alt_text,
        created_at: raw.created_at,
        hash: raw.hash,
        mime_type: raw.mime_type,
        bytes: raw.bytes,
        width: raw.width,
        height: raw.height,
        storage_backend: raw.storage_backend,
        storage_key: raw.storage_key,
        deleted_at: raw.deleted_at,
        thumbnail_available,
        original_available,
    }
}

fn get_asset_by_hash(connection: &Connection, hash: &str) -> Result<Option<ImageAsset>> {
    connection
        .query_row(
            "SELECT id, hash, mime_type, bytes, width, height, storage_backend, storage_key, created_at, deleted_at
             FROM plugin_media_assets
             WHERE hash = ?1",
            [hash],
            |row| {
                Ok(ImageAsset {
                    id: row.get(0)?,
                    hash: row.get(1)?,
                    mime_type: row.get(2)?,
                    bytes: row.get(3)?,
                    width: row.get(4)?,
                    height: row.get(5)?,
                    storage_backend: row.get(6)?,
                    storage_key: row.get(7)?,
                    created_at: row.get(8)?,
                    deleted_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn ensure_image_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS plugin_media_assets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            hash TEXT NOT NULL UNIQUE,
            mime_type TEXT NOT NULL,
            bytes INTEGER NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            storage_backend TEXT NOT NULL,
            storage_key TEXT NOT NULL,
            created_at TEXT NOT NULL,
            deleted_at TEXT
        );
        CREATE TABLE IF NOT EXISTS plugin_entry_media (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_uuid TEXT NOT NULL,
            media_id INTEGER NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            caption TEXT,
            alt_text TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (entry_uuid) REFERENCES entries(uuid) ON DELETE CASCADE,
            FOREIGN KEY (media_id) REFERENCES plugin_media_assets(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_media_assets_hash ON plugin_media_assets(hash);
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_media_entry_uuid ON plugin_entry_media(entry_uuid);
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_media_media_id ON plugin_entry_media(media_id);
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_media_position ON plugin_entry_media(entry_uuid, position);
        ",
    )?;
    Ok(())
}

fn resolve_entry_uuid(connection: &Connection, identifier: &str) -> Result<String> {
    let identifier = normalize_string(Some(identifier))
        .ok_or_else(|| anyhow!("Entry identifier is required."))?;
    connection
        .query_row(
            "SELECT COALESCE(NULLIF(uuid, ''), 'entry_' || id)
             FROM entries
             WHERE uuid = ?1 OR CAST(id AS TEXT) = ?1
             LIMIT 1",
            [identifier.as_str()],
            |row| row.get::<_, String>(0),
        )
        .with_context(|| format!("entry not found: {identifier}"))
}

fn detected_tables(connection: &Connection) -> Result<HashSet<String>> {
    Ok(db::inspect_schema(connection)?
        .detected_tables
        .into_iter()
        .collect())
}

fn media_roots_for_database(db_path: &Path, override_root: Option<PathBuf>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = override_root {
        push_unique_path(&mut roots, root);
    }
    if let Ok(value) = env::var(MEDIA_ROOT_ENV) {
        if !value.trim().is_empty() {
            push_unique_path(&mut roots, PathBuf::from(value));
        }
    }
    if let Some(local_root) = db::local_image_media_root_for_database(db_path) {
        push_unique_path(&mut roots, local_root);
    }
    if let Some(configured) = config_media_root(db_path) {
        push_unique_path(&mut roots, configured);
    }
    push_unique_path(&mut roots, PathBuf::from(DEFAULT_MEDIA_ROOT));
    push_unique_path(
        &mut roots,
        db::database_directory_for_database(db_path).join("media"),
    );
    roots
}

fn config_media_root(db_path: &Path) -> Option<PathBuf> {
    let config_path = db::database_directory_for_database(db_path).join("config.json");
    let raw = fs::read(config_path).ok()?;
    let json = serde_json::from_slice::<serde_json::Value>(&raw).ok()?;
    json.get("images.media_root")
        .and_then(|value| value.as_str())
        .and_then(|value| normalize_string(Some(value)))
        .map(PathBuf::from)
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|item| item == &path) {
        paths.push(path);
    }
}

fn first_existing_or_default_root(roots: &[PathBuf]) -> Option<PathBuf> {
    roots
        .iter()
        .find(|root| root.exists())
        .cloned()
        .or_else(|| roots.first().cloned())
}

fn read_media_bytes(roots: &[PathBuf], storage_key: &str) -> Result<Vec<u8>> {
    for root in roots {
        if let Some(path) = safe_existing_path(root, storage_key)? {
            return fs::read(&path).with_context(|| format!("failed to read {}", path.display()));
        }
    }
    Err(anyhow!(
        "Image file is missing on disk for key '{}'.",
        storage_key
    ))
}

fn write_media_bytes(root: &Path, storage_key: &str, bytes: &[u8]) -> Result<()> {
    let relative = validate_relative_path(storage_key)?;
    fs::create_dir_all(root).with_context(|| format!("failed to create {}", root.display()))?;
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", root.display()))?;
    let target = root.join(relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        let parent = parent
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", parent.display()))?;
        if !parent.starts_with(&root) {
            return Err(anyhow!("media storage key escapes the media root"));
        }
    }
    if !target.exists() {
        fs::write(&target, bytes)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }
    Ok(())
}

fn safe_existing_path(root: &Path, storage_key: &str) -> Result<Option<PathBuf>> {
    let relative = validate_relative_path(storage_key)?;
    if !root.exists() {
        return Ok(None);
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", root.display()))?;
    let candidate = root.join(relative);
    if !candidate.exists() {
        return Ok(None);
    }
    let candidate = candidate
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", candidate.display()))?;
    if !candidate.starts_with(&root) {
        return Err(anyhow!("media storage key escapes the media root"));
    }
    Ok(Some(candidate))
}

fn media_exists(roots: &[PathBuf], storage_key: &str) -> bool {
    roots.iter().any(|root| {
        safe_existing_path(root, storage_key)
            .ok()
            .flatten()
            .is_some()
    })
}

fn validate_relative_path(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => output.push(value),
            Component::CurDir => {}
            _ => return Err(anyhow!("storage key must be a relative media path")),
        }
    }
    if output.as_os_str().is_empty() {
        return Err(anyhow!("storage key is required"));
    }
    Ok(output)
}

fn thumbnail_key(hash: &str) -> String {
    format!("thumb/{}/{}.jpg", &hash[..2], hash)
}

fn detect_image_mime(path: &Path, bytes: &[u8]) -> Result<String> {
    if let Some(mime) = mime_from_path(path) {
        return Ok(mime);
    }
    let format = image::guess_format(bytes).context("unsupported image file")?;
    match format {
        image::ImageFormat::Jpeg => Ok("image/jpeg".to_string()),
        image::ImageFormat::Png => Ok("image/png".to_string()),
        image::ImageFormat::WebP => Ok("image/webp".to_string()),
        _ => Err(anyhow!("unsupported image type")),
    }
}

fn mime_from_path(path: &Path) -> Option<String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => Some("image/jpeg".to_string()),
        Some("png") => Some("image/png".to_string()),
        Some("webp") => Some("image/webp".to_string()),
        _ => None,
    }
}

fn extension_for_mime(mime_type: &str) -> Result<&'static str> {
    match mime_type {
        "image/jpeg" => Ok("jpg"),
        "image/png" => Ok("png"),
        "image/webp" => Ok("webp"),
        other => Err(anyhow!("unsupported image MIME type: {other}")),
    }
}

fn build_thumbnail_bytes(bytes: &[u8], max_dimension: u32) -> Result<Vec<u8>> {
    let image = image::load_from_memory(bytes).context("failed to decode image")?;
    let thumb = image.thumbnail(max_dimension, max_dimension).to_rgb8();
    let mut output = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 82);
    encoder
        .encode_image(&thumb)
        .context("failed to encode thumbnail")?;
    Ok(output)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn data_url(mime_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime_type,
        general_purpose::STANDARD.encode(bytes)
    )
}

fn resolve_covers_root() -> PathBuf {
    if let Ok(value) = env::var(COVER_ROOT_ENV) {
        if !value.trim().is_empty() {
            return PathBuf::from(value);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("local-assets")
        .join("covers")
}

fn resolve_cover_thumbnail_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("local-assets")
        .join("cover_thumbnails")
}

fn iter_cover_files(root: &Path) -> Result<Vec<CoverFile>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut covers = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let path = entry?.path();
        if let Some(cover) = parse_cover_file(&path)? {
            covers.push(cover);
        }
    }
    Ok(covers)
}

fn resolve_cover_file(root: &Path, filename: &str) -> Result<CoverFile> {
    if filename.is_empty()
        || filename
            != Path::new(filename)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("")
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err(anyhow!("Cover not found."));
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", root.display()))?;
    let candidate = root.join(filename);
    let candidate = candidate
        .canonicalize()
        .with_context(|| format!("cover file does not exist: {}", filename))?;
    if !candidate.starts_with(&root) {
        return Err(anyhow!("Cover not found."));
    }
    parse_cover_file(&candidate)?.ok_or_else(|| anyhow!("Cover not found."))
}

fn parse_cover_file(path: &Path) -> Result<Option<CoverFile>> {
    if !path.is_file() || mime_from_path(path).is_none() {
        return Ok(None);
    }
    let stem = match path.file_stem().and_then(|value| value.to_str()) {
        Some(value) => value,
        None => return Ok(None),
    };
    let Some((cover_type, entry_uuid)) = stem.split_once('-') else {
        return Ok(None);
    };
    if !valid_cover_type(cover_type) || !entry_uuid.starts_with("entry_") {
        return Ok(None);
    }
    let metadata = fs::metadata(path)?;
    Ok(Some(CoverFile {
        filename: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
        cover_type: cover_type.to_lowercase(),
        entry_uuid: entry_uuid.to_string(),
        path: path.to_path_buf(),
        bytes: metadata.len(),
        modified_at: metadata.modified().ok().map(db::system_time_to_iso),
    }))
}

fn cover_thumbnail_cache_path(cover: &CoverFile) -> Result<PathBuf> {
    let metadata = fs::metadata(&cover.path)?;
    let fingerprint = format!(
        "{}|{:?}|{}",
        cover.path.canonicalize()?.display(),
        metadata.modified().ok(),
        metadata.len()
    );
    Ok(resolve_cover_thumbnail_root().join(format!("{}.jpg", sha256_hex(fingerprint.as_bytes()))))
}

fn cover_matches(
    cover: &CoverFile,
    entry: &CoverEntrySummary,
    cover_type: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
    tags: &HashSet<String>,
    moods: &HashSet<String>,
) -> bool {
    if let Some(cover_type) = cover_type {
        if cover.cover_type != cover_type {
            return false;
        }
    }
    let date = entry.created_at.get(0..10).unwrap_or(&entry.created_at);
    if let Some(since) = since.and_then(|value| normalize_string(Some(value))) {
        if date < since.as_str() {
            return false;
        }
    }
    if let Some(until) = until.and_then(|value| normalize_string(Some(value))) {
        if date > until.as_str() {
            return false;
        }
    }
    if !tags.is_empty()
        && !entry
            .tags
            .iter()
            .any(|tag| tags.contains(&tag.to_lowercase()))
    {
        return false;
    }
    if !moods.is_empty()
        && !entry
            .mood
            .as_deref()
            .map(|mood| moods.contains(&mood.to_lowercase()))
            .unwrap_or(false)
    {
        return false;
    }
    true
}

fn valid_cover_type(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphanumeric())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn normalized_set(values: Option<&[String]>) -> HashSet<String> {
    values
        .unwrap_or_default()
        .iter()
        .filter_map(|value| normalize_string(Some(value)))
        .map(|value| value.to_lowercase())
        .collect()
}

fn placeholders(count: usize) -> String {
    std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(", ")
}

fn normalize_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn current_timestamp_minutes() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use rusqlite::Connection;

    #[test]
    fn upload_attach_and_remove_image_preserves_backup_and_tombstone() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_image_fixture(temp_dir.path());
        let media_root = temp_dir.path().join("media");
        fs::write(
            temp_dir.path().join("config.json"),
            format!(
                "{{\"images.media_root\":\"{}\"}}",
                db::path_to_string(&media_root).replace('\\', "\\\\")
            ),
        )
        .expect("config");
        let source_path = temp_dir.path().join("source.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgb([42_u8, 90, 120]));
        image.save(&source_path).expect("png");
        let preview_data_url =
            get_local_image_preview_data_url(db::path_to_string(&source_path)).expect("preview");
        assert!(preview_data_url.starts_with("data:image/jpeg;base64,"));

        let upload =
            upload_image_for_database(&db_path, &source_path, &media_root).expect("upload");
        assert!(PathBuf::from(&upload.audit.backup_path).exists());
        assert_eq!(upload.audit.operation, "image.upload");
        assert_eq!(upload.asset.mime_type, "image/png");
        assert_eq!(
            upload.asset.storage_key,
            format!("{}/{}.png", &upload.asset.hash[..2], upload.asset.hash)
        );
        assert!(media_root.join(&upload.asset.storage_key).exists());
        assert!(media_root
            .join(format!(
                "thumb/{}/{}.jpg",
                &upload.asset.hash[..2],
                upload.asset.hash
            ))
            .exists());

        let attach = attach_image_for_database(
            &db_path,
            ImageAttachRequest {
                identifier: "entry_root".to_string(),
                media_id: upload.asset.id,
                caption: Some("Desk".to_string()),
                alt_text: Some("Desk image".to_string()),
                position: None,
            },
            &media_root,
        )
        .expect("attach");
        assert_eq!(attach.images.len(), 1);
        assert!(attach.images[0].original_available);
        assert!(attach.images[0].thumbnail_available);

        let data_url = get_image_data_url_for_database(
            &db_path,
            attach.images[0].attachment_id,
            ImageVariant::Thumb,
        )
        .expect("data url");
        assert!(data_url.starts_with("data:image/jpeg;base64,"));

        let removed =
            remove_image_for_database(&db_path, attach.images[0].attachment_id).expect("remove");
        assert_eq!(removed.images.len(), 0);
        assert_eq!(removed.audit.operation, "image.remove");

        let connection = Connection::open(&db_path).expect("open");
        let tombstones = connection
            .query_row("SELECT COUNT(*) FROM sync_image_tombstones", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("tombstones");
        assert_eq!(tombstones, 1);
    }

    #[test]
    fn cover_wall_reads_repo_local_covers() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_image_fixture(temp_dir.path());
        let covers_root = temp_dir.path().join("covers");
        fs::create_dir_all(&covers_root).expect("covers root");
        let cover_path = covers_root.join("magazine-entry_root.png");
        let image = ImageBuffer::from_pixel(6, 8, Rgb([120_u8, 40, 80]));
        image.save(&cover_path).expect("cover");

        let response = list_cover_wall_for_database_with_root(
            &db_path,
            CoverWallRequest::default(),
            &covers_root,
        )
        .expect("covers");

        assert_eq!(response.total, 1);
        assert_eq!(response.covers[0].cover_type, "magazine");
        assert_eq!(response.covers[0].entry.uuid, "entry_root");
    }

    fn upload_image_for_database(
        db_path: &Path,
        source_path: &Path,
        media_root: &Path,
    ) -> Result<ImageUploadResponse> {
        let source = db::path_to_string(source_path);
        let media_root = media_root.to_path_buf();
        let guarded =
            backup::with_database_backup_for_database(db_path, "image.upload", move |path| {
                upload_image_inner(path, &source, Some(media_root))
            })?;
        Ok(ImageUploadResponse {
            asset: guarded.value,
            audit: guarded.audit,
        })
    }

    fn attach_image_for_database(
        db_path: &Path,
        input: ImageAttachRequest,
        media_root: &Path,
    ) -> Result<ImageMutationResponse> {
        let media_root = media_root.to_path_buf();
        let guarded =
            backup::with_database_backup_for_database(db_path, "image.attach", move |path| {
                attach_image_inner(path, input, Some(media_root))
            })?;
        Ok(ImageMutationResponse {
            entry_uuid: guarded.value.0,
            images: guarded.value.1,
            audit: guarded.audit,
        })
    }

    fn remove_image_for_database(
        db_path: &Path,
        attachment_id: i64,
    ) -> Result<ImageMutationResponse> {
        let guarded =
            backup::with_database_backup_for_database(db_path, "image.remove", move |path| {
                remove_image_inner(path, attachment_id, None)
            })?;
        Ok(ImageMutationResponse {
            entry_uuid: guarded.value.0,
            images: guarded.value.1,
            audit: guarded.audit,
        })
    }

    fn list_cover_wall_for_database_with_root(
        db_path: &Path,
        input: CoverWallRequest,
        covers_root: &Path,
    ) -> Result<CoverWallResponse> {
        let all_covers = iter_cover_files(covers_root)?;
        let entry_uuids = all_covers
            .iter()
            .map(|cover| cover.entry_uuid.clone())
            .collect::<Vec<_>>();
        let entries = entries::list_entries_by_uuids_for_database(db_path, &entry_uuids)?;
        let entries_by_uuid = entries
            .into_iter()
            .filter(|entry| !entry.hidden)
            .map(|entry| {
                let summary = CoverEntrySummary {
                    id: entry.id,
                    uuid: entry.uuid.clone(),
                    created_at: entry.created_at,
                    title: entry.title,
                    mood: entry.mood,
                    tags: entry.tags.into_iter().map(|tag| tag.name).collect(),
                };
                (summary.uuid.clone(), summary)
            })
            .collect::<HashMap<_, _>>();
        let covers = all_covers
            .into_iter()
            .filter_map(|cover| {
                let entry = entries_by_uuid.get(&cover.entry_uuid)?;
                Some(EntryCover {
                    filename: cover.filename,
                    cover_type: cover.cover_type,
                    entry_uuid: cover.entry_uuid,
                    bytes: cover.bytes,
                    modified_at: cover.modified_at,
                    entry: entry.clone(),
                })
            })
            .collect::<Vec<_>>();
        Ok(CoverWallResponse {
            total: covers.len() as i64,
            covers,
            limit: input.limit.unwrap_or(DEFAULT_LIMIT),
            offset: input.offset.unwrap_or(0),
            available_types: vec!["magazine".to_string()],
            orphaned_cover_count: 0,
            covers_root: db::path_to_string(covers_root),
        })
    }

    fn create_image_fixture(path: &Path) -> PathBuf {
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
                    ('entry_root', '2026-01-01 08:00', '2026-01-01 08:00', 'Root text', 'Root text', 'markdown', 'Root', 'happy');
                INSERT INTO tags (name) VALUES ('covers');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1);
                ",
            )
            .expect("fixture");
        drop(connection);
        db_path
    }
}
