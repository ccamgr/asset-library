use super::super::*;

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
    let metadata_url = booth_metadata_url(&request.product_url)?;
    state.log(format!(
        "BOOTH metadata fetch requested url={metadata_url:?}"
    ));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("HTTPクライアントを初期化できません: {error}"))?;
    let response = client
        .get(&metadata_url)
        .send()
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
        cover_url: response
            .images
            .into_iter()
            .next()
            .map(|image| image.original),
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
