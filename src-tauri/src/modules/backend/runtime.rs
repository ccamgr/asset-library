use super::*;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::load(app.handle())
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            state.log("application started");
            let library_root = default_library_path(app.handle())
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            let asset_storage_root = state
                .settings
                .lock()
                .map_err(|_| LibraryError::Message("設定のロックに失敗しました".into()))?
                .asset_storage_path
                .clone()
                .unwrap_or_else(|| library_root.join("asset-storage"));
            allow_cover_paths(app.handle(), &library_root, &asset_storage_root)
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            app.manage(state);
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| LibraryError::Message("メインウィンドウを取得できません".into()))?;
            let state = app.state::<AppState>();
            restore_window_state(&window, &state)
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            window
                .show()
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, WindowEvent::CloseRequested { .. }) {
                save_window_state(window);
            }
        })
        .invoke_handler(handler())
        .run(tauri::generate_context!())
        .expect("error while running VR Asset Library");
}
