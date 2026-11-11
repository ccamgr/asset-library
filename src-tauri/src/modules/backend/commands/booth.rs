use super::super::*;
use rusqlite::OptionalExtension;

#[derive(Deserialize)]
struct BoothResponse {
    name: String,
    description: Option<String>,
    shop: Option<BoothShop>,
    category: Option<BoothCategory>,
    #[serde(default)]
    tags: Vec<BoothTag>,
    #[serde(default)]
    images: Vec<BoothImage>,
}

#[derive(Deserialize)]
struct BoothShop {
    name: String,
}

#[derive(Deserialize)]
struct BoothCategory {
    name: String,
}

#[derive(Deserialize)]
struct BoothTag {
    name: String,
}

#[derive(Deserialize)]
struct BoothImage {
    original: String,
}

#[tauri::command]
pub(crate) fn get_booth_import_settings(
    state: State<'_, AppState>,
) -> Result<BoothImportSettings, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?;
    Ok(BoothImportSettings {
        import_categories: settings.import_booth_categories,
        import_tags: settings.import_booth_tags,
    })
}

#[tauri::command]
pub(crate) fn set_booth_import_settings(
    state: State<'_, AppState>,
    request: BoothImportSettingsRequest,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?;
    settings.import_booth_categories = request.import_categories;
    settings.import_booth_tags = request.import_tags;
    state
        .save_settings(&settings)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn fetch_booth_metadata(
    state: State<'_, AppState>,
    request: BoothMetadataRequest,
) -> Result<BoothMetadata, String> {
    let settings = booth_import_settings(&state)?;
    state.log(format!(
        "BOOTH metadata fetch requested url={:?}",
        booth_metadata_url(&request.product_url)?
    ));
    let client = booth_http_client()?;
    fetch_booth_metadata_with_client(&client, &request.product_url, &settings)
}

#[tauri::command]
pub(crate) fn start_booth_metadata_refresh(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let locations = storage_locations(&app, &state).map_err(|error| error.to_string())?;
    let connection =
        Connection::open(&locations.database_path).map_err(|error| error.to_string())?;
    let assets = connection
        .prepare("SELECT id, title, product_url FROM assets WHERE product_url IS NOT NULL")
        .map_err(|error| error.to_string())?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|(_, _, product_url)| booth_metadata_url(product_url).is_ok())
        .collect::<Vec<_>>();
    if assets.is_empty() {
        return Err("BOOTHの商品URLが設定されたアセットはありません".into());
    }
    let settings = booth_import_settings(&state)?;
    let running = Arc::clone(&state.booth_metadata_refresh_running);
    if running.swap(true, Ordering::AcqRel) {
        return Err("BOOTHメタデータの再取得はすでに実行中です".into());
    }
    let total = assets.len();
    let log_path = state.log_path.clone();
    state.log(format!("BOOTH metadata refresh started assets={total}"));
    let _ = app.emit(
        "booth-metadata-refresh-started",
        BoothMetadataRefreshProgress {
            completed: 0,
            total,
        },
    );
    std::thread::spawn(move || {
        let result = refresh_booth_metadata_batch(&app, &locations, &settings, assets, &log_path);
        let complete = match result {
            Ok(result) => BoothMetadataRefreshComplete {
                result: Some(result),
                error: None,
            },
            Err(error) => BoothMetadataRefreshComplete {
                result: None,
                error: Some(error.to_string()),
            },
        };
        append_log(
            &log_path,
            &format!(
                "BOOTH metadata refresh {}",
                if complete.error.is_some() {
                    "failed"
                } else {
                    "completed"
                }
            ),
        );
        running.store(false, Ordering::Release);
        let _ = app.emit("booth-metadata-refresh-complete", complete);
    });
    Ok(total)
}

fn booth_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("HTTPクライアントを初期化できません: {error}"))
}

fn fetch_booth_metadata_with_client(
    client: &reqwest::blocking::Client,
    product_url: &str,
    settings: &BoothImportSettings,
) -> Result<BoothMetadata, String> {
    let metadata_url = booth_metadata_url(product_url)?;
    let response = client
        .get(&metadata_url)
        .send()
        .map_err(|error| format!("BOOTHのメタデータを取得できません: {error}"))?
        .error_for_status()
        .map_err(|error| format!("BOOTHのメタデータを取得できません: {error}"))?
        .json::<BoothResponse>()
        .map_err(|error| format!("BOOTHのメタデータを読み取れません: {error}"))?;
    Ok(BoothMetadata {
        title: response.name,
        creator: response.shop.map(|shop| shop.name),
        note: response.description,
        cover_url: response
            .images
            .into_iter()
            .next()
            .map(|image| image.original),
        category: settings
            .import_categories
            .then(|| response.category.map(|category| category.name))
            .flatten(),
        tags: if settings.import_tags {
            response.tags.into_iter().map(|tag| tag.name).collect()
        } else {
            Vec::new()
        },
    })
}

fn booth_import_settings(state: &AppState) -> Result<BoothImportSettings, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?;
    Ok(BoothImportSettings {
        import_categories: settings.import_booth_categories,
        import_tags: settings.import_booth_tags,
    })
}

fn refresh_booth_metadata_batch(
    app: &AppHandle,
    locations: &StorageLocations,
    settings: &BoothImportSettings,
    assets: Vec<(String, String, String)>,
    log_path: &Path,
) -> LibraryResult<BoothMetadataRefreshResult> {
    let total = assets.len();
    let mut connection = Connection::open(&locations.database_path)?;
    let client = booth_http_client().map_err(LibraryError::Message)?;
    let mut result = BoothMetadataRefreshResult {
        updated: 0,
        failures: Vec::new(),
    };
    for (index, (asset_id, title, product_url)) in assets.into_iter().enumerate() {
        match fetch_booth_metadata_with_client(&client, &product_url, settings)
            .map_err(LibraryError::Message)
            .and_then(|metadata| apply_booth_metadata(&mut connection, &asset_id, metadata))
        {
            Ok(()) => {
                result.updated += 1;
                append_log(
                    log_path,
                    &format!("BOOTH metadata refresh completed asset_id={asset_id}"),
                );
            }
            Err(error) => {
                append_log(
                    log_path,
                    &format!("BOOTH metadata refresh failed asset_id={asset_id} reason={error}"),
                );
                result.failures.push(BoothMetadataRefreshFailure {
                    title,
                    reason: error.to_string(),
                });
            }
        }
        let _ = app.emit(
            "booth-metadata-refresh-progress",
            BoothMetadataRefreshProgress {
                completed: index + 1,
                total,
            },
        );
    }
    Ok(result)
}

fn apply_booth_metadata(
    connection: &mut Connection,
    asset_id: &str,
    metadata: BoothMetadata,
) -> LibraryResult<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE assets SET title=?2, creator=?3, note=?4, updated_at=?5 WHERE id=?1",
        params![
            asset_id,
            metadata.title,
            metadata.creator,
            metadata.note,
            unix_timestamp()
        ],
    )?;
    if let Some(category_name) = metadata.category {
        let category_id = find_or_create_category(&transaction, &category_name)?;
        transaction.execute(
            "DELETE FROM asset_categories WHERE asset_id=?1",
            params![asset_id],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO asset_categories (asset_id, category_id) VALUES (?1, ?2)",
            params![asset_id, category_id],
        )?;
    }
    for tag_name in metadata.tags {
        let tag_id = find_or_create_tag(&transaction, &tag_name)?;
        transaction.execute(
            "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
            params![asset_id, tag_id],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn find_or_create_category(
    connection: &rusqlite::Transaction<'_>,
    name: &str,
) -> LibraryResult<String> {
    let existing = connection
        .query_row(
            "SELECT id FROM categories WHERE name=?1 COLLATE NOCASE",
            params![name],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO categories (id, parent_id, name, sort_order) VALUES (?1, NULL, ?2, 0)",
        params![id, name],
    )?;
    Ok(id)
}

fn find_or_create_tag(connection: &rusqlite::Transaction<'_>, name: &str) -> LibraryResult<String> {
    let existing = connection
        .query_row(
            "SELECT id FROM tags WHERE name=?1 COLLATE NOCASE",
            params![name],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO tags (id, name, color) VALUES (?1, ?2, NULL)",
        params![id, name],
    )?;
    Ok(id)
}

pub(crate) fn booth_metadata_url(product_url: &str) -> Result<String, String> {
    let input = product_url.trim();
    let (scheme, without_scheme) = input
        .strip_prefix("https://")
        .map(|path| ("https://", path))
        .or_else(|| input.strip_prefix("http://").map(|path| ("http://", path)))
        .ok_or_else(|| "BOOTHの商品URLを入力してください".to_string())?;
    let mut segments = without_scheme.split('/');
    let host = segments.next().unwrap_or_default();
    let normalized_host = host.to_ascii_lowercase();
    if normalized_host != "booth.pm" && !normalized_host.ends_with(".booth.pm") {
        return Err("BOOTHの商品URLを入力してください".into());
    }
    let first_path = segments.next().unwrap_or_default();
    let (path_prefix, item_segment) = if first_path == "items" {
        ("", first_path)
    } else {
        (first_path, segments.next().unwrap_or_default())
    };
    if item_segment != "items" {
        return Err("BOOTHの商品URLを入力してください".into());
    }
    let item_id = segments
        .next()
        .unwrap_or_default()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches(".json");
    if item_id.is_empty() || !item_id.chars().all(|character| character.is_ascii_digit()) {
        return Err("BOOTHの商品URLを入力してください".into());
    }
    let path_prefix = if path_prefix.is_empty() {
        String::new()
    } else {
        format!("{path_prefix}/")
    };
    Ok(format!("{scheme}{host}/{path_prefix}items/{item_id}.json"))
}
