use std::{
    fs, 
    fs::File,
    io::Write,
    process::Command,
};
use serde_json::{self};
use dirs;

use super::static_types::{
    Config,
    Id3Options,
    Playlist
};

fn check_for_config() -> bool {
    if let Ok(current_dir) = std::env::current_dir() {
        let config_path = current_dir.join("config.json");
        match fs::metadata(&config_path) {
            Ok(_) => { true }
            Err(_) => { create_config(); true }
        }
    } else {
        create_config();
        true
    }
}

fn create_config(){
    let file_path = std::env::current_dir().expect("err").join("config.json");
    let mut file = File::create(&file_path).expect("could not create config file");
    
    //get desktop, witch is used as the default download dir "/music/PLAYLISTNAME"
    let mut default_dir;
    if let Some(dir) = dirs::desktop_dir(){ 
        default_dir = dir
    }
    else if let Some(dir) = dirs::home_dir(){ 
        default_dir = dir
    } else {

        //fallback incase the desktop and home could not be found 
        default_dir = std::env::current_dir()
            .expect("could not get local dir")
    }

    default_dir.push("music");

    //check if download dir exists, create it if not
    if !default_dir.exists(){
        let _ = fs::create_dir_all(&default_dir);
    }

    let default_config = serde_json::to_string(&Config {
        client_id: "".to_string(),
        client_secret: "".to_string(),
        thread_count: 2,
        playlists: Vec::new(),
        image_delete: true,
        image_download: true,
        download_dir: default_dir,
        download_source: "ytsearch".to_string(),
        audio_format: "mp3".to_string(),
        id3_options: Id3Options {
            artist: true,
            year: true,
            album: true,
            track_number: true,
            genre: true,
        },
    }).expect("could not parse config file to writeable format");

    let _ = file.write_all(default_config.as_bytes());
}

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    check_for_config();

    //config should exist beyond this point.

    if let Ok(current_dir) = std::env::current_dir() {

        //get the config file
        let config_path = current_dir.join("config.json");
        
        //parse it out
        let content = fs::read_to_string(&config_path).expect("msg");
        let config: Config = serde_json::from_str(&content).map_err(|err| format!("Error parsing JSON: {}", err))?;
        println!("{:?}", config);
        Ok(config)

    } else { 
        Err("could not get config".to_string())
    }
}

#[tauri::command]
pub fn write_config_from_string(new_config: String) -> Result<(), ()>{
    let config: Config = serde_json::from_str(&new_config).unwrap();
    
    write_config(config);
    Ok(())
}

fn write_config(new_config: Config){
    if let Ok(current_dir) = std::env::current_dir() {

        //get the config file
        let config_path = current_dir.join("config.json");
        
        //write to it
        let json_str = serde_json::to_string_pretty(&new_config).expect("could not parse json");
        fs::write(config_path, json_str).expect("could not write to config file");
        
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn set_playlist(playlistId: String, token: &str) -> Result<String, String>{
    let url_parsed = playlistId
        .replace("https://open.spotify.com/playlist/", "");
    
    let data = super::spotify_api::get_playlist_data(url_parsed.to_string(), token).await;
    match data { 
        Ok(data) => {

            let new_playlist = Playlist {
                name: data["name"].to_string(),
                id: url_parsed,
                image_url: data["images"][0]["url"].to_string(),
                download_dir: "Default".to_string()
            };

            match get_config() {
                Ok(mut config) => { 
                    config.playlists.push(new_playlist);                    
                    write_config(config);
                    return Ok("Playlist updated successfully".to_string());
                }
                Err(_) => return Err("couldn't write config".to_string())
            }
        }
        Err(_) => return Err("uhh".to_string())
    }
}

#[tauri::command]
pub fn get_playlist() -> Result<Vec<Playlist>, String>{
    match get_config() {
        Ok(config) => {
            return Ok(config.playlists)
        }
        Err(err) => { println!("err {:?}", err); return Err("uhh".to_string())}  
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn remove_playlist(playlistId: String){
    match get_config() {
        Ok(mut config) => {
            for playlist_idx in 0..config.playlists.len() { 
                if config.playlists[playlist_idx].id == playlistId{
                    config.playlists.remove(playlist_idx);
                    write_config(config);
                    return;
                }
            }

        }
        Err(_) => return
    }
} 


//TODO: expand this to include more install types, linux and outhers
#[tauri::command]
pub fn ytdlp_check() -> Result<String, ()> {

    //check if ytdlp is installed globally
    let cmd: Result<std::process::ExitStatus, std::io::Error> = Command::new("yt-dlp").status();
    let is_global = match cmd { 
        Ok(_) => true,
        Err(_) => false,
    };

    if is_global { return Ok("global".to_string()); }

    let curr_dir = std::env::current_dir()
        .expect("could not get current directory")
        .to_owned().to_string_lossy().to_string();

    let yt_dlp_location = format!("{}\\yt-dlp.exe", curr_dir);
    println!("checking: {:?}", yt_dlp_location);
    let cmd = Command::new(yt_dlp_location).status();
    match cmd { 
        Ok(_) => return Ok("local".to_string()),
        Err(_) => {return Err(())}
    }

}