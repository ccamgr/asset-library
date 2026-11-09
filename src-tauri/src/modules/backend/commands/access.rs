#[tauri::command]
pub(crate) fn set_asset_cover(
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
    let relative_path = asset_relative_path(&asset_id, &format!("cover.{extension}"))
        .map_err(|error| error.to_string())?;
    let destination = locations.asset_storage_root.join(&relative_path);
    let old_path: Option<String> = Connection::open(&locations.database_path)
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
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
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
    remove_cached_thumbnail(&locations.library_root, &asset_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn get_asset_file_path(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<String, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
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
    if !path.is_file() {
        return Err(format!(
            "保存ファイルが見つかりません: {}",
            display_path(&path)
        ));
    }
    Ok(display_path(&path))
}

#[tauri::command]
pub(crate) fn reveal_asset_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), String> {
    let log_path = state.log_path.clone();
    state.log(format!("file reveal requested file_id={file_id}"));
    let path = get_asset_file_path(app, state, file_id)?;
    match reveal_in_file_manager(Path::new(&path)) {
        Ok(()) => {
            append_log(&log_path, &format!("file reveal started path={path:?}"));
            Ok(())
        }
        Err(error) => {
            append_log(
                &log_path,
                &format!("file reveal failed path={path:?} reason={error}"),
            );
            Err(error.to_string())
        }
    }
}

#[tauri::command]
pub(crate) fn reveal_asset_directory(
    app: AppHandle,
    state: State<'_, AppState>,
    asset_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let directory = asset_files_directory(&locations.asset_storage_root, &asset_id)
        .map_err(|error| error.to_string())?;
    if !directory.is_dir() {
        return Err("アセットの保存先が見つかりません".into());
    }
    state.log(format!(
        "asset directory reveal started asset_id={asset_id} path={directory:?}"
    ));
    open_directory_in_file_manager(&directory).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn open_asset_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), String> {
    let log_path = state.log_path.clone();
    state.log(format!("file open requested file_id={file_id}"));
    let path = get_asset_file_path(app, state, file_id)?;
    match open_with_default_application(&path) {
        Ok(()) => {
            append_log(&log_path, &format!("file open started path={path:?}"));
            Ok(())
        }
        Err(error) => {
            append_log(
                &log_path,
                &format!("file open failed path={path:?} reason={error}"),
            );
            Err(error.to_string())
        }
    }
}

#[tauri::command]
pub(crate) fn open_external_url(url: String) -> Result<(), String> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("HTTP(S) のURLだけを開けます".into());
    }
    open_with_default_application(url).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_capacity_report(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CapacityReport, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    Ok(storage_capacity_report(&locations))
}

use super::super::*;
