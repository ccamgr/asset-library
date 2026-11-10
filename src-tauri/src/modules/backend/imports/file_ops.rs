pub(crate) struct ImportedFile {
    pub(crate) id: String,
    pub(crate) original_filename: String,
    pub(crate) stored_filename: String,
    pub(crate) byte_size: u64,
}

pub(crate) fn copy_file(source: &Path, destination: &Path) -> LibraryResult<u64> {
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

pub(crate) fn safe_filename(path: &Path) -> LibraryResult<String> {
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

pub(crate) fn available_filename(
    directory: &Path,
    filename: &str,
    current_filename: Option<&str>,
) -> String {
    if current_filename == Some(filename) || !directory.join(filename).exists() {
        return filename.to_owned();
    }

    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    for suffix in 2.. {
        let candidate = format!("{stem} ({suffix}){extension}");
        if current_filename == Some(candidate.as_str()) || !directory.join(&candidate).exists() {
            return candidate;
        }
    }
    unreachable!("unbounded filename suffix search must find an available name")
}

pub(crate) fn insert_imported_asset(
    database_path: &Path,
    asset_id: &str,
    title: &str,
    files: &[ImportedFile],
    request: Option<&ImportRequest>,
) -> LibraryResult<()> {
    let mut connection = Connection::open(database_path)?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    let transaction = connection.transaction()?;
    let timestamp = unix_timestamp();
    transaction.execute(
        "INSERT INTO assets (id, title, creator, product_url, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            asset_id,
            title,
            request.and_then(|value| value.creator.as_deref()),
            request.and_then(|value| value.product_url.as_deref()),
            request.and_then(|value| value.note.as_deref()),
            timestamp,
        ],
    )?;

    for file in files {
        let relative_path =
            asset_relative_path(asset_id, &format!("files/{}", file.stored_filename))?;
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

pub(crate) fn insert_legacy_imported_asset(
    database_path: &Path,
    asset_id: &str,
    title: &str,
    product_url: Option<&str>,
    note: Option<&str>,
    cover_filename: &str,
    files: &[ImportedFile],
) -> LibraryResult<()> {
    insert_imported_asset(database_path, asset_id, title, files, None)?;
    let connection = Connection::open(database_path)?;
    connection.execute(
        "UPDATE assets
         SET product_url = ?1, note = ?2,
             cover_path = ?3, updated_at = ?4
         WHERE id = ?5",
        params![
            product_url,
            note,
            asset_relative_path(asset_id, cover_filename)?,
            unix_timestamp(),
            asset_id,
        ],
    )?;
    Ok(())
}

pub(crate) fn assign_import_labels(
    database_path: &Path,
    asset_id: &str,
    tag_ids: &[String],
    category_ids: &[String],
) -> LibraryResult<()> {
    if category_ids.len() > 1 {
        return Err(LibraryError::Message(
            "カテゴリはアセットごとに1つだけ設定できます".into(),
        ));
    }
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

pub(crate) fn remove_imported_asset(
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

use super::*;
