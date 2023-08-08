// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod spotify_api;
// use std::sync::Mutex;
use tokio::sync::Mutex;

fn main() {

    let downloader = Mutex::new(spotify_api::Downloader::new());

    tauri::Builder::default()
        .manage(downloader)
        .invoke_handler(tauri::generate_handler![
            config::set_thread_count,
            config::set_credentials,
            config::get_thread_count,       
            config::get_playlist,
            config::set_playlist,
            config::remove_playlist,
            spotify_api::get_token,
            spotify_api::check_link,
            spotify_api::start_download,
            spotify_api::stop_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}
