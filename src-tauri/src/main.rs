// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod events;
pub mod utils;

use events::fire::{fire, ResponseMetrics};
use specta::{collect_types, ts::ExportConfiguration, TypeDefs};
use std::fs::{self, OpenOptions};
use std::io::Write;
use tauri::Manager;
use tauri_specta::ts;

fn main() {
    #[cfg(debug_assertions)]
    {
        let specta_config =
            ExportConfiguration::new().bigint(specta::ts::BigIntExportBehavior::Number);
        let response_metrics_type = specta::ts::export::<ResponseMetrics>(&specta_config).unwrap();

        ts::export_with_cfg(
            collect_types![fire].unwrap(),
            specta_config,
            "../src/bindings.ts",
        )
        .unwrap();

        let mut output_file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("../src/bindings.ts")
            .unwrap();
        writeln!(output_file, "{}", response_metrics_type).unwrap();
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![fire])
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window.open_devtools();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
