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
        asset_bytes: directory_size(&locations.asset_storage_root),
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

#[cfg(target_os = "windows")]
pub(super) fn open_with_default_application(target: &str) -> LibraryResult<()> {
    use std::{ffi::OsStr, iter::once, os::windows::ffi::OsStrExt};
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };

    let target = OsStr::new(target)
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            None,
            None,
            PCWSTR(target.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        return Err(LibraryError::Message(shell_execute_error_message(code)));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn shell_execute_error_message(code: isize) -> String {
    let reason = match code {
        0 => "Windowsのリソースが不足しています",
        2 => "ファイルが見つかりません",
        3 => "パスが見つかりません",
        5 => "ファイルへのアクセスが拒否されました",
        11 => "実行形式が無効です",
        26 => "ファイルの関連付けが不完全です",
        27 => "関連付けアプリとの通信がビジー状態です",
        28 => "関連付けアプリとの通信に失敗しました",
        29 => "関連付けアプリとの通信がタイムアウトしました",
        30 => "関連付けアプリに必要なDLLが見つかりません",
        31 => "この種類のファイルを開くアプリが関連付けられていません",
        32 => "Windowsのメモリが不足しています",
        33 => "ファイルが他のアプリで使用中です",
        _ => return format!("Windows Shellで開けませんでした (エラーコード: {code})"),
    };
    format!("Windows Shellで開けませんでした: {reason} (エラーコード: {code})")
}

#[cfg(not(target_os = "windows"))]
pub(super) fn open_with_default_application(target: &str) -> LibraryResult<()> {
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

pub(super) fn open_directory_in_file_manager(directory: &Path) -> LibraryResult<()> {
    if !directory.is_dir() {
        return Err(LibraryError::Message("フォルダが見つかりません".into()));
    }
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer.exe");
        command.arg(PathBuf::from(display_path(directory).replace('/', "\\")));
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(directory);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(directory);
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

pub(super) fn asset_relative_directory(asset_id: &str) -> LibraryResult<String> {
    let asset_id = validated_asset_id(asset_id)?;
    let shard = asset_id
        .get(asset_id.len().saturating_sub(2)..)
        .ok_or_else(|| LibraryError::Message("アセットIDが不正です".into()))?;
    Ok(format!("{shard}/{asset_id}"))
}

pub(super) fn asset_relative_path(asset_id: &str, filename: &str) -> LibraryResult<String> {
    Ok(format!(
        "{}/{}",
        asset_relative_directory(asset_id)?,
        filename
    ))
}

pub(super) fn existing_asset_directory(
    asset_storage_root: &Path,
    asset_id: &str,
) -> LibraryResult<Option<PathBuf>> {
    let current = asset_directory(asset_storage_root, asset_id)?;
    if current.is_dir() {
        return Ok(Some(current));
    }

    let legacy = asset_storage_root
        .join("assets")
        .join(asset_relative_directory(asset_id)?);
    Ok(legacy.is_dir().then_some(legacy))
}

pub(super) fn asset_relative_path_in_directory(
    asset_storage_root: &Path,
    asset_directory: &Path,
    filename: &str,
) -> LibraryResult<String> {
    let path = asset_directory.join(filename);
    let relative = path
        .strip_prefix(asset_storage_root)
        .map_err(|_| LibraryError::Message("アセット保存先が不正です".into()))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

pub(super) fn asset_storage_integrity_report(
    database_path: &Path,
    asset_storage_root: &Path,
) -> LibraryResult<AssetStorageIntegrityReport> {
    let connection = Connection::open(database_path)?;
    let asset_ids = connection
        .prepare("SELECT id FROM assets ORDER BY id")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let missing_asset_ids = asset_ids
        .iter()
        .filter_map(
            |asset_id| match existing_asset_directory(asset_storage_root, asset_id) {
                Ok(Some(_)) => None,
                Ok(None) | Err(_) => Some(asset_id.clone()),
            },
        )
        .collect();

    Ok(AssetStorageIntegrityReport {
        total: asset_ids.len(),
        missing_asset_ids,
    })
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
    fs::create_dir_all(asset_storage_root)?;
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
