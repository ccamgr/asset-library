use super::*;

type AssetDetailRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(super) fn default_library_path(app: &AppHandle) -> LibraryResult<PathBuf> {
    let path = app
        .path()
        .app_local_data_dir()
        .map_err(|error| LibraryError::Message(format!("既定の保存先を取得できません: {error}")))?;
    Ok(path.join("library"))
}

pub(super) fn storage_locations(
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
        database_path: settings
            .database_path
            .unwrap_or_else(|| library_root.join(DATABASE_FILE)),
        library_root,
    })
}

pub(super) fn allow_cover_paths(
    app: &AppHandle,
    library_root: &Path,
    asset_storage_root: &Path,
) -> LibraryResult<()> {
    app.asset_protocol_scope()
        .allow_directory(asset_storage_root, true)
        .map_err(|error| {
            LibraryError::Message(format!(
                "アセット保存先を画像表示用に許可できません: {error}"
            ))
        })?;
    app.asset_protocol_scope()
        .allow_directory(library_root.join("thumbnail-cache"), true)
        .map_err(|error| {
            LibraryError::Message(format!(
                "サムネイルキャッシュを画像表示用に許可できません: {error}"
            ))
        })
}

pub(super) fn all_tags(connection: &Connection) -> LibraryResult<Vec<Tag>> {
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

pub(super) fn tags_for_asset(connection: &Connection, asset_id: &str) -> LibraryResult<Vec<Tag>> {
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

pub(super) fn tags_for_assets(
    connection: &Connection,
    asset_ids: &[&str],
) -> LibraryResult<HashMap<String, Vec<Tag>>> {
    if asset_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = std::iter::repeat_n("?", asset_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT at.asset_id, t.id, t.name, t.color FROM asset_tags at JOIN tags t ON t.id=at.tag_id WHERE at.asset_id IN ({placeholders}) ORDER BY t.name COLLATE NOCASE"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(asset_ids.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            Tag {
                id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
            },
        ))
    })?;
    let mut grouped = HashMap::<String, Vec<Tag>>::new();
    for row in rows {
        let (asset_id, tag) = row?;
        grouped.entry(asset_id).or_default().push(tag);
    }
    Ok(grouped)
}

pub(super) fn all_categories(connection: &Connection) -> LibraryResult<Vec<Category>> {
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

pub(super) fn categories_for_assets(
    connection: &Connection,
    asset_ids: &[&str],
) -> LibraryResult<HashMap<String, Vec<Category>>> {
    if asset_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = std::iter::repeat_n("?", asset_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT ac.asset_id, c.id, c.parent_id, c.name, c.sort_order FROM asset_categories ac JOIN categories c ON c.id=ac.category_id WHERE ac.asset_id IN ({placeholders}) ORDER BY c.sort_order, c.name COLLATE NOCASE"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(asset_ids.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            Category {
                id: row.get(1)?,
                parent_id: row.get(2)?,
                name: row.get(3)?,
                sort_order: row.get(4)?,
            },
        ))
    })?;
    let mut grouped = HashMap::<String, Vec<Category>>::new();
    for row in rows {
        let (asset_id, category) = row?;
        grouped.entry(asset_id).or_default().push(category);
    }
    Ok(grouped)
}

pub(super) fn asset_detail(
    connection: &Connection,
    library_root: &Path,
    asset_storage_root: &Path,
    asset_id: &str,
) -> LibraryResult<AssetDetail> {
    let (id, title, creator, source, product_url, version, note, cover_path): AssetDetailRow = connection.query_row(
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
        cover_path: cover_path.and_then(|path| {
            cached_thumbnail_path(library_root, asset_id)
                .ok()
                .flatten()
                .or_else(|| {
                    generate_cached_thumbnail(library_root, asset_storage_root, asset_id, &path)
                        .ok()?;
                    cached_thumbnail_path(library_root, asset_id).ok().flatten()
                })
        }),
        tags: tags_for_asset(connection, asset_id)?,
        categories,
        files,
        relations,
    })
}
