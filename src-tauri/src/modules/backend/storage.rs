use super::*;

pub(super) fn directory_size(path: &Path) -> u64 {
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

pub(super) fn database_size(path: &Path) -> u64 {
    let mut paths = vec![path.to_path_buf()];
    if let Some(filename) = path.file_name().and_then(|value| value.to_str()) {
        paths.push(path.with_file_name(format!("{filename}-wal")));
        paths.push(path.with_file_name(format!("{filename}-shm")));
    }
    paths
        .into_iter()
        .filter_map(|path| fs::metadata(path).ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

pub(super) fn storage_capacity_report(locations: &StorageLocations) -> CapacityReport {
    CapacityReport {
        library_bytes: directory_size(&locations.library_root),
        asset_bytes: directory_size(&locations.asset_storage_root.join("assets")),
        database_bytes: database_size(&locations.database_path),
        thumbnail_cache_bytes: directory_size(&locations.library_root.join("thumbnail-cache")),
        database_backup_bytes: directory_size(&locations.database_backup_root),
    }
}

pub(super) fn copy_database_snapshot(
    source_path: &Path,
    destination_path: &Path,
) -> LibraryResult<()> {
    let source = Connection::open(source_path)?;
    let mut destination = Connection::open(destination_path)?;
    let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
    backup.run_to_completion(32, Duration::from_millis(10), None)?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) fn reveal_in_file_manager(path: &Path) -> LibraryResult<()> {
    let folder = path
        .parent()
        .ok_or_else(|| LibraryError::Message("ファイルの親フォルダを取得できません".into()))?;
    let folder = PathBuf::from(display_path(folder).replace('/', "\\"));
    Command::new("explorer.exe").arg(folder).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub(super) fn reveal_in_file_manager(path: &Path) -> LibraryResult<()> {
    Command::new("open").arg("-R").arg(path).spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(super) fn reveal_in_file_manager(path: &Path) -> LibraryResult<()> {
    Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn()?;
    Ok(())
}

pub(super) fn open_with_default_application(target: &str) -> LibraryResult<()> {
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

pub(super) fn validate_storage_path(input: &str, label: &str) -> LibraryResult<PathBuf> {
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

pub(super) fn validated_asset_id(asset_id: &str) -> LibraryResult<&str> {
    Uuid::parse_str(asset_id).map_err(|_| LibraryError::Message("アセットIDが不正です".into()))?;
    Ok(asset_id)
}

pub(super) fn asset_directory(asset_storage_root: &Path, asset_id: &str) -> LibraryResult<PathBuf> {
    Ok(asset_storage_root.join(asset_relative_directory(asset_id)?))
}

pub(super) fn asset_files_directory(
    asset_storage_root: &Path,
    asset_id: &str,
) -> LibraryResult<PathBuf> {
    Ok(asset_directory(asset_storage_root, asset_id)?.join("files"))
}

pub(super) fn asset_relative_directory(asset_id: &str) -> LibraryResult<String> {
    let asset_id = validated_asset_id(asset_id)?;
    let shard = asset_id
        .get(asset_id.len().saturating_sub(2)..)
        .ok_or_else(|| LibraryError::Message("アセットIDが不正です".into()))?;
    Ok(format!("assets/{shard}/{asset_id}"))
}

pub(super) fn asset_relative_path(asset_id: &str, filename: &str) -> LibraryResult<String> {
    Ok(format!(
        "{}/{}",
        asset_relative_directory(asset_id)?,
        filename
    ))
}

pub(super) fn cleanup_added_files(destination: &Path, files: &[ImportedFile]) {
    for file in files {
        let _ = fs::remove_file(destination.join(&file.stored_filename));
    }
}

pub(super) fn prepare_library(
    root: &Path,
    database_path: &Path,
    asset_storage_root: &Path,
    database_backup_root: &Path,
) -> LibraryResult<()> {
    for directory in ["thumbnail-cache", "exports"] {
        fs::create_dir_all(root.join(directory))?;
    }
    fs::create_dir_all(asset_storage_root.join("assets"))?;
    fs::create_dir_all(database_backup_root)?;
    let database_parent = database_path
        .parent()
        .ok_or_else(|| LibraryError::Message("DBファイルの保存先が不正です".into()))?;
    fs::create_dir_all(database_parent)?;
    let mut connection = Connection::open(database_path)?;
    migrate(&mut connection)?;
    Ok(())
}

pub(super) fn validate_database_path(value: &str) -> LibraryResult<PathBuf> {
    let path = PathBuf::from(value.trim());
    if !path.is_absolute() {
        return Err(LibraryError::Message(
            "DBファイルには絶対パスを指定してください".into(),
        ));
    }
    if path.file_name().and_then(|name| name.to_str()) != Some(DATABASE_FILE) {
        return Err(LibraryError::Message(format!(
            "DBファイル名は {DATABASE_FILE} を指定してください"
        )));
    }
    Ok(path)
}
