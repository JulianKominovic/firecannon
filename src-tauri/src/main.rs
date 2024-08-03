// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod events;
pub mod utils;

use events::{fire::fire, greet};
use specta::{collect_types, ts::ExportConfiguration};
use tauri::Manager;
use tauri_specta::ts;

fn main() {
    let specta_config = ExportConfiguration::new().bigint(specta::ts::BigIntExportBehavior::Number);
    #[cfg(debug_assertions)]
    ts::export_with_cfg(
        collect_types![greet, fire].unwrap(),
        specta_config,
        "../src/bindings.ts",
    )
    .unwrap();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, fire])
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window.open_devtools();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
