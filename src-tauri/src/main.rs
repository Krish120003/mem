// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{CustomMenuItem, RunEvent, SystemTray, SystemTrayMenu, SystemTrayMenuItem};

use screenshots::Screen;
use std::thread;
use std::time::Duration;
use std::time::Instant;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    let handle = thread::spawn(|| loop {
        let primary_screen = Screen::all().unwrap()[0];

        let mut image = primary_screen.capture().unwrap();
        let now = Instant::now();

        let img_path = format!(
            "/tmp/target/{}_{}.jpeg",
            primary_screen.display_info.id,
            now.elapsed().as_secs()
        );

        image.save(&img_path).unwrap();

        thread::sleep(Duration::from_millis(2000));
    });

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let tray_menu = SystemTrayMenu::new()
        .add_item(quit)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(hide);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    let mut app = tauri::Builder::default()
        .system_tray(system_tray)
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_app_handle, e| match e {
        // Keep the event loop running even if all windows are closed
        // This allow us to catch system tray events when there is no window
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    });
}
