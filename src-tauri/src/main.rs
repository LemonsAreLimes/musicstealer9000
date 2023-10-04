// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod spotify_api;
mod downloader;
mod static_types;
use tokio::sync::Mutex;

fn main() {

    let downloader = Mutex::new(spotify_api::Downloader::new());

    tauri::Builder::default()
        .manage(downloader)
        .invoke_handler(tauri::generate_handler![      
            config::get_playlist,
            config::set_playlist,
            config::remove_playlist,
            config::ytdlp_check,
            config::get_config,
            config::write_config_from_string,
            spotify_api::get_token,
            spotify_api::check_link,
            spotify_api::playlist_url_to_id,
            spotify_api::start_download,
            spotify_api::stop_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}
