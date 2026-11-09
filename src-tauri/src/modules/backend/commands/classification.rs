#[tauri::command]
pub(crate) fn set_asset_tags(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetAssignmentRequest,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let mut connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM asset_tags WHERE asset_id=?1",
            params![request.asset_id],
        )
        .map_err(|error| error.to_string())?;
    for tag_id in request.ids {
        transaction
            .execute(
                "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
                params![request.asset_id, tag_id],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn list_categories(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<Category>, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    all_categories(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn create_category(
    app: AppHandle,
    state: State<'_, AppState>,
    request: NamedEntityRequest,
) -> Result<Category, String> {
    let name = non_empty_name(&request.name).map_err(|error| error.to_string())?;
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let category = Category {
        id: Uuid::now_v7().to_string(),
        parent_id: None,
        name,
        sort_order: 0,
    };
    connection
        .execute(
            "INSERT INTO categories (id, parent_id, name, sort_order) VALUES (?1, NULL, ?2, 0)",
            params![category.id, category.name],
        )
        .map_err(|error| error.to_string())?;
    Ok(category)
}

#[tauri::command]
pub(crate) fn set_asset_categories(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssetAssignmentRequest,
) -> Result<(), String> {
    if request.ids.len() > 1 {
        return Err("カテゴリはアセットごとに1つだけ設定できます".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let mut connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM asset_categories WHERE asset_id=?1",
            params![request.asset_id],
        )
        .map_err(|error| error.to_string())?;
    for category_id in request.ids {
        transaction
            .execute(
                "INSERT OR IGNORE INTO asset_categories (asset_id, category_id) VALUES (?1, ?2)",
                params![request.asset_id, category_id],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn add_asset_relation(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AddRelationRequest,
) -> Result<(), String> {
    if request.source_asset_id == request.target_asset_id {
        return Err("同じアセット同士は関連付けできません".into());
    }
    if ![
        "addon_for",
        "compatible_with",
        "requires",
        "variant_of",
        "related_to",
    ]
    .contains(&request.relation_type.as_str())
    {
        return Err("不正な関連タイプです".into());
    }
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    connection.execute("INSERT OR REPLACE INTO asset_relations (source_asset_id, target_asset_id, relation_type, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![request.source_asset_id, request.target_asset_id, request.relation_type, optional_text(request.note), unix_timestamp()]).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn remove_asset_relation(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RemoveRelationRequest,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    connection.execute("DELETE FROM asset_relations WHERE source_asset_id=?1 AND target_asset_id=?2 AND relation_type=?3", params![request.source_asset_id, request.target_asset_id, request.relation_type]).map_err(|error| error.to_string())?;
    Ok(())
}

use super::super::*;
