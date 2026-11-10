use super::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LibraryStatus {
    pub(super) default_library_path: String,
    pub(super) default_asset_storage_path: String,
    pub(super) default_database_backup_path: String,
    pub(super) default_database_path: String,
    pub(super) selected_library_path: Option<String>,
    pub(super) asset_storage_path: Option<String>,
    pub(super) database_backup_path: Option<String>,
    pub(super) database_path: Option<String>,
    pub(super) initialized: bool,
    pub(super) asset_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InitializeLibraryRequest {
    pub(super) asset_storage_path: String,
    pub(super) database_backup_path: String,
    #[serde(default)]
    pub(super) database_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ImportRequest {
    pub(super) title: String,
    pub(super) source_paths: Vec<String>,
    pub(super) creator: Option<String>,
    pub(super) product_url: Option<String>,
    pub(super) note: Option<String>,
    #[serde(default)]
    pub(super) tag_ids: Vec<String>,
    #[serde(default)]
    pub(super) category_ids: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct ImportResult {
    pub(super) asset_id: String,
    pub(super) title: String,
    pub(super) file_count: usize,
    pub(super) total_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyBatchImportRequest {
    pub(super) directory_paths: Vec<String>,
    #[serde(default)]
    pub(super) tag_ids: Vec<String>,
    #[serde(default)]
    pub(super) category_ids: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyImportFailure {
    pub(super) source_filename: String,
    pub(super) reason: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyDirectoryImportResult {
    pub(super) imported: Vec<ImportResult>,
    pub(super) failures: Vec<LegacyImportFailure>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyImportComplete {
    pub(super) result: Option<LegacyDirectoryImportResult>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyImportProgress {
    pub(super) completed: usize,
    pub(super) total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Tag {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Category {
    pub(super) id: String,
    pub(super) parent_id: Option<String>,
    pub(super) name: String,
    pub(super) sort_order: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssetSummary {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) creator: Option<String>,
    pub(super) search_text: String,
    pub(super) cover_path: Option<String>,
    pub(super) tags: Vec<Tag>,
    pub(super) categories: Vec<Category>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssetFileView {
    pub(super) id: String,
    pub(super) original_filename: String,
    pub(super) byte_size: i64,
    pub(super) storage_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssetRelationView {
    pub(super) asset_id: String,
    pub(super) title: String,
    pub(super) relation_type: String,
    pub(super) note: Option<String>,
    pub(super) direction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssetDetail {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) creator: Option<String>,
    pub(super) product_url: Option<String>,
    pub(super) note: Option<String>,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
    pub(super) cover_path: Option<String>,
    pub(super) tags: Vec<Tag>,
    pub(super) categories: Vec<Category>,
    pub(super) files: Vec<AssetFileView>,
    pub(super) relations: Vec<AssetRelationView>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssetUpdateRequest {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) creator: Option<String>,
    pub(super) product_url: Option<String>,
    pub(super) note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BoothMetadataRequest {
    pub(super) product_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BoothMetadata {
    pub(super) title: String,
    pub(super) creator: Option<String>,
    pub(super) note: Option<String>,
    pub(super) category: Option<String>,
    pub(super) tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BoothImportSettings {
    pub(super) import_categories: bool,
    pub(super) import_tags: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BoothImportSettingsRequest {
    pub(super) import_categories: bool,
    pub(super) import_tags: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssetAssignmentRequest {
    pub(super) asset_id: String,
    pub(super) ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AssetFilesRequest {
    pub(super) asset_id: String,
    pub(super) source_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NamedEntityRequest {
    pub(super) name: String,
    pub(super) color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RenameTagRequest {
    pub(super) id: String,
    pub(super) name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AddRelationRequest {
    pub(super) source_asset_id: String,
    pub(super) target_asset_id: String,
    pub(super) relation_type: String,
    pub(super) note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RemoveRelationRequest {
    pub(super) source_asset_id: String,
    pub(super) target_asset_id: String,
    pub(super) relation_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CapacityReport {
    pub(super) library_bytes: u64,
    pub(super) asset_bytes: u64,
    pub(super) database_bytes: u64,
    pub(super) thumbnail_cache_bytes: u64,
    pub(super) database_backup_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BackupResult {
    pub(super) path: String,
    pub(super) created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BackupEntry {
    pub(super) path: String,
    pub(super) name: String,
    pub(super) byte_size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThumbnailCacheRebuildResult {
    pub(super) generated: usize,
    pub(super) skipped: usize,
}

#[derive(Debug, Clone)]
pub(super) struct StorageLocations {
    pub(super) library_root: PathBuf,
    pub(super) asset_storage_root: PathBuf,
    pub(super) database_backup_root: PathBuf,
    pub(super) database_path: PathBuf,
}

#[derive(Clone)]
pub(super) struct ThumbnailJob {
    pub(super) library_root: PathBuf,
    pub(super) asset_storage_root: PathBuf,
    pub(super) asset_id: String,
    pub(super) cover_path: String,
}
