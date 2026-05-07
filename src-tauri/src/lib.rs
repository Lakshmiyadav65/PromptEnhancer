mod clipboard;
mod enhance;
mod hotkey;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from project root (or any ancestor of the cwd) before reading env vars.
    // Failure is non-fatal — env vars set in the shell still work.
    if let Ok(path) = dotenvy::dotenv() {
        println!("[env] loaded {}", path.display());
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            tray::build(app.handle())?;
            hotkey::register(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
