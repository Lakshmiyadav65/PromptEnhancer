use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};

mod clipboard;
mod enhance;
mod hotkey;
mod settings;
mod status_window;
mod tray;
mod updater;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Ok(path) = dotenvy::dotenv() {
        println!("[env] loaded {}", path.display());
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            settings::api_key_status,
            settings::save_api_key,
            settings::clear_api_key,
            settings::get_hotkey,
            settings::save_hotkey,
            settings::test_connection,
            settings::open_settings,
            updater::check_for_updates,
        ])
        .setup(|app| {
            let user_settings = settings::load(app.handle());
            tray::build(app.handle())?;
            hotkey::register(app.handle(), &user_settings.hotkey)?;
            maybe_show_settings_on_first_run(app.handle(), &user_settings);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// First-run UX: if there is no API key in either the GROQ_API_KEY env var or
/// the saved settings.json, auto-open the Settings window so the user can paste
/// one without having to discover the tray menu.
fn maybe_show_settings_on_first_run<R: Runtime>(
    app: &AppHandle<R>,
    user_settings: &settings::UserSettings,
) {
    let has_env_key = std::env::var("GROQ_API_KEY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let has_saved_key = user_settings
        .api_key
        .as_ref()
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);

    if has_env_key || has_saved_key {
        return;
    }

    let Some(window) = app.get_webview_window("settings") else {
        return;
    };

    println!("[onboarding] no API key found — auto-showing Settings window");
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        // Tiny delay so the rest of the app is ready (tray icon visible, etc.)
        tokio::time::sleep(Duration::from_millis(400)).await;
        let _ = window.show();
        let _ = window.set_focus();
    });
}
