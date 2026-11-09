#[tauri::command]
pub(crate) fn list_tags(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Tag>, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    all_tags(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn create_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    request: NamedEntityRequest,
) -> Result<Tag, String> {
    let name = non_empty_name(&request.name).map_err(|error| error.to_string())?;
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let id = Uuid::now_v7().to_string();
    connection
        .execute(
            "INSERT OR IGNORE INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
            params![id, name, optional_text(request.color)],
        )
        .map_err(|error| error.to_string())?;
    connection
        .query_row(
            "SELECT id, name, color FROM tags WHERE name=?1 COLLATE NOCASE",
            params![name],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            },
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn delete_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    tag_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| error.to_string())?;
    connection
        .execute("DELETE FROM tags WHERE id=?1", params![tag_id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn delete_category(
    app: AppHandle,
    state: State<'_, AppState>,
    category_id: String,
) -> Result<(), String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let mut connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    // Removing the assignment first deliberately leaves affected assets uncategorized.
    transaction
        .execute(
            "DELETE FROM asset_categories WHERE category_id=?1",
            params![category_id],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM categories WHERE id=?1", params![category_id])
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn rename_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RenameTagRequest,
) -> Result<(), String> {
    let name = non_empty_name(&request.name).map_err(|error| error.to_string())?;
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let changed = connection
        .execute(
            "UPDATE tags SET name=?1 WHERE id=?2",
            params![name, request.id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("タグが見つかりません".into());
    }
    Ok(())
}

use super::super::*;
