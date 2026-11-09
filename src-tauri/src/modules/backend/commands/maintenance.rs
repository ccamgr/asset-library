#[tauri::command]
pub(crate) fn rebuild_thumbnail_cache(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ThumbnailCacheRebuildResult, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let cache_root = locations.library_root.join("thumbnail-cache");
    if cache_root.exists() {
        fs::remove_dir_all(&cache_root).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&cache_root).map_err(|error| error.to_string())?;

    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare("SELECT id, cover_path FROM assets WHERE cover_path IS NOT NULL")
        .map_err(|error| error.to_string())?;
    let covers = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let mut generated = 0;
    let mut skipped = 0;
    for (asset_id, cover_path) in covers {
        match generate_cached_thumbnail(
            &locations.library_root,
            &locations.asset_storage_root,
            &asset_id,
            &cover_path,
        ) {
            Ok(()) => generated += 1,
            Err(_) => skipped += 1,
        }
    }
    Ok(ThumbnailCacheRebuildResult { generated, skipped })
}

#[tauri::command]
pub(crate) fn create_database_backup(
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
    copy_database_snapshot(&locations.database_path, &temporary_path)
        .map_err(|error| error.to_string())?;
    fs::rename(&temporary_path, &backup_path).map_err(|error| error.to_string())?;
    Ok(BackupResult {
        path: display_path(&backup_path),
        created_at,
    })
}

#[tauri::command]
pub(crate) fn list_database_backups(
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
pub(crate) fn restore_database_backup(
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
    copy_database_snapshot(&backup_file, &locations.database_path)
        .map_err(|error| error.to_string())?;
    Ok(())
}

use super::super::*;
