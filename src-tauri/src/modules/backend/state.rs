use super::*;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AppSettings {
    pub(super) asset_storage_path: Option<PathBuf>,
    pub(super) database_backup_path: Option<PathBuf>,
    pub(super) database_path: Option<PathBuf>,
    #[serde(default)]
    pub(super) window_state: Option<WindowState>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WindowState {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    maximized: bool,
}

pub(super) struct AppState {
    config_path: PathBuf,
    pub(super) log_path: PathBuf,
    pub(super) settings: Mutex<AppSettings>,
    pub(super) legacy_import_running: Arc<AtomicBool>,
    pub(super) thumbnail_jobs: Arc<Mutex<VecDeque<ThumbnailJob>>>,
    pub(super) thumbnail_job_ids: Arc<Mutex<HashSet<String>>>,
    pub(super) thumbnail_worker_running: Arc<AtomicBool>,
}

impl AppState {
    pub(super) fn load(app: &AppHandle) -> LibraryResult<Self> {
        let config_dir = app.path().app_local_data_dir().map_err(|error| {
            LibraryError::Message(format!("設定フォルダを取得できません: {error}"))
        })?;
        fs::create_dir_all(&config_dir)?;
        let log_directory = config_dir.join(LOG_DIRECTORY);
        fs::create_dir_all(&log_directory)?;

        let config_path = config_dir.join(SETTINGS_FILE);
        let settings = match fs::read_to_string(&config_path) {
            Ok(contents) => serde_json::from_str(&contents)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => AppSettings::default(),
            Err(error) => return Err(error.into()),
        };

        Ok(Self {
            config_path,
            log_path: log_directory.join(LOG_FILE),
            settings: Mutex::new(settings),
            legacy_import_running: Arc::new(AtomicBool::new(false)),
            thumbnail_jobs: Arc::new(Mutex::new(VecDeque::new())),
            thumbnail_job_ids: Arc::new(Mutex::new(HashSet::new())),
            thumbnail_worker_running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub(super) fn save_settings(&self, settings: &AppSettings) -> LibraryResult<()> {
        let contents = serde_json::to_vec_pretty(settings)?;
        let temporary_path = self.config_path.with_extension("json.tmp");
        fs::write(&temporary_path, contents)?;
        fs::rename(temporary_path, &self.config_path)?;
        Ok(())
    }

    pub(super) fn log(&self, event: impl AsRef<str>) {
        append_log(&self.log_path, event.as_ref());
    }
}

pub(super) fn append_log(path: &Path, event: &str) {
    let sanitized = event.replace(['\r', '\n'], " ");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}\t{}", unix_timestamp(), sanitized);
    }
}

pub(super) fn restore_window_state(window: &WebviewWindow, state: &AppState) -> LibraryResult<()> {
    let window_state = state
        .settings
        .lock()
        .map_err(|_| LibraryError::Message("設定のロックに失敗しました".into()))?
        .window_state
        .clone();

    let Some(window_state) = window_state else {
        return Ok(());
    };
    if window_state.width == 0 || window_state.height == 0 {
        return Ok(());
    }

    window
        .set_size(PhysicalSize::new(window_state.width, window_state.height))
        .map_err(|error| {
            LibraryError::Message(format!("ウィンドウサイズを復元できません: {error}"))
        })?;
    let center_x = window_state
        .x
        .saturating_add((window_state.width / 2).min(i32::MAX as u32) as i32);
    let center_y = window_state
        .y
        .saturating_add((window_state.height / 2).min(i32::MAX as u32) as i32);
    if window
        .monitor_from_point(center_x as f64, center_y as f64)
        .map_err(|error| LibraryError::Message(format!("画面を確認できません: {error}")))?
        .is_some()
    {
        window
            .set_position(PhysicalPosition::new(window_state.x, window_state.y))
            .map_err(|error| {
                LibraryError::Message(format!("ウィンドウ位置を復元できません: {error}"))
            })?;
    }
    if window_state.maximized {
        window.maximize().map_err(|error| {
            LibraryError::Message(format!("ウィンドウを最大化できません: {error}"))
        })?;
    }
    Ok(())
}

pub(super) fn save_window_state(window: &Window) {
    let Ok(size) = window.outer_size() else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(maximized) = window.is_maximized() else {
        return;
    };
    let Some(state) = window.app_handle().try_state::<AppState>() else {
        return;
    };
    let Ok(mut settings) = state.settings.lock() else {
        return;
    };
    settings.window_state = Some(WindowState {
        width: size.width,
        height: size.height,
        x: position.x,
        y: position.y,
        maximized,
    });
    if let Err(error) = state.save_settings(&settings) {
        eprintln!("failed to save window state: {error}");
    }
}
