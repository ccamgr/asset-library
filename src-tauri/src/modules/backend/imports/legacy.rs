pub(crate) fn import_legacy_archive(
    locations: &StorageLocations,
    archive_path: &Path,
) -> LibraryResult<ImportResult> {
    let database_path = locations.database_path.clone();
    if !database_path.is_file() {
        return Err(LibraryError::Message(
            "ライブラリが初期化されていません".into(),
        ));
    }

    let archive_file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(BufReader::new(archive_file))
        .map_err(|error| LibraryError::Message(format!("ZIPを開けません: {error}")))?;
    let entries = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .map(|entry| LegacyArchiveEntry {
                    source_name: entry.name().to_owned(),
                    normalized_name: normalize_archive_name(entry.name()),
                })
                .map_err(|error| LibraryError::Message(format!("ZIPを読み取れません: {error}")))
        })
        .collect::<LibraryResult<Vec<_>>>()?;
    if entries
        .iter()
        .any(|entry| !safe_archive_entry(&entry.normalized_name))
    {
        return Err(LibraryError::Message(
            "ZIP内に安全でないパスが含まれています".into(),
        ));
    }
    let info = entries
        .iter()
        .find(|entry| entry.normalized_name == "_assetinfo.md")
        .ok_or_else(|| LibraryError::Message("_assetinfo.md がありません".into()))?;
    let thumbnail = entries
        .iter()
        .find(|entry| {
            matches!(
                entry.normalized_name.as_str(),
                "_thumbnail.jpg" | "_thumbnail.jpeg" | "_thumbnail.png"
            )
        })
        .ok_or_else(|| {
            LibraryError::Message(
                "_thumbnail.jpg、_thumbnail.jpeg、_thumbnail.png のいずれかがありません".into(),
            )
        })?;

    let mut roots = entries
        .iter()
        .filter_map(|entry| archive_root_directory(&entry.normalized_name))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    if roots.len() != 1 {
        return Err(LibraryError::Message(
            "商品ファイルの最上位ディレクトリを1つだけ含めてください".into(),
        ));
    }
    let product_directory = roots.remove(0);
    let title = safe_archive_name(&product_directory)?;
    let info = read_zip_entry(&mut archive, &info.source_name)?;
    let thumbnail_bytes = read_zip_entry(&mut archive, &thumbnail.source_name)?;
    let cover_filename = format!(
        "cover.{}",
        Path::new(&thumbnail.normalized_name)
            .extension()
            .and_then(|extension| extension.to_str())
            .ok_or_else(|| LibraryError::Message("サムネイルの形式が不正です".into()))?
    );
    let product_groups = product_archive_groups(&entries, &product_directory)?;

    let asset_id = Uuid::now_v7().to_string();
    let staging_asset_dir = std::env::temp_dir().join("VRAssetLibrary").join(&asset_id);
    let staging_files_dir = staging_asset_dir.join("files");
    fs::create_dir_all(&staging_files_dir)?;
    let result = (|| -> LibraryResult<ImportResult> {
        let mut files = Vec::with_capacity(product_groups.len());
        let mut total_bytes = 0_u64;
        for (group_name, group_entries) in product_groups {
            let file_id = Uuid::now_v7().to_string();
            let is_directory = archive_group_is_directory(&group_name, &group_entries);
            let requested_filename = if is_directory {
                format!("{group_name}.zip")
            } else {
                group_name.clone()
            };
            let original_filename = safe_filename(Path::new(&requested_filename))?;
            let stored_filename = available_filename(&staging_files_dir, &original_filename, None);
            let staged_path = staging_files_dir.join(&stored_filename);
            let byte_size = if is_directory {
                write_directory_archive(&mut archive, &group_name, &group_entries, &staged_path)?
            } else {
                copy_archive_file(&mut archive, &group_entries, &staged_path)?
            };
            total_bytes = total_bytes.saturating_add(byte_size);
            files.push(ImportedFile {
                id: file_id,
                original_filename,
                stored_filename,
                byte_size,
            });
        }
        fs::write(staging_asset_dir.join(&cover_filename), thumbnail_bytes)?;
        let asset_directory = asset_directory(&locations.asset_storage_root, &asset_id)?;
        copy_staged_asset(
            &staging_asset_dir,
            &asset_directory,
            &files,
            Some(&cover_filename),
        )?;
        let info = String::from_utf8_lossy(&info);
        let (product_url, note) = parse_legacy_asset_info(&info);
        let database_result = insert_legacy_imported_asset(
            &database_path,
            &asset_id,
            &title,
            product_url.as_deref(),
            note.as_deref(),
            &cover_filename,
            &files,
        );
        if let Err(error) = database_result {
            let _ = fs::remove_dir_all(&asset_directory);
            return Err(error);
        }
        Ok(ImportResult {
            asset_id,
            title,
            file_count: files.len(),
            total_bytes,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_asset_dir);
    }
    result
}

#[derive(Clone)]
pub(crate) struct LegacyArchiveEntry {
    source_name: String,
    normalized_name: String,
}

pub(crate) fn normalize_archive_name(name: &str) -> String {
    name.replace('\\', "/")
}

pub(crate) fn archive_root_directory(name: &str) -> Option<String> {
    if name.starts_with('_') || !safe_archive_entry(name) {
        return None;
    }
    let normalized = name.trim_end_matches('/');
    let mut components = normalized.split('/');
    let first = components.next()?;
    if components.next().is_none() && !name.ends_with('/') {
        return None;
    }
    Some(first.to_owned())
}

pub(crate) fn safe_archive_entry(name: &str) -> bool {
    let normalized = normalize_archive_name(name);
    !normalized.is_empty()
        && !normalized.starts_with('/')
        && normalized
            .trim_end_matches('/')
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

pub(crate) fn safe_archive_name(name: &str) -> LibraryResult<String> {
    safe_filename(Path::new(name))
}

pub(crate) fn read_zip_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> LibraryResult<Vec<u8>> {
    let mut entry = archive
        .by_name(name)
        .map_err(|error| LibraryError::Message(format!("ZIP内の {name} を読めません: {error}")))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub(crate) fn product_archive_groups(
    entries: &[LegacyArchiveEntry],
    product_directory: &str,
) -> LibraryResult<BTreeMap<String, Vec<LegacyArchiveEntry>>> {
    let prefix = format!("{product_directory}/");
    let mut groups = BTreeMap::new();
    for entry in entries {
        let Some(relative_name) = entry.normalized_name.strip_prefix(&prefix) else {
            continue;
        };
        if relative_name.is_empty() {
            continue;
        }
        let group_name = relative_name
            .split('/')
            .next()
            .ok_or_else(|| LibraryError::Message("商品ファイルの構成が不正です".into()))?;
        groups
            .entry(group_name.to_owned())
            .or_insert_with(Vec::new)
            .push(LegacyArchiveEntry {
                source_name: entry.source_name.clone(),
                normalized_name: relative_name.to_owned(),
            });
    }
    if groups.is_empty() {
        return Err(LibraryError::Message("商品ディレクトリが空です".into()));
    }
    Ok(groups)
}

pub(crate) fn archive_group_is_directory(group_name: &str, entries: &[LegacyArchiveEntry]) -> bool {
    let prefix = format!("{group_name}/");
    entries
        .iter()
        .any(|entry| entry.normalized_name.starts_with(&prefix))
}

pub(crate) fn copy_archive_file<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    entries: &[LegacyArchiveEntry],
    destination: &Path,
) -> LibraryResult<u64> {
    let entry_name = entries
        .iter()
        .find(|entry| !entry.normalized_name.ends_with('/'))
        .ok_or_else(|| LibraryError::Message("商品ファイルの構成が不正です".into()))?;
    if entries.len() != 1 {
        return Err(LibraryError::Message("商品ファイルの構成が不正です".into()));
    }
    let mut input = archive.by_name(&entry_name.source_name).map_err(|error| {
        LibraryError::Message(format!(
            "ZIP内の {} を読めません: {error}",
            entry_name.source_name
        ))
    })?;
    let output = File::create(destination)?;
    let mut output = BufWriter::new(output);
    let byte_size = std::io::copy(&mut input, &mut output)?;
    output.flush()?;
    Ok(byte_size)
}

pub(crate) fn write_directory_archive<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    group_name: &str,
    entries: &[LegacyArchiveEntry],
    destination: &Path,
) -> LibraryResult<u64> {
    let output = File::create(destination)?;
    let mut writer = ZipWriter::new(BufWriter::new(output));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let prefix = format!("{group_name}/");
    for entry_name in entries {
        let Some(archived_name) = entry_name.normalized_name.strip_prefix(&prefix) else {
            continue;
        };
        if archived_name.is_empty() {
            continue;
        }
        let mut entry = archive.by_name(&entry_name.source_name).map_err(|error| {
            LibraryError::Message(format!(
                "ZIP内の {} を読めません: {error}",
                entry_name.source_name
            ))
        })?;
        if entry.is_dir() {
            writer
                .add_directory(archived_name, options)
                .map_err(|error| LibraryError::Message(format!("ZIPを作成できません: {error}")))?;
        } else {
            writer
                .start_file(archived_name, options)
                .map_err(|error| LibraryError::Message(format!("ZIPを作成できません: {error}")))?;
            std::io::copy(&mut entry, &mut writer)?;
        }
    }
    let output = writer
        .finish()
        .map_err(|error| LibraryError::Message(format!("ZIPを作成できません: {error}")))?;
    output.get_ref().sync_all()?;
    Ok(fs::metadata(destination)?.len())
}

pub(crate) fn parse_legacy_asset_info(contents: &str) -> (Option<String>, Option<String>) {
    let mut lines = contents.lines();
    let first_line = lines.next().unwrap_or_default().trim();
    let product_url = if first_line.starts_with("https://") || first_line.starts_with("http://") {
        Some(first_line.to_owned())
    } else {
        None
    };
    let note = lines.collect::<Vec<_>>().join("\n");
    (product_url, (!note.trim().is_empty()).then_some(note))
}

pub(crate) fn copy_staged_asset(
    staging_asset_dir: &Path,
    asset_directory: &Path,
    files: &[ImportedFile],
    cover_filename: Option<&str>,
) -> LibraryResult<()> {
    let destination_files = asset_directory.join("files");
    fs::create_dir_all(&destination_files)?;
    for file in files {
        fs::copy(
            staging_asset_dir.join("files").join(&file.stored_filename),
            destination_files.join(&file.stored_filename),
        )?;
    }
    if let Some(cover_filename) = cover_filename {
        let staged_cover = staging_asset_dir.join(cover_filename);
        if staged_cover.is_file() {
            fs::copy(staged_cover, asset_directory.join(cover_filename))?;
        }
    }
    fs::remove_dir_all(staging_asset_dir)?;
    Ok(())
}

use super::*;
