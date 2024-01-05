// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[allow(warnings, unused)]
mod prisma;

use prisma::*;

use tauri::{CustomMenuItem, RunEvent, SystemTray, SystemTrayMenu, SystemTrayMenuItem};

use screenshots::Screen;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tokio::task;
use tokio::time::sleep;

use chrono::{self, Utc};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tokio::main]
async fn main() {
    // create db connection
    let db = PrismaClient::_builder().build().await.unwrap();

    let screencap_active = Arc::new(Mutex::new(true));
    let screencap_active_handle: Arc<Mutex<bool>> = Arc::clone(&screencap_active);

    let _handle = task::spawn(async move {
        loop {
            // thread::sleep(Duration::from_secs(2));
            sleep(Duration::from_secs(2)).await;
            {
                let is_active = screencap_active_handle.lock().unwrap();
                if !*is_active {
                    continue;
                }
            }

            let primary_screen = Screen::all().unwrap()[0];

            let image = primary_screen.capture().unwrap();
            let now = chrono::offset::Utc::now();

            let img_path_str = format!(
                "/tmp/target/{}_{}.jpeg",
                primary_screen.display_info.id, now
            );
            let img_path = Path::new(img_path_str.as_str());

            let current_time: chrono::prelude::DateTime<Utc> = Utc::now();

            match fs::create_dir_all(img_path.parent().expect("Invalid parent directory")) {
                Ok(_) => match image.save(&img_path) {
                    Ok(_) => {
                        println!("Saved image to {}", img_path_str);
                        let db_result = db
                            .capture()
                            .create(img_path_str, current_time.into(), vec![])
                            .exec()
                            .await
                            .unwrap();

                        println!("id: {}", db_result.id);
                    }
                    Err(err) => {
                        eprintln!("Error saving image: {}", err);
                    }
                },
                Err(err) => {
                    eprintln!("Error creating directory: {}", err);
                }
            }
        }
    });

    let toggle_tray_item = CustomMenuItem::new("toggle".to_string(), "Pause/Resume");
    let quit_tray_item = CustomMenuItem::new("quit".to_string(), "Quit");

    // this is very hacky way to do pause/resume
    // waiting for tauri 2.0 to support dynamic menu

    let tray_menu = SystemTrayMenu::new()
        .add_item(toggle_tray_item)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit_tray_item);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    let mut app = tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(move |app, event| {
            match event {
                tauri::SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "quit" => {
                        // app.quit();
                        println!("Quit");
                        app.exit(0);
                    }
                    "toggle" => {
                        println!("Toggle");
                        let mut screencap_ac = screencap_active.lock().unwrap();
                        *screencap_ac = !*screencap_ac;

                        if *screencap_ac {
                            println!("Resuming");
                        } else {
                            println!("Pausing");
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
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
