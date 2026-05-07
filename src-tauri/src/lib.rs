mod clipboard;
mod enhance;
mod hotkey;
mod settings;
mod status_window;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Ok(path) = dotenvy::dotenv() {
        println!("[env] loaded {}", path.display());
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            settings::api_key_status,
            settings::save_api_key,
            settings::clear_api_key,
            settings::get_hotkey,
            settings::save_hotkey,
            settings::test_connection,
            settings::open_settings,
        ])
        .setup(|app| {
            let user_settings = settings::load(app.handle());
            tray::build(app.handle())?;
            hotkey::register(app.handle(), &user_settings.hotkey)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
