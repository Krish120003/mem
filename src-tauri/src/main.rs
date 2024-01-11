// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use entity::capture;
use migration::{Migrator, MigratorTrait};
use sea_orm::ActiveModelTrait;
use sea_orm::{Database, DatabaseConnection, Set, TryIntoModel};
use tauri::{CustomMenuItem, Manager, RunEvent, SystemTray, SystemTrayMenu, SystemTrayMenuItem};

use screenshots::Screen;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{self, Utc};
use std::time::Duration;

use tokio::task;
use tokio::time::sleep;

#[derive(Clone)]
struct AppState {
    conn: DatabaseConnection,
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tokio::main]
async fn main() {
    let conn: DatabaseConnection = Database::connect("sqlite:///tmp/mem.sqlite?mode=rwc")
        .await
        .expect("Database connection failed");

    let handle_conn = conn.clone();

    Migrator::up(&conn, None).await.unwrap();

    let state = AppState { conn };

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
                        // TODO: Add metadata to image

                        let im_model = capture::ActiveModel {
                            path: Set(img_path_str.to_owned()),
                            timestamp: Set(current_time.to_owned().to_rfc3339()),
                            ..Default::default()
                        };

                        im_model.insert(&handle_conn).await.unwrap();
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
    let view_tray_item = CustomMenuItem::new("view".to_string(), "View History");
    let quit_tray_item = CustomMenuItem::new("quit".to_string(), "Quit");

    // this is very hacky way to do pause/resume
    // waiting for tauri 2.0 to support dynamic menu

    let tray_menu = SystemTrayMenu::new()
        .add_item(toggle_tray_item)
        .add_item(view_tray_item)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit_tray_item);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    let app = tauri::Builder::default()
        .manage(state)
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
                    "view" => {
                        println!("View");
                        app.get_window("viewer").unwrap().show().unwrap();
                        app.get_window("viewer").unwrap().set_focus().unwrap();
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    let viewer_window = tauri::WindowBuilder::new(
        &app,
        "viewer", /* the unique window label */
        tauri::WindowUrl::App("index.html".into()),
    )
    .build()
    .unwrap();

    viewer_window.hide().unwrap();

    app.run(|_app_handle, e| match e {
        // Keep the event loop running even if all windows are closed
        // This allow us to catch system tray events when there is no window
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    });
}
