// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[allow(warnings, unused)]
mod prisma;

use prisma::*;

use screenshots::image::flat::ViewMut;
use tauri::{
    CustomMenuItem, Manager, RunEvent, State, SystemTray, SystemTrayMenu, SystemTrayMenuItem,
};

use screenshots::Screen;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tokio::task;
use tokio::time::sleep;

use chrono::{self, Utc};

use specta::Type;

type DbState<'a> = State<'a, Arc<PrismaClient>>;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
#[tauri::command]
#[specta::specta]
fn get_image_ranges(db: DbState<'_>) -> (usize, usize) {
    return (0, 0);
}

#[tokio::main]
async fn main() {
    // create db connection
    let db = PrismaClient::_builder().build().await.unwrap();

    #[cfg(debug_assertions)]
    ts::export(collect_types![get_posts, create_post], "../src/bindings.ts").unwrap();

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
