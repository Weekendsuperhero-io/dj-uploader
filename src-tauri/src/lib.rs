pub mod artwork;
pub mod audio;
pub mod cli;
pub mod commands;
pub mod config;
pub mod platforms;

use tauri::{LogicalSize, Manager};

const IDEAL_WINDOW_WIDTH: f64 = 620.0;
const IDEAL_WINDOW_HEIGHT: f64 = 820.0;
const WINDOW_MARGIN: f64 = 24.0;

/// Build and run the Tauri desktop application (the GUI entry point).
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                // Keep one fixed window size for the current display. On a
                // smaller work area, choose a smaller fixed viewport; the web
                // UI scales itself to that viewport.
                if let Some(monitor) = window.current_monitor()? {
                    let scale_factor = monitor.scale_factor();
                    let work_area = monitor.work_area();
                    let available_width =
                        work_area.size.width as f64 / scale_factor - WINDOW_MARGIN * 2.0;
                    let available_height =
                        work_area.size.height as f64 / scale_factor - WINDOW_MARGIN * 2.0;
                    let width = IDEAL_WINDOW_WIDTH.min(available_width.max(320.0));
                    let height = IDEAL_WINDOW_HEIGHT.min(available_height.max(480.0));
                    window.set_size(LogicalSize::new(width, height))?;
                }
                window.set_resizable(false)?;
                window.center()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_auth_status,
            commands::connect_platform,
            commands::disconnect_platform,
            commands::upload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DJ Uploader");
}
