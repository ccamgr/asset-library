#[tauri::command]
pub(crate) fn get_library_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LibraryStatus, String> {
    let default_library_path = default_library_path(&app).map_err(|error| error.to_string())?;
    let default_asset_storage_path = default_library_path.join("asset-storage");
    let default_database_backup_path = default_library_path.join("database-backups");
    let settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?
        .clone();
    let database_path = settings
        .database_path
        .clone()
        .unwrap_or_else(|| default_library_path.join(DATABASE_FILE));
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
        default_database_path: display_path(&default_library_path.join(DATABASE_FILE)),
        selected_library_path: Some(display_path(&default_library_path)),
        asset_storage_path: Some(display_path(&asset_storage_path)),
        database_backup_path: Some(display_path(&database_backup_path)),
        database_path: Some(display_path(&database_path)),
        initialized,
        asset_count,
    })
}

#[tauri::command]
pub(crate) fn initialize_library(
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
    let database_path = request
        .database_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(validate_database_path)
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| library_path.join(DATABASE_FILE));
    if database_path.is_file() {
        let report = asset_storage_integrity_report(&database_path, &asset_storage_path)
            .map_err(|error| error.to_string())?;
        if !report.missing_asset_ids.is_empty() {
            return Err(format!(
                "指定したアセット保存先には、DBに登録された{}件のアセット保存フォルダがありません。保存先とDBの組み合わせを確認してください",
                report.missing_asset_ids.len()
            ));
        }
    }
    prepare_library(
        &library_path,
        &database_path,
        &asset_storage_path,
        &database_backup_path,
    )
    .map_err(|error| error.to_string())?;
    allow_cover_paths(&app, &library_path, &asset_storage_path)
        .map_err(|error| error.to_string())?;

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?;
    settings.asset_storage_path = Some(asset_storage_path.clone());
    settings.database_backup_path = Some(database_backup_path.clone());
    settings.database_path = Some(database_path.clone());
    state
        .save_settings(&settings)
        .map_err(|error| error.to_string())?;
    state.log(format!(
        "library initialized database={} asset_storage={} database_backups={}",
        display_path(&database_path),
        display_path(&asset_storage_path),
        display_path(&database_backup_path)
    ));
    Ok(())
}

#[tauri::command]
pub(crate) fn import_files(
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
        .unwrap_or_else(|| library_path.join("asset-storage"));
    let database_path = settings
        .database_path
        .unwrap_or_else(|| library_path.join(DATABASE_FILE));

    let tag_ids = request.tag_ids.clone();
    let category_ids = request.category_ids.clone();
    let title = request.title.clone();
    let file_count = request.source_paths.len();
    state.log(format!(
        "asset import started title={title:?} files={file_count}"
    ));
    let result = import_into_library(&database_path, &asset_storage_path, request)
        .map_err(|error| error.to_string())?;
    if let Err(error) =
        assign_import_labels(&database_path, &result.asset_id, &tag_ids, &category_ids)
    {
        let _ = remove_imported_asset(&database_path, &asset_storage_path, &result.asset_id);
        state.log(format!(
            "asset import failed asset_id={} reason={error}",
            result.asset_id
        ));
        return Err(error.to_string());
    }
    state.log(format!(
        "asset import completed asset_id={} files={file_count}",
        result.asset_id
    ));
    Ok(result)
}

#[tauri::command]
pub(crate) fn count_legacy_archives(directory_path: String) -> Result<usize, String> {
    legacy_archives(Path::new(directory_path.trim()))
        .map(|archives| archives.len())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn start_legacy_import(
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
    let log_path = state.log_path.clone();
    append_log(
        &log_path,
        &format!(
            "legacy ZIP import started directories={} tags={} categories={}",
            request.directory_paths.len(),
            request.tag_ids.len(),
            request.category_ids.len()
        ),
    );

    std::thread::spawn(move || {
        let result = import_legacy_batch(&app, &locations, request, &log_path);
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
        append_log(
            &log_path,
            &format!(
                "legacy ZIP import {}",
                if complete.error.is_some() {
                    "failed"
                } else {
                    "completed"
                }
            ),
        );
        running.store(false, Ordering::Release);
        let _ = app.emit("legacy-import-complete", complete);
    });
    Ok(())
}

pub(crate) fn import_legacy_batch(
    app: &AppHandle,
    locations: &StorageLocations,
    request: LegacyBatchImportRequest,
    log_path: &Path,
) -> LibraryResult<LegacyDirectoryImportResult> {
    let mut archives = Vec::new();
    for directory_path in request.directory_paths {
        archives.extend(legacy_archives(Path::new(directory_path.trim()))?);
    }
    let total = archives.len();
    append_log(log_path, &format!("legacy ZIP import archives={total}"));
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
                    &locations.database_path,
                    &result.asset_id,
                    &request.tag_ids,
                    &request.category_ids,
                ) {
                    let _ = remove_imported_asset(
                        &locations.database_path,
                        &locations.asset_storage_root,
                        &result.asset_id,
                    );
                    failures.push(LegacyImportFailure {
                        source_filename: source_filename.clone(),
                        reason: format!("ラベルの設定に失敗しました: {error}"),
                    });
                    append_log(
                        log_path,
                        &format!(
                            "legacy ZIP import failed archive={source_filename:?} reason={error}"
                        ),
                    );
                } else {
                    append_log(
                        log_path,
                        &format!(
                            "legacy ZIP import completed archive={source_filename:?} asset_id={}",
                            result.asset_id
                        ),
                    );
                    imported.push(result);
                }
            }
            Err(error) => {
                append_log(
                    log_path,
                    &format!("legacy ZIP import failed archive={source_filename:?} reason={error}"),
                );
                failures.push(LegacyImportFailure {
                    source_filename,
                    reason: error.to_string(),
                });
            }
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

pub(crate) fn legacy_archives(directory: &Path) -> LibraryResult<Vec<PathBuf>> {
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

use super::super::*;
