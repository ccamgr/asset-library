use super::*;

pub(super) fn thumbnail_cache_path(library_root: &Path, asset_id: &str) -> LibraryResult<PathBuf> {
    validated_asset_id(asset_id)?;
    Ok(library_root
        .join("thumbnail-cache")
        .join(format!("{asset_id}.jpg")))
}

pub(super) fn cached_thumbnail_path(
    library_root: &Path,
    asset_id: &str,
) -> LibraryResult<Option<String>> {
    let cache_path = thumbnail_cache_path(library_root, asset_id)?;
    if !cache_path.is_file() {
        return Ok(None);
    }
    let cache_is_valid = image::image_dimensions(&cache_path).is_ok();
    if !cache_is_valid {
        let _ = fs::remove_file(&cache_path);
        return Ok(None);
    }
    Ok(Some(display_path(&cache_path)))
}

pub(super) fn generate_cached_thumbnail(
    library_root: &Path,
    asset_storage_root: &Path,
    asset_id: &str,
    cover_path: &str,
) -> LibraryResult<()> {
    let source = asset_storage_root.join(cover_path);
    if !source.is_file() {
        return Err(LibraryError::Message("カバー画像が見つかりません".into()));
    }
    let image = image::open(&source)
        .map_err(|error| LibraryError::Message(format!("カバー画像を読み取れません: {error}")))?;
    let thumbnail = DynamicImage::ImageRgb8(image.thumbnail(480, 360).to_rgb8());
    let cache_path = thumbnail_cache_path(library_root, asset_id)?;
    let parent = cache_path
        .parent()
        .ok_or_else(|| LibraryError::Message("サムネイル保存先が不正です".into()))?;
    fs::create_dir_all(parent)?;
    let temporary = cache_path.with_extension("jpg.partial");
    let result = (|| -> LibraryResult<()> {
        let output = File::create(&temporary)?;
        let mut encoder = JpegEncoder::new_with_quality(BufWriter::new(output), 72);
        encoder.encode_image(&thumbnail).map_err(|error| {
            LibraryError::Message(format!("サムネイルを作成できません: {error}"))
        })?;
        fs::rename(&temporary, &cache_path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(super) fn schedule_thumbnail_generation(
    app: &AppHandle,
    state: &AppState,
    locations: &StorageLocations,
    asset_id: &str,
    cover_path: &str,
) {
    let Ok(mut job_ids) = state.thumbnail_job_ids.lock() else {
        return;
    };
    if !job_ids.insert(asset_id.to_owned()) {
        return;
    }
    drop(job_ids);
    if let Ok(mut jobs) = state.thumbnail_jobs.lock() {
        jobs.push_back(ThumbnailJob {
            library_root: locations.library_root.clone(),
            asset_storage_root: locations.asset_storage_root.clone(),
            asset_id: asset_id.to_owned(),
            cover_path: cover_path.to_owned(),
        });
    } else {
        let _ = state
            .thumbnail_job_ids
            .lock()
            .map(|mut ids| ids.remove(asset_id));
        return;
    }
    if state.thumbnail_worker_running.swap(true, Ordering::AcqRel) {
        return;
    }
    run_thumbnail_worker(
        app.clone(),
        Arc::clone(&state.thumbnail_jobs),
        Arc::clone(&state.thumbnail_job_ids),
        Arc::clone(&state.thumbnail_worker_running),
    );
}

pub(super) fn run_thumbnail_worker(
    app: AppHandle,
    jobs: Arc<Mutex<VecDeque<ThumbnailJob>>>,
    job_ids: Arc<Mutex<HashSet<String>>>,
    running: Arc<AtomicBool>,
) {
    std::thread::spawn(move || loop {
        let job = jobs.lock().ok().and_then(|mut jobs| jobs.pop_front());
        let Some(job) = job else {
            running.store(false, Ordering::Release);
            if jobs.lock().is_ok_and(|jobs| !jobs.is_empty())
                && !running.swap(true, Ordering::AcqRel)
            {
                continue;
            }
            break;
        };
        let generated = generate_cached_thumbnail(
            &job.library_root,
            &job.asset_storage_root,
            &job.asset_id,
            &job.cover_path,
        )
        .is_ok();
        if let Ok(mut ids) = job_ids.lock() {
            ids.remove(&job.asset_id);
        }
        if generated {
            let _ = app.emit("thumbnail-cache-updated", &job.asset_id);
        }
    });
}

pub(super) fn remove_cached_thumbnail(library_root: &Path, asset_id: &str) {
    if let Ok(path) = thumbnail_cache_path(library_root, asset_id) {
        let _ = fs::remove_file(path);
    }
}
