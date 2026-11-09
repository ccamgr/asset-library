use super::*;

pub(super) fn migrate(connection: &mut Connection) -> LibraryResult<()> {
    // A half-applied schema migration makes the library hard to recover.  Keep
    // schema creation, data cleanup, and the version marker in one SQLite
    // transaction instead.
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    let transaction = connection.transaction()?;
    migrate_schema(&transaction)?;
    transaction.commit()?;
    Ok(())
}

pub(super) fn migrate_schema(connection: &Connection) -> LibraryResult<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS app_metadata (
          key TEXT PRIMARY KEY NOT NULL,
          value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS assets (
          id TEXT PRIMARY KEY NOT NULL,
          title TEXT NOT NULL,
          creator TEXT,
          source TEXT,
          product_url TEXT,
          version TEXT,
          acquired_at INTEGER,
          note TEXT,
          cover_path TEXT,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS asset_files (
          id TEXT PRIMARY KEY NOT NULL,
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          storage_mode TEXT NOT NULL CHECK(storage_mode IN ('managed', 'external')),
          relative_path TEXT,
          external_path TEXT,
          original_filename TEXT NOT NULL,
          byte_size INTEGER NOT NULL,
          sha256 TEXT,
          created_at INTEGER NOT NULL,
          CHECK(
            (storage_mode = 'managed' AND relative_path IS NOT NULL AND external_path IS NULL) OR
            (storage_mode = 'external' AND external_path IS NOT NULL AND relative_path IS NULL)
          )
        );

        CREATE TABLE IF NOT EXISTS tags (
          id TEXT PRIMARY KEY NOT NULL,
          name TEXT NOT NULL COLLATE NOCASE UNIQUE,
          color TEXT
        );

        CREATE TABLE IF NOT EXISTS asset_tags (
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
          PRIMARY KEY (asset_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS categories (
          id TEXT PRIMARY KEY NOT NULL,
          parent_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
          name TEXT NOT NULL,
          sort_order INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS asset_categories (
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
          PRIMARY KEY (asset_id, category_id)
        );

        CREATE TABLE IF NOT EXISTS asset_relations (
          source_asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          target_asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          relation_type TEXT NOT NULL CHECK(relation_type IN ('addon_for', 'compatible_with', 'requires', 'variant_of', 'related_to')),
          note TEXT,
          created_at INTEGER NOT NULL,
          PRIMARY KEY (source_asset_id, target_asset_id, relation_type),
          CHECK(source_asset_id <> target_asset_id)
        );

        CREATE INDEX IF NOT EXISTS idx_assets_updated_at ON assets(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_asset_files_asset_id ON asset_files(asset_id);
        CREATE INDEX IF NOT EXISTS idx_asset_relations_target_asset_id ON asset_relations(target_asset_id);
        ",
    )?;

    let has_shop_url = connection
        .prepare("PRAGMA table_info(assets)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|column| column == "shop_url");
    if has_shop_url {
        connection.execute(
            "UPDATE assets SET product_url = COALESCE(NULLIF(product_url, ''), shop_url)",
            [],
        )?;
        connection.execute_batch("ALTER TABLE assets DROP COLUMN shop_url;")?;
    }
    let has_description = connection
        .prepare("PRAGMA table_info(assets)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|column| column == "description");
    if has_description {
        connection.execute(
            "UPDATE assets SET note = COALESCE(NULLIF(note, ''), description)",
            [],
        )?;
        connection.execute_batch("ALTER TABLE assets DROP COLUMN description;")?;
    }

    connection.execute_batch(
        "
        DELETE FROM asset_categories
        WHERE rowid NOT IN (
          SELECT MIN(rowid) FROM asset_categories GROUP BY asset_id
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_categories_one_per_asset
        ON asset_categories(asset_id);
        ",
    )?;

    connection.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('schema_version', '2')",
        [],
    )?;
    Ok(())
}

pub(super) fn count_assets(database_path: &Path) -> LibraryResult<i64> {
    let connection = Connection::open(database_path)?;
    Ok(connection.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))?)
}

#[allow(dead_code)]
pub(super) fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
