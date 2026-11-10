#[tauri::command]
pub(crate) fn list_assets(
    app: AppHandle,
    state: State<'_, AppState>,
    query: Option<String>,
    tag_ids: Option<Vec<String>>,
    category_id: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<AssetSummary>, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let search = format!("%{}%", query.unwrap_or_default().trim());
    let limit = limit.unwrap_or(100).clamp(1, 200) as i64;
    let offset = offset.unwrap_or(0) as i64;
    let tag_ids = tag_ids.unwrap_or_default();
    let mut filters = String::new();
    let mut values = vec![Value::Text(search)];
    if let Some(category_id) = category_id.filter(|id| !id.is_empty()) {
        filters.push_str(" AND EXISTS (SELECT 1 FROM asset_categories ac WHERE ac.asset_id = assets.id AND ac.category_id = ?)");
        values.push(Value::Text(category_id));
    }
    if !tag_ids.is_empty() {
        let tag_count = tag_ids.len() as i64;
        let placeholders = std::iter::repeat_n("?", tag_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        filters.push_str(&format!(
            " AND (SELECT COUNT(DISTINCT at.tag_id) FROM asset_tags at WHERE at.asset_id = assets.id AND at.tag_id IN ({placeholders})) = ?"
        ));
        values.extend(tag_ids.into_iter().map(Value::Text));
        values.push(Value::Integer(tag_count));
    }
    values.push(Value::Integer(limit));
    values.push(Value::Integer(offset));
    let mut statement = connection
        .prepare(&format!(
            "SELECT id, title, creator, cover_path,
                    title || ' ' || COALESCE(creator, '') || ' ' ||
                    COALESCE(product_url, '') || ' ' || COALESCE(note, '')
             FROM assets
             WHERE (title LIKE ?1
                OR COALESCE(creator, '') LIKE ?1
                OR COALESCE(product_url, '') LIKE ?1
                OR COALESCE(note, '') LIKE ?1)
             {filters}
             ORDER BY updated_at DESC, title COLLATE NOCASE
             LIMIT ? OFFSET ?"
        ))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;

    let rows = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let asset_ids = rows
        .iter()
        .map(|(id, _, _, _, _)| id.as_str())
        .collect::<Vec<_>>();
    let mut tags_by_asset =
        tags_for_assets(&connection, &asset_ids).map_err(|error| error.to_string())?;
    let mut categories_by_asset =
        categories_for_assets(&connection, &asset_ids).map_err(|error| error.to_string())?;
    let mut assets = Vec::with_capacity(rows.len());
    for row in rows {
        let (id, title, creator, cover_path, search_text) = row;
        let tags = tags_by_asset.remove(&id).unwrap_or_default();
        let categories = categories_by_asset.remove(&id).unwrap_or_default();
        let cover_path = cover_path.and_then(|path| {
            let cached = cached_thumbnail_path(&locations.library_root, &id)
                .ok()
                .flatten();
            if cached.is_none() {
                schedule_thumbnail_generation(&app, &state, &locations, &id, &path);
            }
            cached
        });
        assets.push(AssetSummary {
            id,
            title,
            creator,
            search_text,
            cover_path,
            tags,
            categories,
        });
    }
    Ok(assets)
}

#[tauri::command]
pub(crate) fn get_asset_detail(
    app: AppHandle,
    state: State<'_, AppState>,
    asset_id: String,
) -> Result<AssetDetail, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    asset_detail(
        &connection,
        &locations.library_root,
        &locations.asset_storage_root,
        &asset_id,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_asset(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetUpdateRequest,
) -> Result<(), String> {
    state.log(format!("asset update started asset_id={}", request.id));
    if request.title.trim().is_empty() {
        return Err("アセット名を入力してください".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let changed = connection
        .execute(
            "UPDATE assets SET title=?2, creator=?3, product_url=?4,
             note=?5, updated_at=?6 WHERE id=?1",
            params![
                request.id,
                request.title.trim(),
                optional_text(request.creator),
                optional_text(request.product_url),
                optional_text(request.note),
                unix_timestamp(),
            ],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("アセットが見つかりません".into());
    }
    state.log(format!("asset updated asset_id={}", request.id));
    Ok(())
}

use super::super::*;
