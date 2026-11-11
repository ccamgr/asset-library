fn previewable_image_path(source_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(source_path);
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|value| ["png", "jpg", "jpeg", "webp", "gif"].contains(&value.as_str()))
        .ok_or_else(|| "PNG、JPEG、WebP、GIF画像を選んでください".to_string())?;
    path.canonicalize()
        .map_err(|error| format!("画像ファイルを確認できません: {error}"))
}

#[tauri::command]
pub(crate) fn allow_cover_preview_file(
    app: AppHandle,
    source_path: String,
) -> Result<String, String> {
    let path = previewable_image_path(&source_path)?;
    if !path.is_file() {
        return Err("画像ファイルを選んでください".into());
    }
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|error| format!("画像プレビューを許可できません: {error}"))?;
    Ok(display_path(&path))
}

#[tauri::command]
pub(crate) fn revoke_cover_preview_file(app: AppHandle, source_path: String) -> Result<(), String> {
    app.asset_protocol_scope()
        .forbid_file(PathBuf::from(source_path))
        .map_err(|error| format!("画像プレビューの許可を解除できません: {error}"))
}

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
pub(crate) fn set_asset_cover_from_url(
    app: AppHandle,
    state: State<'_, AppState>,
    asset_id: String,
    image_url: String,
) -> Result<(), String> {
    if !image_url.starts_with("https://booth.pximg.net/") {
        return Err("BOOTHのカバー画像URLが不正です".into());
    }
    state.log(format!(
        "BOOTH cover download requested asset_id={asset_id}"
    ));
    const MAX_COVER_BYTES: u64 = 20 * 1024 * 1024;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("HTTPクライアントを初期化できません: {error}"))?;
    let response = client
        .get(&image_url)
        .send()
        .map_err(|error| format!("BOOTHのカバー画像を取得できません: {error}"))?
        .error_for_status()
        .map_err(|error| format!("BOOTHのカバー画像を取得できません: {error}"))?;
    if response
        .content_length()
        .is_some_and(|size| size > MAX_COVER_BYTES)
    {
        return Err("BOOTHのカバー画像が大きすぎます（上限20 MiB）".into());
    }
    let extension = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .and_then(|value| match value {
            "image/jpeg" => Some("jpg"),
            "image/png" => Some("png"),
            "image/webp" => Some("webp"),
            "image/gif" => Some("gif"),
            _ => None,
        })
        .ok_or_else(|| "BOOTHのカバー画像形式に対応していません".to_string())?;
    let mut bytes = Vec::new();
    response
        .take(MAX_COVER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("BOOTHのカバー画像を読み取れません: {error}"))?;
    if bytes.len() as u64 > MAX_COVER_BYTES {
        return Err("BOOTHのカバー画像が大きすぎます（上限20 MiB）".into());
    }
    let temporary_path = std::env::temp_dir().join(format!(
        "asset-library-cover-{}.{}",
        Uuid::now_v7(),
        extension
    ));
    fs::write(&temporary_path, bytes).map_err(|error| error.to_string())?;
    let result = set_asset_cover(app, state, asset_id, display_path(&temporary_path));
    let _ = fs::remove_file(temporary_path);
    result
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
    let log_path = state.log_path.clone();
    state.log(format!(
        "asset directory reveal requested asset_id={asset_id}"
    ));
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let file_id: String = connection
        .query_row(
            "SELECT id FROM asset_files WHERE asset_id=?1 ORDER BY created_at, id LIMIT 1",
            params![&asset_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let path = get_asset_file_path(app, state, file_id)?;
    let directory = Path::new(&path)
        .parent()
        .ok_or_else(|| "ファイルの親フォルダを取得できません".to_string())?;
    append_log(
        &log_path,
        &format!("asset directory reveal started asset_id={asset_id} path={directory:?}"),
    );
    open_directory_in_file_manager(directory).map_err(|error| error.to_string())
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
pub(crate) fn open_external_url(state: State<'_, AppState>, url: String) -> Result<(), String> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("HTTP(S) のURLだけを開けます".into());
    }
    let log_path = state.log_path.clone();
    state.log(format!("external URL open requested url={url:?}"));
    match open_with_default_application(url) {
        Ok(()) => {
            append_log(&log_path, &format!("external URL open started url={url:?}"));
            Ok(())
        }
        Err(error) => {
            append_log(
                &log_path,
                &format!("external URL open failed url={url:?} reason={error}"),
            );
            Err(error.to_string())
        }
    }
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
