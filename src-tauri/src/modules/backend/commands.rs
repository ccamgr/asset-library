#[macro_use]
mod access;
#[macro_use]
mod catalog;
#[macro_use]
mod booth;
#[macro_use]
mod classification;
#[macro_use]
mod files;
#[macro_use]
mod labels;
#[macro_use]
mod library;
#[macro_use]
mod maintenance;

pub(super) use access::*;
pub(super) use booth::*;
pub(super) use catalog::*;
pub(super) use classification::*;
pub(super) use files::*;
pub(super) use labels::*;
pub(super) use library::*;
pub(super) use maintenance::*;

pub(super) fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_library_status,
        initialize_library,
        import_files,
        count_legacy_archives,
        start_legacy_import,
        list_assets,
        get_asset_detail,
        update_asset,
        get_booth_import_settings,
        set_booth_import_settings,
        fetch_booth_metadata,
        list_tags,
        create_tag,
        delete_tag,
        rename_tag,
        set_asset_tags,
        list_categories,
        create_category,
        delete_category,
        set_asset_categories,
        add_asset_relation,
        remove_asset_relation,
        set_asset_cover,
        set_asset_cover_from_url,
        delete_asset,
        add_asset_files,
        delete_asset_file,
        get_asset_file_path,
        reveal_asset_file,
        reveal_asset_directory,
        open_asset_file,
        open_external_url,
        get_capacity_report,
        rebuild_thumbnail_cache,
        create_database_backup,
        list_database_backups,
        restore_database_backup
    ]
}
