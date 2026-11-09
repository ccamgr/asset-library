#[tauri::command]
pub(crate) fn delete_asset(
    app: AppHandle,
    state: State<'_, AppState>,
    asset_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    remove_imported_asset(
        &locations.database_path,
        &locations.asset_storage_root,
        &asset_id,
    )
    .map_err(|error| error.to_string())?;
    remove_cached_thumbnail(&locations.library_root, &asset_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn add_asset_files(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetFilesRequest,
) -> Result<(), String> {
    if request.source_paths.is_empty() {
        return Err("少なくとも1つのファイルを選んでください".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let database_path = locations.database_path.clone();
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
        let stored_filename = available_filename(&destination, &original_filename, None);
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
            transaction.execute("INSERT INTO asset_files (id, asset_id, storage_mode, relative_path, external_path, original_filename, byte_size, sha256, created_at) VALUES (?1, ?2, 'managed', ?3, NULL, ?4, ?5, NULL, ?6)", params![file.id, request.asset_id, asset_relative_path(&request.asset_id, &format!("files/{}", file.stored_filename))?, file.original_filename, file.byte_size as i64, timestamp])?;
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
pub(crate) fn delete_asset_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let database_path = locations.database_path.clone();
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

use super::super::*;
