// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod spotify_api;


fn main() {

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            config::set_thread_count,
            config::set_credentials,
            config::get_thread_count,       
            config::get_playlist,
            config::set_playlist,
            config::remove_playlist,
            spotify_api::get_token,
            spotify_api::check_link,
            spotify_api::start_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}
