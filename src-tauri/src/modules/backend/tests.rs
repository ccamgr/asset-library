use super::*;

#[test]
fn creates_a_library_schema() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-test-{}", unix_timestamp()));
    prepare_library(
        &root,
        &root.join(DATABASE_FILE),
        &root.join("asset-storage"),
        &root.join("db-backups"),
    )
    .unwrap();

    let database_path = root.join(DATABASE_FILE);
    assert!(database_path.is_file());
    assert_eq!(count_assets(&database_path).unwrap(), 0);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn creates_library_database_at_a_custom_path() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-custom-db-{}", Uuid::now_v7()));
    let library = root.join("library");
    let database = root.join("database").join(DATABASE_FILE);
    let asset_storage = root.join("asset-storage");
    let backups = root.join("backups");

    prepare_library(&library, &database, &asset_storage, &backups).unwrap();

    assert!(database.is_file());
    assert!(!library.join(DATABASE_FILE).exists());
    assert_eq!(count_assets(&database).unwrap(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn settings_without_window_state_remain_supported() {
    let settings: AppSettings = serde_json::from_str(
        r#"{"assetStoragePath":"C:/assets","databaseBackupPath":"C:/backups"}"#,
    )
    .expect("older settings should deserialize");

    assert!(settings.window_state.is_none());
}

#[test]
fn refuses_an_empty_storage_path() {
    assert!(validate_storage_path("  ", "アセット保存先").is_err());
}

#[test]
fn refuses_a_relative_database_path() {
    assert!(validate_database_path(DATABASE_FILE).is_err());
}

#[test]
fn refuses_an_invalid_asset_id_for_storage_paths() {
    assert!(asset_directory(Path::new("/tmp/assets"), "../outside").is_err());
}

#[test]
fn shards_asset_directories_by_random_uuid_suffix() {
    let root = Path::new("/tmp/assets");
    let asset_id = "01900000-0000-7000-8000-0000000000af";
    assert_eq!(
        asset_directory(root, asset_id).unwrap(),
        root.join("assets").join("af").join(asset_id)
    );
}

#[test]
fn stores_asset_paths_in_the_same_shard_as_the_asset_directory() {
    let asset_id = "01900000-0000-7000-8000-0000000000af";
    assert_eq!(
        asset_relative_path(asset_id, "files/example.zip").unwrap(),
        format!("assets/af/{asset_id}/files/example.zip")
    );
    assert_eq!(
        asset_relative_path(asset_id, "cover.png").unwrap(),
        format!("assets/af/{asset_id}/cover.png")
    );
}

#[test]
fn counts_an_external_database_in_library_capacity() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-capacity-{}", Uuid::now_v7()));
    let database_path = root.join("database").join(DATABASE_FILE);
    fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    fs::write(&database_path, b"database").unwrap();
    let locations = StorageLocations {
        library_root: root.join("library"),
        asset_storage_root: root.join("assets"),
        database_backup_root: root.join("backups"),
        database_path,
    };

    assert_eq!(storage_capacity_report(&locations).database_bytes, 8);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn migrates_shop_url_into_the_single_product_url() {
    let database = std::env::temp_dir().join(format!("vr-asset-library-url-{}.db", Uuid::now_v7()));
    let mut connection = Connection::open(&database).unwrap();
    connection.execute_batch("CREATE TABLE assets (id TEXT PRIMARY KEY NOT NULL, title TEXT NOT NULL, creator TEXT, source TEXT, product_url TEXT, shop_url TEXT, description TEXT, version TEXT, acquired_at INTEGER, note TEXT, cover_path TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL); INSERT INTO assets (id, title, shop_url, created_at, updated_at) VALUES ('asset-1', 'Legacy', 'https://booth.pm/ja/items/1', 0, 0);").unwrap();
    migrate(&mut connection).unwrap();
    let url: Option<String> = connection
        .query_row(
            "SELECT product_url FROM assets WHERE id='asset-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(url.as_deref(), Some("https://booth.pm/ja/items/1"));
    let columns = connection
        .prepare("PRAGMA table_info(assets)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(!columns.iter().any(|column| column == "shop_url"));
    fs::remove_file(database).unwrap();
}

#[test]
fn schema_allows_only_one_category_per_asset() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-category-{}", Uuid::now_v7()));
    prepare_library(
        &root,
        &root.join(DATABASE_FILE),
        &root.join("asset-storage"),
        &root.join("db-backups"),
    )
    .unwrap();
    let connection = Connection::open(root.join(DATABASE_FILE)).unwrap();
    connection.execute("INSERT INTO assets (id, title, created_at, updated_at) VALUES ('asset-1', 'Asset', 0, 0)", []).unwrap();
    connection
        .execute(
            "INSERT INTO categories (id, name) VALUES ('category-1', 'One'), ('category-2', 'Two')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO asset_categories (asset_id, category_id) VALUES ('asset-1', 'category-1')",
            [],
        )
        .unwrap();
    assert!(connection
        .execute(
            "INSERT INTO asset_categories (asset_id, category_id) VALUES ('asset-1', 'category-2')",
            []
        )
        .is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn creates_a_small_local_thumbnail_cache() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-thumbnail-{}", Uuid::now_v7()));
    let asset_root = root.join("asset-storage");
    let asset_id = Uuid::now_v7().to_string();
    let relative_path = asset_relative_path(&asset_id, "cover.png").unwrap();
    let source = asset_root.join(&relative_path);
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    DynamicImage::new_rgb8(1600, 900).save(&source).unwrap();

    generate_cached_thumbnail(&root, &asset_root, &asset_id, &relative_path).unwrap();
    fs::remove_file(&source).unwrap();
    let cached = cached_thumbnail_path(&root, &asset_id).unwrap().unwrap();
    let dimensions = image::image_dimensions(&cached).unwrap();
    assert!(dimensions.0 <= 480);
    assert!(dimensions.1 <= 360);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn imports_multiple_files_into_an_asset_directory() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-import-{}", Uuid::now_v7()));
    let source = root.join("source");
    let library = root.join("library");
    let asset_storage = root.join("asset-storage");
    let database_backups = root.join("db-backups");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("main.zip"), b"main asset").unwrap();
    fs::write(source.join("readme.txt"), b"instructions").unwrap();
    prepare_library(
        &library,
        &library.join(DATABASE_FILE),
        &asset_storage,
        &database_backups,
    )
    .unwrap();

    let result = import_into_library(
        &library.join(DATABASE_FILE),
        &asset_storage,
        ImportRequest {
            title: "Test asset".into(),
            source_paths: vec![
                display_path(&source.join("main.zip")),
                display_path(&source.join("readme.txt")),
            ],
            creator: None,
            source: None,
            product_url: None,
            version: None,
            note: None,
            tag_ids: vec![],
            category_ids: vec![],
        },
    )
    .unwrap();

    assert_eq!(result.file_count, 2);
    assert_eq!(count_assets(&library.join(DATABASE_FILE)).unwrap(), 1);
    let asset_directory = asset_directory(&asset_storage, &result.asset_id).unwrap();
    assert_eq!(
        fs::read_dir(asset_directory.join("files")).unwrap().count(),
        2
    );
    let stored_path: String = Connection::open(library.join(DATABASE_FILE))
            .unwrap()
            .query_row(
                "SELECT relative_path FROM asset_files WHERE asset_id = ?1 ORDER BY original_filename LIMIT 1",
                params![result.asset_id],
                |row| row.get(0),
            )
            .unwrap();
    assert!(stored_path.starts_with(&asset_relative_directory(&result.asset_id).unwrap()));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn imports_legacy_archive_children_as_separate_archives() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-legacy-{}", Uuid::now_v7()));
    let library = root.join("library");
    let asset_storage = root.join("asset-storage");
    let backups = root.join("backups");
    let source_dir = root.join("source");
    let source_zip = source_dir.join("legacy.zip");
    fs::create_dir_all(&source_dir).unwrap();
    prepare_library(
        &library,
        &library.join(DATABASE_FILE),
        &asset_storage,
        &backups,
    )
    .unwrap();
    {
        let file = File::create(&source_zip).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file("_assetinfo.md", options).unwrap();
        writer
            .write_all("https://booth.pm/ja/items/123".as_bytes())
            .unwrap();
        writer.start_file("_thumbnail.png", options).unwrap();
        writer.write_all(b"not-a-real-jpeg").unwrap();
        writer.start_file("Product\\readme.txt", options).unwrap();
        writer.write_all(b"asset data").unwrap();
        writer
            .start_file("Product\\Folder\\data.txt", options)
            .unwrap();
        writer.write_all(b"nested asset data").unwrap();
        writer.start_file("Product\\already.zip", options).unwrap();
        writer.write_all(b"existing archive").unwrap();
        writer.finish().unwrap();
    }
    let locations = StorageLocations {
        library_root: library.clone(),
        asset_storage_root: asset_storage.clone(),
        database_backup_root: backups,
        database_path: library.join(DATABASE_FILE),
    };
    let result = import_legacy_archive(&locations, &source_zip).unwrap();
    assert_eq!(result.title, "Product");
    assert_eq!(result.file_count, 3);
    assert_eq!(count_assets(&library.join(DATABASE_FILE)).unwrap(), 1);
    let asset_directory = asset_directory(&asset_storage, &result.asset_id).unwrap();
    assert!(asset_directory.join("cover.png").is_file());
    let detail = Connection::open(library.join(DATABASE_FILE))
        .unwrap()
        .query_row(
            "SELECT product_url, note FROM assets WHERE id = ?1",
            params![result.asset_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .unwrap();
    assert_eq!(detail.0, "https://booth.pm/ja/items/123");
    assert_eq!(detail.1, None);
    let stored = fs::read_dir(asset_directory.join("files"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(stored.len(), 3);
    let names = stored
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(names.contains(&"readme.txt".to_owned()));
    assert!(names.contains(&"already.zip".to_owned()));
    assert!(names.contains(&"Folder.zip".to_owned()));
    assert_eq!(
        fs::read(asset_directory.join("files").join("already.zip")).unwrap(),
        b"existing archive"
    );
    let folder_archive =
        ZipArchive::new(File::open(asset_directory.join("files").join("Folder.zip")).unwrap())
            .unwrap();
    assert!(folder_archive.file_names().any(|name| name == "data.txt"));
    assert!(!folder_archive
        .file_names()
        .any(|name| name.starts_with("Folder/")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn keeps_legacy_asset_info_lines_after_the_url_as_a_note() {
    assert_eq!(
        parse_legacy_asset_info("https://booth.pm/ja/items/123\n対応アバター: Example\n"),
        (
            Some("https://booth.pm/ja/items/123".into()),
            Some("対応アバター: Example".into())
        )
    );
}

#[test]
fn database_snapshot_contains_imported_assets() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-backup-{}", Uuid::now_v7()));
    let library = root.join("library");
    let asset_storage = root.join("asset-storage");
    let database_backups = root.join("db-backups");
    let source = root.join("source.zip");
    fs::create_dir_all(&root).unwrap();
    fs::write(&source, b"asset").unwrap();
    prepare_library(
        &library,
        &library.join(DATABASE_FILE),
        &asset_storage,
        &database_backups,
    )
    .unwrap();
    import_into_library(
        &library.join(DATABASE_FILE),
        &asset_storage,
        ImportRequest {
            title: "Backup test".into(),
            source_paths: vec![display_path(&source)],
            creator: None,
            source: None,
            product_url: None,
            version: None,
            note: None,
            tag_ids: vec![],
            category_ids: vec![],
        },
    )
    .unwrap();

    let snapshot = database_backups.join("snapshot.db");
    copy_database_snapshot(&library.join(DATABASE_FILE), &snapshot).unwrap();
    assert_eq!(count_assets(&snapshot).unwrap(), 1);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn appends_sanitized_log_events() {
    let root = std::env::temp_dir().join(format!("vr-asset-library-log-{}", Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let log_path = root.join("asset-library.log");

    append_log(&log_path, "operation started\nwith details");

    let contents = fs::read_to_string(&log_path).unwrap();
    assert!(contents.contains("operation started with details"));
    assert_eq!(contents.lines().count(), 1);
    fs::remove_dir_all(root).unwrap();
}
