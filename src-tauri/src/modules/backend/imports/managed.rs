pub(crate) fn import_into_library(
    database_path: &Path,
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
            let stored_filename = available_filename(&staging_files_dir, &original_filename, None);
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

        let asset_directory = asset_directory(asset_storage_root, &asset_id)?;
        copy_staged_asset(&staging_asset_dir, &asset_directory, &files, None)?;

        let database_result =
            insert_imported_asset(database_path, &asset_id, title, &files, Some(&request));
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

use super::*;
