use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use thiserror::Error;
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

mod utils;
use utils::{display_path, non_empty_name, optional_text};

const SETTINGS_FILE: &str = "settings.json";
const DATABASE_FILE: &str = "library.db";

#[derive(Debug, Error)]
enum LibraryError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

type LibraryResult<T> = Result<T, LibraryError>;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    asset_storage_path: Option<PathBuf>,
    database_backup_path: Option<PathBuf>,
}

struct AppState {
    config_path: PathBuf,
    settings: Mutex<AppSettings>,
    legacy_import_running: Arc<AtomicBool>,
}

impl AppState {
    fn load(app: &AppHandle) -> LibraryResult<Self> {
        let config_dir = app.path().app_local_data_dir().map_err(|error| {
            LibraryError::Message(format!("設定フォルダを取得できません: {error}"))
        })?;
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join(SETTINGS_FILE);
        let settings = match fs::read_to_string(&config_path) {
            Ok(contents) => serde_json::from_str(&contents)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => AppSettings::default(),
            Err(error) => return Err(error.into()),
        };

        Ok(Self {
            config_path,
            settings: Mutex::new(settings),
            legacy_import_running: Arc::new(AtomicBool::new(false)),
        })
    }

    fn save_settings(&self, settings: &AppSettings) -> LibraryResult<()> {
        let contents = serde_json::to_vec_pretty(settings)?;
        let temporary_path = self.config_path.with_extension("json.tmp");
        fs::write(&temporary_path, contents)?;
        fs::rename(temporary_path, &self.config_path)?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryStatus {
    default_library_path: String,
    default_asset_storage_path: String,
    default_database_backup_path: String,
    selected_library_path: Option<String>,
    asset_storage_path: Option<String>,
    database_backup_path: Option<String>,
    initialized: bool,
    asset_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeLibraryRequest {
    asset_storage_path: String,
    database_backup_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    title: String,
    source_paths: Vec<String>,
    creator: Option<String>,
    source: Option<String>,
    product_url: Option<String>,
    version: Option<String>,
    note: Option<String>,
    #[serde(default)]
    tag_ids: Vec<String>,
    #[serde(default)]
    category_ids: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    asset_id: String,
    title: String,
    file_count: usize,
    total_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyDirectoryImportRequest {
    directory_path: String,
    #[serde(default)]
    tag_ids: Vec<String>,
    #[serde(default)]
    category_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyBatchImportRequest {
    directory_paths: Vec<String>,
    #[serde(default)]
    tag_ids: Vec<String>,
    #[serde(default)]
    category_ids: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LegacyImportFailure {
    source_filename: String,
    reason: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LegacyDirectoryImportResult {
    imported: Vec<ImportResult>,
    failures: Vec<LegacyImportFailure>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LegacyImportComplete {
    result: Option<LegacyDirectoryImportResult>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LegacyImportProgress {
    completed: usize,
    total: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Tag {
    id: String,
    name: String,
    color: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Category {
    id: String,
    parent_id: Option<String>,
    name: String,
    sort_order: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetSummary {
    id: String,
    title: String,
    creator: Option<String>,
    search_text: String,
    cover_data_url: Option<String>,
    tags: Vec<Tag>,
    categories: Vec<Category>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetFileView {
    id: String,
    original_filename: String,
    byte_size: i64,
    storage_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetRelationView {
    asset_id: String,
    title: String,
    relation_type: String,
    note: Option<String>,
    direction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetDetail {
    id: String,
    title: String,
    creator: Option<String>,
    source: Option<String>,
    product_url: Option<String>,
    version: Option<String>,
    note: Option<String>,
    cover_data_url: Option<String>,
    tags: Vec<Tag>,
    categories: Vec<Category>,
    files: Vec<AssetFileView>,
    relations: Vec<AssetRelationView>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetUpdateRequest {
    id: String,
    title: String,
    creator: Option<String>,
    source: Option<String>,
    product_url: Option<String>,
    version: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetAssignmentRequest {
    asset_id: String,
    ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetFilesRequest {
    asset_id: String,
    source_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NamedEntityRequest {
    name: String,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameTagRequest {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddRelationRequest {
    source_asset_id: String,
    target_asset_id: String,
    relation_type: String,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveRelationRequest {
    source_asset_id: String,
    target_asset_id: String,
    relation_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapacityReport {
    library_bytes: u64,
    asset_bytes: u64,
    thumbnail_cache_bytes: u64,
    database_backup_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupResult {
    path: String,
    created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupEntry {
    path: String,
    name: String,
    byte_size: u64,
}

#[derive(Debug, Clone)]
struct StorageLocations {
    library_root: PathBuf,
    asset_storage_root: PathBuf,
    database_backup_root: PathBuf,
}

#[tauri::command]
fn get_library_status(app: AppHandle, state: State<'_, AppState>) -> Result<LibraryStatus, String> {
    let default_library_path = default_library_path(&app).map_err(|error| error.to_string())?;
    let default_asset_storage_path = default_library_path.join("asset-storage");
    let default_database_backup_path = default_library_path.join("database-backups");
    let settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?
        .clone();
    let database_path = default_library_path.join(DATABASE_FILE);
    let asset_storage_path = settings
        .asset_storage_path
        .unwrap_or_else(|| default_asset_storage_path.clone());
    let database_backup_path = settings
        .database_backup_path
        .unwrap_or_else(|| default_database_backup_path.clone());
    let initialized = database_path.is_file();
    let asset_count = if initialized {
        count_assets(&database_path).unwrap_or(0)
    } else {
        0
    };

    Ok(LibraryStatus {
        default_library_path: display_path(&default_library_path),
        default_asset_storage_path: display_path(&default_asset_storage_path),
        default_database_backup_path: display_path(&default_database_backup_path),
        selected_library_path: Some(display_path(&default_library_path)),
        asset_storage_path: Some(display_path(&asset_storage_path)),
        database_backup_path: Some(display_path(&database_backup_path)),
        initialized,
        asset_count,
    })
}

#[tauri::command]
fn initialize_library(
    app: AppHandle,
    state: State<'_, AppState>,
    request: InitializeLibraryRequest,
) -> Result<(), String> {
    let library_path = default_library_path(&app).map_err(|error| error.to_string())?;
    let asset_storage_path = validate_storage_path(&request.asset_storage_path, "アセット保存先")
        .map_err(|error| error.to_string())?;
    let database_backup_path =
        validate_storage_path(&request.database_backup_path, "DBバックアップ先")
            .map_err(|error| error.to_string())?;
    prepare_library(&library_path, &asset_storage_path, &database_backup_path)
        .map_err(|error| error.to_string())?;

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?;
    settings.asset_storage_path = Some(asset_storage_path);
    settings.database_backup_path = Some(database_backup_path);
    state
        .save_settings(&settings)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn import_files(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ImportRequest,
) -> Result<ImportResult, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?
        .clone();
    let library_path = default_library_path(&app).map_err(|error| error.to_string())?;
    let asset_storage_path = settings
        .asset_storage_path
        .unwrap_or_else(|| library_path.join("assets"));

    let tag_ids = request.tag_ids.clone();
    let category_ids = request.category_ids.clone();
    let result = import_into_library(&library_path, &asset_storage_path, request)
        .map_err(|error| error.to_string())?;
    if let Err(error) = assign_import_labels(
        &library_path.join(DATABASE_FILE),
        &result.asset_id,
        &tag_ids,
        &category_ids,
    ) {
        let _ = remove_imported_asset(
            &library_path.join(DATABASE_FILE),
            &asset_storage_path,
            &result.asset_id,
        );
        return Err(error.to_string());
    }
    Ok(result)
}

#[tauri::command]
fn count_legacy_archives(directory_path: String) -> Result<usize, String> {
    legacy_archives(Path::new(directory_path.trim()))
        .map(|archives| archives.len())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn import_legacy_directory(
    app: AppHandle,
    state: State<'_, AppState>,
    request: LegacyDirectoryImportRequest,
) -> Result<LegacyDirectoryImportResult, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let archives = legacy_archives(Path::new(request.directory_path.trim()))
        .map_err(|error| error.to_string())?;
    let total = archives.len();

    let mut imported = Vec::new();
    let mut failures = Vec::new();
    for (index, archive) in archives.into_iter().enumerate() {
        let source_filename = archive
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("(名前を取得できないZIP)")
            .to_owned();
        match import_legacy_archive(&locations, &archive) {
            Ok(result) => {
                if let Err(error) = assign_import_labels(
                    &locations.library_root.join(DATABASE_FILE),
                    &result.asset_id,
                    &request.tag_ids,
                    &request.category_ids,
                ) {
                    let _ = remove_imported_asset(
                        &locations.library_root.join(DATABASE_FILE),
                        &locations.asset_storage_root,
                        &result.asset_id,
                    );
                    failures.push(LegacyImportFailure {
                        source_filename,
                        reason: format!("ラベルの設定に失敗しました: {error}"),
                    });
                } else {
                    imported.push(result);
                }
            }
            Err(error) => failures.push(LegacyImportFailure {
                source_filename,
                reason: error.to_string(),
            }),
        }
        let _ = app.emit(
            "legacy-import-progress",
            LegacyImportProgress {
                completed: index + 1,
                total,
            },
        );
    }
    Ok(LegacyDirectoryImportResult { imported, failures })
}

#[tauri::command]
fn start_legacy_import(
    app: AppHandle,
    state: State<'_, AppState>,
    request: LegacyBatchImportRequest,
) -> Result<(), String> {
    if request.directory_paths.is_empty() {
        return Err("取り込み元フォルダを指定してください".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let running = Arc::clone(&state.legacy_import_running);
    if running.swap(true, Ordering::AcqRel) {
        return Err("旧形式ZIPの取り込みはすでに実行中です".into());
    }

    std::thread::spawn(move || {
        let result = import_legacy_batch(&app, &locations, request);
        let complete = match result {
            Ok(result) => LegacyImportComplete {
                result: Some(result),
                error: None,
            },
            Err(error) => LegacyImportComplete {
                result: None,
                error: Some(error.to_string()),
            },
        };
        running.store(false, Ordering::Release);
        let _ = app.emit("legacy-import-complete", complete);
    });
    Ok(())
}

fn import_legacy_batch(
    app: &AppHandle,
    locations: &StorageLocations,
    request: LegacyBatchImportRequest,
) -> LibraryResult<LegacyDirectoryImportResult> {
    let mut archives = Vec::new();
    for directory_path in request.directory_paths {
        archives.extend(legacy_archives(Path::new(directory_path.trim()))?);
    }
    let total = archives.len();
    let mut imported = Vec::new();
    let mut failures = Vec::new();
    for (index, archive) in archives.into_iter().enumerate() {
        let source_filename = archive
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("(名前を取得できないZIP)")
            .to_owned();
        match import_legacy_archive(locations, &archive) {
            Ok(result) => {
                if let Err(error) = assign_import_labels(
                    &locations.library_root.join(DATABASE_FILE),
                    &result.asset_id,
                    &request.tag_ids,
                    &request.category_ids,
                ) {
                    let _ = remove_imported_asset(
                        &locations.library_root.join(DATABASE_FILE),
                        &locations.asset_storage_root,
                        &result.asset_id,
                    );
                    failures.push(LegacyImportFailure {
                        source_filename,
                        reason: format!("ラベルの設定に失敗しました: {error}"),
                    });
                } else {
                    imported.push(result);
                }
            }
            Err(error) => failures.push(LegacyImportFailure {
                source_filename,
                reason: error.to_string(),
            }),
        }
        let _ = app.emit(
            "legacy-import-progress",
            LegacyImportProgress {
                completed: index + 1,
                total,
            },
        );
    }
    Ok(LegacyDirectoryImportResult { imported, failures })
}

fn legacy_archives(directory: &Path) -> LibraryResult<Vec<PathBuf>> {
    if !directory.is_dir() {
        return Err(LibraryError::Message(
            "取り込み元としてフォルダを指定してください".into(),
        ));
    }
    let mut archives = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        })
        .collect::<Vec<_>>();
    archives.sort();
    Ok(archives)
}

#[tauri::command]
fn list_assets(
    app: AppHandle,
    state: State<'_, AppState>,
    query: Option<String>,
) -> Result<Vec<AssetSummary>, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    let search = format!("%{}%", query.unwrap_or_default().trim());
    let mut statement = connection
        .prepare(
            "SELECT id, title, creator, cover_path,
                    title || ' ' || COALESCE(creator, '') || ' ' || COALESCE(source, '') || ' ' ||
                    COALESCE(product_url, '') || ' ' ||
                    COALESCE(version, '') || ' ' || COALESCE(note, '')
             FROM assets
             WHERE title LIKE ?1 OR COALESCE(creator, '') LIKE ?1 OR COALESCE(note, '') LIKE ?1
             ORDER BY updated_at DESC, title COLLATE NOCASE",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![search], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;

    let mut assets = Vec::new();
    for row in rows {
        let (id, title, creator, cover_path, search_text) =
            row.map_err(|error| error.to_string())?;
        let tags = tags_for_asset(&connection, &id).map_err(|error| error.to_string())?;
        let categories =
            categories_for_asset(&connection, &id).map_err(|error| error.to_string())?;
        assets.push(AssetSummary {
            id,
            title,
            creator,
            search_text,
            cover_data_url: cover_path
                .and_then(|path| cover_data_url(&locations.asset_storage_root, &path)),
            tags,
            categories,
        });
    }
    Ok(assets)
}

#[tauri::command]
fn get_asset_detail(
    app: AppHandle,
    state: State<'_, AppState>,
    asset_id: String,
) -> Result<AssetDetail, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    asset_detail(&connection, &locations.asset_storage_root, &asset_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_asset(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetUpdateRequest,
) -> Result<(), String> {
    if request.title.trim().is_empty() {
        return Err("アセット名を入力してください".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE assets SET title=?2, creator=?3, source=?4, product_url=?5,
             version=?6, note=?7, updated_at=?8 WHERE id=?1",
            params![
                request.id,
                request.title.trim(),
                optional_text(request.creator),
                optional_text(request.source),
                optional_text(request.product_url),
                optional_text(request.version),
                optional_text(request.note),
                unix_timestamp(),
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn list_tags(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Tag>, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    all_tags(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    request: NamedEntityRequest,
) -> Result<Tag, String> {
    let name = non_empty_name(&request.name).map_err(|error| error.to_string())?;
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    let id = Uuid::now_v7().to_string();
    connection
        .execute(
            "INSERT OR IGNORE INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
            params![id, name, optional_text(request.color)],
        )
        .map_err(|error| error.to_string())?;
    connection
        .query_row(
            "SELECT id, name, color FROM tags WHERE name=?1 COLLATE NOCASE",
            params![name],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_tag(app: AppHandle, state: State<'_, AppState>, tag_id: String) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| error.to_string())?;
    connection
        .execute("DELETE FROM tags WHERE id=?1", params![tag_id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_category(
    app: AppHandle,
    state: State<'_, AppState>,
    category_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let mut connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    // Removing the assignment first deliberately leaves affected assets uncategorized.
    transaction
        .execute(
            "DELETE FROM asset_categories WHERE category_id=?1",
            params![category_id],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM categories WHERE id=?1", params![category_id])
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn rename_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RenameTagRequest,
) -> Result<(), String> {
    let name = non_empty_name(&request.name).map_err(|error| error.to_string())?;
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    let changed = connection
        .execute(
            "UPDATE tags SET name=?1 WHERE id=?2",
            params![name, request.id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("タグが見つかりません".into());
    }
    Ok(())
}

#[tauri::command]
fn delete_asset(
    app: AppHandle,
    state: State<'_, AppState>,
    asset_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    remove_imported_asset(
        &locations.library_root.join(DATABASE_FILE),
        &locations.asset_storage_root,
        &asset_id,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_asset_files(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetFilesRequest,
) -> Result<(), String> {
    if request.source_paths.is_empty() {
        return Err("少なくとも1つのファイルを選んでください".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let database_path = locations.library_root.join(DATABASE_FILE);
    let destination = asset_files_directory(&locations.asset_storage_root, &request.asset_id)
        .map_err(|error| error.to_string())?;
    let connection = Connection::open(&database_path).map_err(|error| error.to_string())?;
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE id=?1)",
            params![request.asset_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err("アセットが見つかりません".into());
    }

    let sources = request
        .source_paths
        .into_iter()
        .map(PathBuf::from)
        .map(|source| {
            let metadata = fs::metadata(&source)?;
            if !metadata.is_file() {
                return Err(LibraryError::Message("通常ファイルのみ追加できます".into()));
            }
            Ok((source, metadata.len()))
        })
        .collect::<LibraryResult<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    let mut files = Vec::new();
    for (source, byte_size) in sources {
        let id = Uuid::now_v7().to_string();
        let original_filename = safe_filename(&source).map_err(|error| error.to_string())?;
        let stored_filename = format!("{id}-{original_filename}");
        let target = destination.join(&stored_filename);
        if let Err(error) = fs::copy(&source, &target) {
            let _ = fs::remove_file(target);
            cleanup_added_files(&destination, &files);
            return Err(error.to_string());
        }
        files.push(ImportedFile {
            id,
            original_filename,
            stored_filename,
            byte_size,
        });
    }
    let mut connection = connection;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let timestamp = unix_timestamp();
    let result = (|| -> LibraryResult<()> {
        for file in &files {
            transaction.execute("INSERT INTO asset_files (id, asset_id, storage_mode, relative_path, external_path, original_filename, byte_size, sha256, created_at) VALUES (?1, ?2, 'managed', ?3, NULL, ?4, ?5, NULL, ?6)", params![file.id, request.asset_id, format!("assets/{}/files/{}", request.asset_id, file.stored_filename), file.original_filename, file.byte_size as i64, timestamp])?;
        }
        transaction.execute(
            "UPDATE assets SET updated_at=?1 WHERE id=?2",
            params![timestamp, request.asset_id],
        )?;
        transaction.commit()?;
        Ok(())
    })();
    if result.is_err() {
        cleanup_added_files(&destination, &files);
    }
    result.map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_asset_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let database_path = locations.library_root.join(DATABASE_FILE);
    let connection = Connection::open(&database_path).map_err(|error| error.to_string())?;
    let (asset_id, storage_mode, relative): (String, String, Option<String>) = connection
        .query_row(
            "SELECT asset_id, storage_mode, relative_path FROM asset_files WHERE id=?1",
            params![file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| error.to_string())?;
    if storage_mode == "managed" {
        let filename = Path::new(
            relative
                .as_deref()
                .ok_or_else(|| "管理ファイルの保存先が不正です".to_string())?,
        )
        .file_name()
        .ok_or_else(|| "管理ファイルの保存先が不正です".to_string())?;
        let path = asset_files_directory(&locations.asset_storage_root, &asset_id)
            .map_err(|error| error.to_string())?
            .join(filename);
        if path.is_file() {
            let deletion_path = path.with_file_name(format!(".deleting-{file_id}"));
            fs::rename(&path, &deletion_path).map_err(|error| error.to_string())?;
            let result =
                connection.execute("DELETE FROM asset_files WHERE id=?1", params![file_id]);
            if let Err(error) = result {
                let _ = fs::rename(&deletion_path, &path);
                return Err(error.to_string());
            }
            fs::remove_file(deletion_path).map_err(|error| error.to_string())?;
            return Ok(());
        }
    }
    connection
        .execute("DELETE FROM asset_files WHERE id=?1", params![file_id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_asset_tags(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetAssignmentRequest,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let mut connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM asset_tags WHERE asset_id=?1",
            params![request.asset_id],
        )
        .map_err(|error| error.to_string())?;
    for tag_id in request.ids {
        transaction
            .execute(
                "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
                params![request.asset_id, tag_id],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[tauri::command]
fn list_categories(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    all_categories(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_category(
    app: AppHandle,
    state: State<'_, AppState>,
    request: NamedEntityRequest,
) -> Result<Category, String> {
    let name = non_empty_name(&request.name).map_err(|error| error.to_string())?;
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    let category = Category {
        id: Uuid::now_v7().to_string(),
        parent_id: None,
        name,
        sort_order: 0,
    };
    connection
        .execute(
            "INSERT INTO categories (id, parent_id, name, sort_order) VALUES (?1, NULL, ?2, 0)",
            params![category.id, category.name],
        )
        .map_err(|error| error.to_string())?;
    Ok(category)
}

#[tauri::command]
fn set_asset_categories(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetAssignmentRequest,
) -> Result<(), String> {
    if request.ids.len() > 1 {
        return Err("カテゴリはアセットごとに1つだけ設定できます".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let mut connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM asset_categories WHERE asset_id=?1",
            params![request.asset_id],
        )
        .map_err(|error| error.to_string())?;
    for category_id in request.ids {
        transaction
            .execute(
                "INSERT OR IGNORE INTO asset_categories (asset_id, category_id) VALUES (?1, ?2)",
                params![request.asset_id, category_id],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[tauri::command]
fn add_asset_relation(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AddRelationRequest,
) -> Result<(), String> {
    if request.source_asset_id == request.target_asset_id {
        return Err("同じアセット同士は関連付けできません".into());
    }
    if ![
        "addon_for",
        "compatible_with",
        "requires",
        "variant_of",
        "related_to",
    ]
    .contains(&request.relation_type.as_str())
    {
        return Err("不正な関連タイプです".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    connection.execute("INSERT OR REPLACE INTO asset_relations (source_asset_id, target_asset_id, relation_type, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![request.source_asset_id, request.target_asset_id, request.relation_type, optional_text(request.note), unix_timestamp()]).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn remove_asset_relation(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RemoveRelationRequest,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    connection.execute("DELETE FROM asset_relations WHERE source_asset_id=?1 AND target_asset_id=?2 AND relation_type=?3", params![request.source_asset_id, request.target_asset_id, request.relation_type]).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_asset_cover(
    app: AppHandle,
    state: State<'_, AppState>,
    asset_id: String,
    source_path: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let extension = Path::new(&source_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|value| ["png", "jpg", "jpeg", "webp", "gif"].contains(&value.as_str()))
        .ok_or_else(|| "PNG、JPEG、WebP、GIF画像を選んでください".to_string())?;
    let relative_path = format!("assets/{asset_id}/cover.{extension}");
    let destination = locations.asset_storage_root.join(&relative_path);
    let old_path: Option<String> = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?
        .query_row(
            "SELECT cover_path FROM assets WHERE id=?1",
            params![asset_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(
        destination
            .parent()
            .ok_or_else(|| "カバー画像の保存先が不正です".to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::copy(&source_path, &destination).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE assets SET cover_path=?2, updated_at=?3 WHERE id=?1",
            params![asset_id, relative_path, unix_timestamp()],
        )
        .map_err(|error| error.to_string())?;
    if let Some(old_path) = old_path {
        let old = locations.asset_storage_root.join(old_path);
        if old != destination {
            let _ = fs::remove_file(old);
        }
    }
    Ok(())
}

#[tauri::command]
fn get_asset_file_path(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<String, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection = Connection::open(locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    let (storage_mode, relative_path, external_path): (String, Option<String>, Option<String>) =
        connection
            .query_row(
                "SELECT storage_mode, relative_path, external_path FROM asset_files WHERE id=?1",
                params![file_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| error.to_string())?;
    let path = if storage_mode == "managed" {
        locations
            .asset_storage_root
            .join(relative_path.ok_or_else(|| "管理ファイルのパスがありません".to_string())?)
    } else {
        PathBuf::from(external_path.ok_or_else(|| "外部ファイルのパスがありません".to_string())?)
    };
    Ok(display_path(&path))
}

#[tauri::command]
fn reveal_asset_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), String> {
    let path = get_asset_file_path(app, state, file_id)?;
    reveal_in_file_manager(Path::new(&path)).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("HTTP(S) のURLだけを開けます".into());
    }
    open_with_default_application(url).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_capacity_report(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CapacityReport, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    Ok(CapacityReport {
        library_bytes: directory_size(&locations.library_root),
        asset_bytes: directory_size(&locations.asset_storage_root.join("assets")),
        thumbnail_cache_bytes: directory_size(&locations.library_root.join("thumbnail-cache")),
        database_backup_bytes: directory_size(&locations.database_backup_root),
    })
}

#[tauri::command]
fn create_database_backup(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BackupResult, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    fs::create_dir_all(&locations.database_backup_root).map_err(|error| error.to_string())?;
    let created_at = unix_timestamp();
    let backup_path = locations
        .database_backup_root
        .join(format!("library-{created_at}-{}.db", Uuid::now_v7()));
    let temporary_path = backup_path.with_extension("db.partial");
    copy_database_snapshot(&locations.library_root.join(DATABASE_FILE), &temporary_path)
        .map_err(|error| error.to_string())?;
    fs::rename(&temporary_path, &backup_path).map_err(|error| error.to_string())?;
    Ok(BackupResult {
        path: display_path(&backup_path),
        created_at,
    })
}

#[tauri::command]
fn list_database_backups(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<BackupEntry>, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let mut backups = fs::read_dir(&locations.database_backup_root)
        .map_err(|error| error.to_string())?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let is_backup = path.extension().and_then(|extension| extension.to_str()) == Some("db");
            is_backup
                .then(|| {
                    entry.metadata().ok().map(|metadata| BackupEntry {
                        path: display_path(&path),
                        name: entry.file_name().to_string_lossy().into_owned(),
                        byte_size: metadata.len(),
                    })
                })
                .flatten()
        })
        .collect::<Vec<_>>();
    backups.sort_by(|left, right| right.name.cmp(&left.name));
    Ok(backups)
}

#[tauri::command]
fn restore_database_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let backup_path = PathBuf::from(backup_path);
    let backup_root =
        fs::canonicalize(&locations.database_backup_root).map_err(|error| error.to_string())?;
    let backup_file = fs::canonicalize(&backup_path).map_err(|error| error.to_string())?;
    if !backup_file.starts_with(&backup_root)
        || backup_file
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("db")
    {
        return Err("指定されたDBバックアップを復元できません".into());
    }
    copy_database_snapshot(&backup_file, &locations.library_root.join(DATABASE_FILE))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn default_library_path(app: &AppHandle) -> LibraryResult<PathBuf> {
    let path = app
        .path()
        .app_local_data_dir()
        .map_err(|error| LibraryError::Message(format!("既定の保存先を取得できません: {error}")))?;
    Ok(path.join("library"))
}

fn storage_locations(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> LibraryResult<StorageLocations> {
    let library_root = default_library_path(app)?;
    let settings = state
        .settings
        .lock()
        .map_err(|_| LibraryError::Message("設定のロックに失敗しました".into()))?
        .clone();
    Ok(StorageLocations {
        asset_storage_root: settings
            .asset_storage_path
            .unwrap_or_else(|| library_root.join("asset-storage")),
        database_backup_root: settings
            .database_backup_path
            .unwrap_or_else(|| library_root.join("database-backups")),
        library_root,
    })
}

fn all_tags(connection: &Connection) -> LibraryResult<Vec<Tag>> {
    let mut statement =
        connection.prepare("SELECT id, name, color FROM tags ORDER BY name COLLATE NOCASE")?;
    let rows = statement.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn tags_for_asset(connection: &Connection, asset_id: &str) -> LibraryResult<Vec<Tag>> {
    let mut statement = connection.prepare("SELECT t.id, t.name, t.color FROM tags t JOIN asset_tags at ON at.tag_id=t.id WHERE at.asset_id=?1 ORDER BY t.name COLLATE NOCASE")?;
    let rows = statement.query_map(params![asset_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn all_categories(connection: &Connection) -> LibraryResult<Vec<Category>> {
    let mut statement = connection.prepare("SELECT id, parent_id, name, sort_order FROM categories ORDER BY sort_order, name COLLATE NOCASE")?;
    let rows = statement.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn categories_for_asset(connection: &Connection, asset_id: &str) -> LibraryResult<Vec<Category>> {
    let mut statement = connection.prepare("SELECT c.id, c.parent_id, c.name, c.sort_order FROM categories c JOIN asset_categories ac ON ac.category_id=c.id WHERE ac.asset_id=?1 ORDER BY c.sort_order, c.name COLLATE NOCASE")?;
    let rows = statement.query_map(params![asset_id], |row| {
        Ok(Category {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn asset_detail(
    connection: &Connection,
    asset_root: &Path,
    asset_id: &str,
) -> LibraryResult<AssetDetail> {
    let (id, title, creator, source, product_url, version, note, cover_path): (String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) = connection.query_row(
        "SELECT id, title, creator, source, product_url, version, note, cover_path FROM assets WHERE id=?1",
        params![asset_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
    )?;
    let mut file_statement = connection.prepare("SELECT id, original_filename, byte_size, storage_mode FROM asset_files WHERE asset_id=?1 ORDER BY created_at")?;
    let files = file_statement
        .query_map(params![asset_id], |row| {
            Ok(AssetFileView {
                id: row.get(0)?,
                original_filename: row.get(1)?,
                byte_size: row.get(2)?,
                storage_mode: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut category_statement = connection.prepare("SELECT c.id, c.parent_id, c.name, c.sort_order FROM categories c JOIN asset_categories ac ON ac.category_id=c.id WHERE ac.asset_id=?1 ORDER BY c.sort_order, c.name COLLATE NOCASE")?;
    let categories = category_statement
        .query_map(params![asset_id], |row| {
            Ok(Category {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                sort_order: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut relation_statement = connection.prepare(
        "SELECT r.target_asset_id, a.title, r.relation_type, r.note, 'outgoing' FROM asset_relations r JOIN assets a ON a.id=r.target_asset_id WHERE r.source_asset_id=?1
         UNION ALL
         SELECT r.source_asset_id, a.title, r.relation_type, r.note, 'incoming' FROM asset_relations r JOIN assets a ON a.id=r.source_asset_id WHERE r.target_asset_id=?1"
    )?;
    let relations = relation_statement
        .query_map(params![asset_id], |row| {
            Ok(AssetRelationView {
                asset_id: row.get(0)?,
                title: row.get(1)?,
                relation_type: row.get(2)?,
                note: row.get(3)?,
                direction: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AssetDetail {
        id,
        title,
        creator,
        source,
        product_url,
        version,
        note,
        cover_data_url: cover_path.and_then(|path| cover_data_url(asset_root, &path)),
        tags: tags_for_asset(connection, asset_id)?,
        categories,
        files,
        relations,
    })
}

fn cover_data_url(asset_root: &Path, relative_path: &str) -> Option<String> {
    let extension = Path::new(relative_path)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    };
    let bytes = fs::read(asset_root.join(relative_path)).ok()?;
    Some(format!("data:{mime};base64,{}", BASE64.encode(bytes)))
}

fn directory_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            match entry.metadata() {
                Ok(metadata) if metadata.is_dir() => directory_size(&path),
                Ok(metadata) => metadata.len(),
                Err(_) => 0,
            }
        })
        .sum()
}

fn copy_database_snapshot(source_path: &Path, destination_path: &Path) -> LibraryResult<()> {
    let source = Connection::open(source_path)?;
    let mut destination = Connection::open(destination_path)?;
    let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
    backup.run_to_completion(32, Duration::from_millis(10), None)?;
    Ok(())
}

fn reveal_in_file_manager(path: &Path) -> LibraryResult<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer.exe");
        command.arg(format!("/select,{}", display_path(path)));
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path.parent().unwrap_or(path));
        command
    };
    command.spawn()?;
    Ok(())
}

fn open_with_default_application(target: &str) -> LibraryResult<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer.exe");
        command.arg(target);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(target);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(target);
        command
    };
    command.spawn()?;
    Ok(())
}

fn validate_storage_path(input: &str, label: &str) -> LibraryResult<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(LibraryError::Message(format!("{label}を指定してください")));
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(LibraryError::Message(format!(
            "{label}には絶対パスを指定してください"
        )));
    }
    Ok(path)
}

fn validated_asset_id(asset_id: &str) -> LibraryResult<&str> {
    Uuid::parse_str(asset_id).map_err(|_| LibraryError::Message("アセットIDが不正です".into()))?;
    Ok(asset_id)
}

fn asset_directory(asset_storage_root: &Path, asset_id: &str) -> LibraryResult<PathBuf> {
    Ok(asset_storage_root
        .join("assets")
        .join(validated_asset_id(asset_id)?))
}

fn asset_files_directory(asset_storage_root: &Path, asset_id: &str) -> LibraryResult<PathBuf> {
    Ok(asset_directory(asset_storage_root, asset_id)?.join("files"))
}

fn cleanup_added_files(destination: &Path, files: &[ImportedFile]) {
    for file in files {
        let _ = fs::remove_file(destination.join(&file.stored_filename));
    }
}

fn prepare_library(
    root: &Path,
    asset_storage_root: &Path,
    database_backup_root: &Path,
) -> LibraryResult<()> {
    for directory in ["thumbnail-cache", "exports"] {
        fs::create_dir_all(root.join(directory))?;
    }
    fs::create_dir_all(asset_storage_root.join("assets"))?;
    fs::create_dir_all(database_backup_root)?;
    let database_path = root.join(DATABASE_FILE);
    let connection = Connection::open(database_path)?;
    migrate(&connection)?;
    Ok(())
}

fn import_into_library(
    root: &Path,
    asset_storage_root: &Path,
    request: ImportRequest,
) -> LibraryResult<ImportResult> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err(LibraryError::Message("アセット名を入力してください".into()));
    }
    if title.chars().count() > 256 {
        return Err(LibraryError::Message(
            "アセット名は256文字以内にしてください".into(),
        ));
    }
    if request.source_paths.is_empty() {
        return Err(LibraryError::Message(
            "少なくとも1つのファイルを選んでください".into(),
        ));
    }

    let database_path = root.join(DATABASE_FILE);
    if !database_path.is_file() {
        return Err(LibraryError::Message(
            "ライブラリが初期化されていません".into(),
        ));
    }

    let asset_id = Uuid::now_v7().to_string();
    let staging_asset_dir = std::env::temp_dir().join("VRAssetLibrary").join(&asset_id);
    let staging_files_dir = staging_asset_dir.join("files");
    fs::create_dir_all(&staging_files_dir)?;

    let import_result = (|| -> LibraryResult<ImportResult> {
        let mut files = Vec::with_capacity(request.source_paths.len());
        let mut total_bytes = 0_u64;

        for source_path in &request.source_paths {
            let source_path = PathBuf::from(source_path);
            let metadata = fs::metadata(&source_path)?;
            if !metadata.is_file() {
                return Err(LibraryError::Message(format!(
                    "通常ファイルではないため取り込めません: {}",
                    display_path(&source_path)
                )));
            }

            let file_id = Uuid::now_v7().to_string();
            let original_filename = safe_filename(&source_path)?;
            let stored_filename = format!("{file_id}-{original_filename}");
            let staged_path = staging_files_dir.join(&stored_filename);
            let byte_size = copy_file(&source_path, &staged_path)?;
            total_bytes = total_bytes.saturating_add(byte_size);

            files.push(ImportedFile {
                id: file_id,
                original_filename,
                stored_filename,
                byte_size,
            });
        }

        let asset_directory = asset_storage_root.join("assets").join(&asset_id);
        copy_staged_asset(&staging_asset_dir, &asset_directory, &files)?;

        let database_result = insert_imported_asset(&database_path, &asset_id, title, &files)
            .and_then(|_| apply_import_metadata(&database_path, &asset_id, &request));
        if let Err(error) = database_result {
            let _ = fs::remove_dir_all(&asset_directory);
            return Err(error);
        }

        Ok(ImportResult {
            asset_id,
            title: title.to_owned(),
            file_count: files.len(),
            total_bytes,
        })
    })();

    if import_result.is_err() {
        let _ = fs::remove_dir_all(&staging_asset_dir);
    }
    import_result
}

fn apply_import_metadata(
    database_path: &Path,
    asset_id: &str,
    request: &ImportRequest,
) -> LibraryResult<()> {
    let connection = Connection::open(database_path)?;
    connection.execute(
        "UPDATE assets
         SET creator = ?1, source = ?2, product_url = ?3,
             version = ?4, note = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            request.creator,
            request.source,
            request.product_url,
            request.version,
            request.note,
            unix_timestamp(),
            asset_id,
        ],
    )?;
    Ok(())
}

fn import_legacy_archive(
    locations: &StorageLocations,
    archive_path: &Path,
) -> LibraryResult<ImportResult> {
    let database_path = locations.library_root.join(DATABASE_FILE);
    if !database_path.is_file() {
        return Err(LibraryError::Message(
            "ライブラリが初期化されていません".into(),
        ));
    }

    let archive_file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(BufReader::new(archive_file))
        .map_err(|error| LibraryError::Message(format!("ZIPを開けません: {error}")))?;
    let names = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .map(|entry| entry.name().to_owned())
                .map_err(|error| LibraryError::Message(format!("ZIPを読み取れません: {error}")))
        })
        .collect::<LibraryResult<Vec<_>>>()?;
    let info_name = names
        .iter()
        .find(|name| name.as_str() == "_assetinfo.md")
        .ok_or_else(|| LibraryError::Message("_assetinfo.md がありません".into()))?;
    let thumbnail_name = names
        .iter()
        .find(|name| name.as_str() == "_thumbnail.jpg")
        .ok_or_else(|| LibraryError::Message("_thumbnail.jpg がありません".into()))?;

    let mut roots = names
        .iter()
        .filter_map(|name| archive_root_directory(name))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    if roots.len() != 1 {
        return Err(LibraryError::Message(
            "商品ファイルの最上位ディレクトリを1つだけ含めてください".into(),
        ));
    }
    let product_directory = roots.remove(0);
    let title = safe_archive_name(&product_directory)?;
    let info = read_zip_entry(&mut archive, info_name)?;
    let thumbnail = read_zip_entry(&mut archive, thumbnail_name)?;
    let product_prefix = format!("{product_directory}/");
    let product_entries = names
        .iter()
        .filter(|name| name.starts_with(&product_prefix))
        .cloned()
        .collect::<Vec<_>>();
    if product_entries.iter().any(|name| !safe_archive_entry(name)) {
        return Err(LibraryError::Message(
            "ZIP内に安全でないパスが含まれています".into(),
        ));
    }

    let asset_id = Uuid::now_v7().to_string();
    let staging_asset_dir = std::env::temp_dir().join("VRAssetLibrary").join(&asset_id);
    let staging_files_dir = staging_asset_dir.join("files");
    fs::create_dir_all(&staging_files_dir)?;
    let result = (|| -> LibraryResult<ImportResult> {
        let file_id = Uuid::now_v7().to_string();
        let original_filename = format!("{title}.zip");
        let stored_filename = format!("{file_id}-{original_filename}");
        let staged_zip = staging_files_dir.join(&stored_filename);
        let byte_size = write_product_archive(
            &mut archive,
            &product_entries,
            &product_directory,
            &staged_zip,
        )?;
        fs::write(staging_asset_dir.join("cover.jpg"), thumbnail)?;

        let files = vec![ImportedFile {
            id: file_id,
            original_filename,
            stored_filename,
            byte_size,
        }];
        let asset_directory = locations.asset_storage_root.join("assets").join(&asset_id);
        copy_staged_asset(&staging_asset_dir, &asset_directory, &files)?;
        let info = String::from_utf8_lossy(&info);
        let (product_url, note) = parse_legacy_asset_info(&info);
        let database_result = insert_legacy_imported_asset(
            &database_path,
            &asset_id,
            &title,
            product_url.as_deref(),
            note.as_deref(),
            &files,
        );
        if let Err(error) = database_result {
            let _ = fs::remove_dir_all(&asset_directory);
            return Err(error);
        }
        Ok(ImportResult {
            asset_id,
            title,
            file_count: files.len(),
            total_bytes: byte_size,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_asset_dir);
    }
    result
}

fn archive_root_directory(name: &str) -> Option<String> {
    if name.starts_with('_') || !safe_archive_entry(name) {
        return None;
    }
    let mut components = Path::new(name).components();
    let Component::Normal(first) = components.next()? else {
        return None;
    };
    if components.next().is_none() && !name.ends_with('/') {
        return None;
    }
    first.to_str().map(ToOwned::to_owned)
}

fn safe_archive_entry(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn safe_archive_name(name: &str) -> LibraryResult<String> {
    safe_filename(Path::new(name))
}

fn read_zip_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> LibraryResult<Vec<u8>> {
    let mut entry = archive
        .by_name(name)
        .map_err(|error| LibraryError::Message(format!("ZIP内の {name} を読めません: {error}")))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn write_product_archive<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    entries: &[String],
    product_directory: &str,
    destination: &Path,
) -> LibraryResult<u64> {
    let output = File::create(destination)?;
    let mut writer = ZipWriter::new(BufWriter::new(output));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    if entries.is_empty() {
        writer
            .add_directory(format!("{product_directory}/"), options)
            .map_err(|error| LibraryError::Message(format!("ZIPを作成できません: {error}")))?;
    }
    for name in entries {
        let mut entry = archive.by_name(name).map_err(|error| {
            LibraryError::Message(format!("ZIP内の {name} を読めません: {error}"))
        })?;
        if entry.is_dir() {
            writer
                .add_directory(name, options)
                .map_err(|error| LibraryError::Message(format!("ZIPを作成できません: {error}")))?;
        } else {
            writer
                .start_file(name, options)
                .map_err(|error| LibraryError::Message(format!("ZIPを作成できません: {error}")))?;
            std::io::copy(&mut entry, &mut writer)?;
        }
    }
    let output = writer
        .finish()
        .map_err(|error| LibraryError::Message(format!("ZIPを作成できません: {error}")))?;
    output.get_ref().sync_all()?;
    Ok(fs::metadata(destination)?.len())
}

fn parse_legacy_asset_info(contents: &str) -> (Option<String>, Option<String>) {
    let mut lines = contents.lines();
    let first_line = lines.next().unwrap_or_default().trim();
    let product_url = if first_line.starts_with("https://") || first_line.starts_with("http://") {
        Some(first_line.to_owned())
    } else {
        None
    };
    let note = lines.collect::<Vec<_>>().join("\n");
    (product_url, (!note.trim().is_empty()).then_some(note))
}

fn copy_staged_asset(
    staging_asset_dir: &Path,
    asset_directory: &Path,
    files: &[ImportedFile],
) -> LibraryResult<()> {
    let destination_files = asset_directory.join("files");
    fs::create_dir_all(&destination_files)?;
    for file in files {
        fs::copy(
            staging_asset_dir.join("files").join(&file.stored_filename),
            destination_files.join(&file.stored_filename),
        )?;
    }
    let staged_cover = staging_asset_dir.join("cover.jpg");
    if staged_cover.is_file() {
        fs::copy(staged_cover, asset_directory.join("cover.jpg"))?;
    }
    fs::remove_dir_all(staging_asset_dir)?;
    Ok(())
}

struct ImportedFile {
    id: String,
    original_filename: String,
    stored_filename: String,
    byte_size: u64,
}

fn copy_file(source: &Path, destination: &Path) -> LibraryResult<u64> {
    let input = File::open(source)?;
    let output = File::create(destination)?;
    let mut reader = BufReader::new(input);
    let mut writer = BufWriter::new(output);
    let mut buffer = [0_u8; 1024 * 128];
    let mut byte_size = 0_u64;

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        writer.write_all(&buffer[..read])?;
        byte_size = byte_size.saturating_add(read as u64);
    }
    writer.flush()?;

    Ok(byte_size)
}

fn safe_filename(path: &Path) -> LibraryResult<String> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| LibraryError::Message("ファイル名を読み取れません".into()))?;

    let sanitized = filename
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        return Err(LibraryError::Message("使用できないファイル名です".into()));
    }
    Ok(sanitized)
}

fn insert_imported_asset(
    database_path: &Path,
    asset_id: &str,
    title: &str,
    files: &[ImportedFile],
) -> LibraryResult<()> {
    let mut connection = Connection::open(database_path)?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    let transaction = connection.transaction()?;
    let timestamp = unix_timestamp();
    transaction.execute(
        "INSERT INTO assets (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![asset_id, title, timestamp],
    )?;

    for file in files {
        let relative_path = format!("assets/{asset_id}/files/{}", file.stored_filename);
        transaction.execute(
            "
            INSERT INTO asset_files (
              id, asset_id, storage_mode, relative_path, external_path,
              original_filename, byte_size, sha256, created_at
            ) VALUES (?1, ?2, 'managed', ?3, NULL, ?4, ?5, ?6, ?7)
            ",
            params![
                file.id,
                asset_id,
                relative_path,
                file.original_filename,
                file.byte_size as i64,
                Option::<String>::None,
                timestamp,
            ],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn insert_legacy_imported_asset(
    database_path: &Path,
    asset_id: &str,
    title: &str,
    product_url: Option<&str>,
    note: Option<&str>,
    files: &[ImportedFile],
) -> LibraryResult<()> {
    insert_imported_asset(database_path, asset_id, title, files)?;
    let connection = Connection::open(database_path)?;
    connection.execute(
        "UPDATE assets
         SET source = 'legacy archive import', product_url = ?1, note = ?2,
             cover_path = ?3, updated_at = ?4
         WHERE id = ?5",
        params![
            product_url,
            note,
            format!("assets/{asset_id}/cover.jpg"),
            unix_timestamp(),
            asset_id,
        ],
    )?;
    Ok(())
}

fn assign_import_labels(
    database_path: &Path,
    asset_id: &str,
    tag_ids: &[String],
    category_ids: &[String],
) -> LibraryResult<()> {
    if tag_ids.is_empty() && category_ids.is_empty() {
        return Ok(());
    }
    let mut connection = Connection::open(database_path)?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    let transaction = connection.transaction()?;
    for tag_id in tag_ids {
        transaction.execute(
            "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
            params![asset_id, tag_id],
        )?;
    }
    for category_id in category_ids {
        transaction.execute(
            "INSERT OR IGNORE INTO asset_categories (asset_id, category_id) VALUES (?1, ?2)",
            params![asset_id, category_id],
        )?;
    }
    transaction.execute(
        "UPDATE assets SET updated_at = ?1 WHERE id = ?2",
        params![unix_timestamp(), asset_id],
    )?;
    transaction.commit()?;
    Ok(())
}

fn remove_imported_asset(
    database_path: &Path,
    asset_storage_root: &Path,
    asset_id: &str,
) -> LibraryResult<()> {
    let asset_directory = asset_directory(asset_storage_root, asset_id)?;
    let deletion_directory = asset_storage_root
        .join("assets")
        .join(format!(".deleting-{}", Uuid::now_v7()));
    if asset_directory.exists() {
        fs::rename(&asset_directory, &deletion_directory)?;
    }

    let result = (|| -> LibraryResult<()> {
        let connection = Connection::open(database_path)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        let deleted = connection.execute("DELETE FROM assets WHERE id = ?1", params![asset_id])?;
        if deleted == 0 {
            return Err(LibraryError::Message("アセットが見つかりません".into()));
        }
        Ok(())
    })();

    if let Err(error) = result {
        if deletion_directory.exists() {
            let _ = fs::rename(&deletion_directory, &asset_directory);
        }
        return Err(error);
    }
    if deletion_directory.exists() {
        fs::remove_dir_all(deletion_directory)?;
    }
    Ok(())
}

fn migrate(connection: &Connection) -> LibraryResult<()> {
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS app_metadata (
          key TEXT PRIMARY KEY NOT NULL,
          value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS assets (
          id TEXT PRIMARY KEY NOT NULL,
          title TEXT NOT NULL,
          creator TEXT,
          source TEXT,
          product_url TEXT,
          version TEXT,
          acquired_at INTEGER,
          note TEXT,
          cover_path TEXT,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS asset_files (
          id TEXT PRIMARY KEY NOT NULL,
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          storage_mode TEXT NOT NULL CHECK(storage_mode IN ('managed', 'external')),
          relative_path TEXT,
          external_path TEXT,
          original_filename TEXT NOT NULL,
          byte_size INTEGER NOT NULL,
          sha256 TEXT,
          created_at INTEGER NOT NULL,
          CHECK(
            (storage_mode = 'managed' AND relative_path IS NOT NULL AND external_path IS NULL) OR
            (storage_mode = 'external' AND external_path IS NOT NULL AND relative_path IS NULL)
          )
        );

        CREATE TABLE IF NOT EXISTS tags (
          id TEXT PRIMARY KEY NOT NULL,
          name TEXT NOT NULL COLLATE NOCASE UNIQUE,
          color TEXT
        );

        CREATE TABLE IF NOT EXISTS asset_tags (
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
          PRIMARY KEY (asset_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS categories (
          id TEXT PRIMARY KEY NOT NULL,
          parent_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
          name TEXT NOT NULL,
          sort_order INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS asset_categories (
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
          PRIMARY KEY (asset_id, category_id)
        );

        CREATE TABLE IF NOT EXISTS asset_relations (
          source_asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          target_asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          relation_type TEXT NOT NULL CHECK(relation_type IN ('addon_for', 'compatible_with', 'requires', 'variant_of', 'related_to')),
          note TEXT,
          created_at INTEGER NOT NULL,
          PRIMARY KEY (source_asset_id, target_asset_id, relation_type),
          CHECK(source_asset_id <> target_asset_id)
        );
        ",
    )?;

    let has_shop_url = connection
        .prepare("PRAGMA table_info(assets)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|column| column == "shop_url");
    if has_shop_url {
        connection.execute(
            "UPDATE assets SET product_url = COALESCE(NULLIF(product_url, ''), shop_url)",
            [],
        )?;
        connection.execute_batch("ALTER TABLE assets DROP COLUMN shop_url;")?;
    }
    let has_description = connection
        .prepare("PRAGMA table_info(assets)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|column| column == "description");
    if has_description {
        connection.execute(
            "UPDATE assets SET note = COALESCE(NULLIF(note, ''), description)",
            [],
        )?;
        connection.execute_batch("ALTER TABLE assets DROP COLUMN description;")?;
    }

    connection.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('schema_version', '2')",
        [],
    )?;
    Ok(())
}

fn count_assets(database_path: &Path) -> LibraryResult<i64> {
    let connection = Connection::open(database_path)?;
    Ok(connection.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))?)
}

#[allow(dead_code)]
fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::load(app.handle())
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_library_status,
            initialize_library,
            import_files,
            count_legacy_archives,
            import_legacy_directory,
            start_legacy_import,
            list_assets,
            get_asset_detail,
            update_asset,
            list_tags,
            create_tag,
            delete_tag,
            rename_tag,
            set_asset_tags,
            list_categories,
            create_category,
            delete_category,
            set_asset_categories,
            add_asset_relation,
            remove_asset_relation,
            set_asset_cover,
            delete_asset,
            add_asset_files,
            delete_asset_file,
            get_asset_file_path,
            reveal_asset_file,
            open_external_url,
            get_capacity_report,
            create_database_backup,
            list_database_backups,
            restore_database_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running VR Asset Library");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_library_schema() {
        let root = std::env::temp_dir().join(format!("vr-asset-library-test-{}", unix_timestamp()));
        prepare_library(&root, &root.join("asset-storage"), &root.join("db-backups")).unwrap();

        let database_path = root.join(DATABASE_FILE);
        assert!(database_path.is_file());
        assert_eq!(count_assets(&database_path).unwrap(), 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_an_empty_storage_path() {
        assert!(validate_storage_path("  ", "アセット保存先").is_err());
    }

    #[test]
    fn refuses_an_invalid_asset_id_for_storage_paths() {
        assert!(asset_directory(Path::new("/tmp/assets"), "../outside").is_err());
    }

    #[test]
    fn migrates_shop_url_into_the_single_product_url() {
        let database =
            std::env::temp_dir().join(format!("vr-asset-library-url-{}.db", Uuid::now_v7()));
        let connection = Connection::open(&database).unwrap();
        connection.execute_batch("CREATE TABLE assets (id TEXT PRIMARY KEY NOT NULL, title TEXT NOT NULL, creator TEXT, source TEXT, product_url TEXT, shop_url TEXT, description TEXT, version TEXT, acquired_at INTEGER, note TEXT, cover_path TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL); INSERT INTO assets (id, title, shop_url, created_at, updated_at) VALUES ('asset-1', 'Legacy', 'https://booth.pm/ja/items/1', 0, 0);").unwrap();
        migrate(&connection).unwrap();
        let url: Option<String> = connection
            .query_row(
                "SELECT product_url FROM assets WHERE id='asset-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(url.as_deref(), Some("https://booth.pm/ja/items/1"));
        let columns = connection
            .prepare("PRAGMA table_info(assets)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(!columns.iter().any(|column| column == "shop_url"));
        fs::remove_file(database).unwrap();
    }

    #[test]
    fn imports_multiple_files_into_an_asset_directory() {
        let root = std::env::temp_dir().join(format!("vr-asset-library-import-{}", Uuid::now_v7()));
        let source = root.join("source");
        let library = root.join("library");
        let asset_storage = root.join("asset-storage");
        let database_backups = root.join("db-backups");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("main.zip"), b"main asset").unwrap();
        fs::write(source.join("readme.txt"), b"instructions").unwrap();
        prepare_library(&library, &asset_storage, &database_backups).unwrap();

        let result = import_into_library(
            &library,
            &asset_storage,
            ImportRequest {
                title: "Test asset".into(),
                source_paths: vec![
                    display_path(&source.join("main.zip")),
                    display_path(&source.join("readme.txt")),
                ],
                creator: None,
                source: None,
                product_url: None,
                version: None,
                note: None,
                tag_ids: vec![],
                category_ids: vec![],
            },
        )
        .unwrap();

        assert_eq!(result.file_count, 2);
        assert_eq!(count_assets(&library.join(DATABASE_FILE)).unwrap(), 1);
        assert_eq!(
            fs::read_dir(
                asset_storage
                    .join("assets")
                    .join(result.asset_id)
                    .join("files")
            )
            .unwrap()
            .count(),
            2
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imports_a_legacy_archive_and_preserves_product_directory() {
        let root = std::env::temp_dir().join(format!("vr-asset-library-legacy-{}", Uuid::now_v7()));
        let library = root.join("library");
        let asset_storage = root.join("asset-storage");
        let backups = root.join("backups");
        let source_dir = root.join("source");
        let source_zip = source_dir.join("legacy.zip");
        fs::create_dir_all(&source_dir).unwrap();
        prepare_library(&library, &asset_storage, &backups).unwrap();
        {
            let file = File::create(&source_zip).unwrap();
            let mut writer = ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            writer.start_file("_assetinfo.md", options).unwrap();
            writer
                .write_all("https://booth.pm/ja/items/123".as_bytes())
                .unwrap();
            writer.start_file("_thumbnail.jpg", options).unwrap();
            writer.write_all(b"not-a-real-jpeg").unwrap();
            writer.start_file("Product/readme.txt", options).unwrap();
            writer.write_all(b"asset data").unwrap();
            writer.finish().unwrap();
        }
        let locations = StorageLocations {
            library_root: library.clone(),
            asset_storage_root: asset_storage.clone(),
            database_backup_root: backups,
        };
        let result = import_legacy_archive(&locations, &source_zip).unwrap();
        assert_eq!(result.title, "Product");
        assert_eq!(count_assets(&library.join(DATABASE_FILE)).unwrap(), 1);
        assert!(asset_storage
            .join("assets")
            .join(&result.asset_id)
            .join("cover.jpg")
            .is_file());
        let detail = Connection::open(library.join(DATABASE_FILE))
            .unwrap()
            .query_row(
                "SELECT product_url, note FROM assets WHERE id = ?1",
                params![result.asset_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .unwrap();
        assert_eq!(detail.0, "https://booth.pm/ja/items/123");
        assert_eq!(detail.1, None);
        let stored = fs::read_dir(
            asset_storage
                .join("assets")
                .join(result.asset_id)
                .join("files"),
        )
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
        let archive = ZipArchive::new(File::open(stored).unwrap()).unwrap();
        assert!(archive
            .file_names()
            .any(|name| name == "Product/readme.txt"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeps_legacy_asset_info_lines_after_the_url_as_a_note() {
        assert_eq!(
            parse_legacy_asset_info("https://booth.pm/ja/items/123\n対応アバター: Example\n"),
            (
                Some("https://booth.pm/ja/items/123".into()),
                Some("対応アバター: Example".into())
            )
        );
    }

    #[test]
    fn database_snapshot_contains_imported_assets() {
        let root = std::env::temp_dir().join(format!("vr-asset-library-backup-{}", Uuid::now_v7()));
        let library = root.join("library");
        let asset_storage = root.join("asset-storage");
        let database_backups = root.join("db-backups");
        let source = root.join("source.zip");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"asset").unwrap();
        prepare_library(&library, &asset_storage, &database_backups).unwrap();
        import_into_library(
            &library,
            &asset_storage,
            ImportRequest {
                title: "Backup test".into(),
                source_paths: vec![display_path(&source)],
                creator: None,
                source: None,
                product_url: None,
                version: None,
                note: None,
                tag_ids: vec![],
                category_ids: vec![],
            },
        )
        .unwrap();

        let snapshot = database_backups.join("snapshot.db");
        copy_database_snapshot(&library.join(DATABASE_FILE), &snapshot).unwrap();
        assert_eq!(count_assets(&snapshot).unwrap(), 1);

        fs::remove_dir_all(root).unwrap();
    }
}
