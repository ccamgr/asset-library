use super::super::*;

#[derive(Deserialize)]
struct BoothResponse {
    name: String,
    description: Option<String>,
    shop: Option<BoothShop>,
    category: Option<BoothCategory>,
    #[serde(default)]
    tags: Vec<BoothTag>,
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
    let metadata_url = booth_metadata_url(&request.product_url)?;
    state.log(format!(
        "BOOTH metadata fetch requested url={metadata_url:?}"
    ));
    let response = reqwest::blocking::get(&metadata_url)
        .map_err(|error| format!("BOOTHのメタデータを取得できません: {error}"))?
        .error_for_status()
        .map_err(|error| format!("BOOTHのメタデータを取得できません: {error}"))?
        .json::<BoothResponse>()
        .map_err(|error| format!("BOOTHのメタデータを読み取れません: {error}"))?;
    let settings = state
        .settings
        .lock()
        .map_err(|_| "設定のロックに失敗しました".to_string())?;
    Ok(BoothMetadata {
        title: response.name,
        creator: response.shop.map(|shop| shop.name),
        note: response.description,
        category: settings
            .import_booth_categories
            .then(|| response.category.map(|category| category.name))
            .flatten(),
        tags: if settings.import_booth_tags {
            response.tags.into_iter().map(|tag| tag.name).collect()
        } else {
            Vec::new()
        },
    })
}

fn booth_metadata_url(product_url: &str) -> Result<String, String> {
    let input = product_url.trim();
    let without_scheme = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
        .ok_or_else(|| "BOOTHの商品URLを入力してください".to_string())?;
    let mut segments = without_scheme.split('/');
    let host = segments.next().unwrap_or_default().to_ascii_lowercase();
    if host != "booth.pm" && !host.ends_with(".booth.pm") {
        return Err("BOOTHの商品URLを入力してください".into());
    }
    let first_path = segments.next();
    let item_segment = if first_path == Some("items") {
        first_path
    } else {
        segments.next()
    };
    if item_segment != Some("items") {
        return Err("BOOTHの商品URLを入力してください".into());
    }
    let item_id = segments
        .next()
        .unwrap_or_default()
        .trim_end_matches(".json");
    if item_id.is_empty() || !item_id.chars().all(|character| character.is_ascii_digit()) {
        return Err("BOOTHの商品URLを入力してください".into());
    }
    Ok(format!("https://{host}/items/{item_id}.json"))
}
