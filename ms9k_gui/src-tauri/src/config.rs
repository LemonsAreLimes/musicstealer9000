use std::{
    fs, 
    fs::File,
    io::{ Write, Cursor },
    process::Command,
};
use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;    //for downloading ytdlp

#[derive(Debug, Deserialize, Serialize)]
pub struct Playlist {
    pub name: String,
    pub id: String,
    pub download_dir: String,
    pub image_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub thread_count: u32,
    pub playlists: Vec<Playlist>
}


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
    
    let default_config = serde_json::to_string(&Config {
        client_id: "".to_string(),
        client_secret: "".to_string(),
        thread_count: 2,
        playlists: Vec::new(),
    }).expect("could not parse config file to writeable format");

    let _ = file.write_all(default_config.as_bytes());
}

pub fn get_config() -> Result<Config, String> {
    check_for_config();
    if let Ok(current_dir) = std::env::current_dir() {

        //get the config file
        let config_path = current_dir.join("config.json");
        
        //parse it out
        let content = fs::read_to_string(&config_path).expect("msg");
        let config: Config = serde_json::from_str(&content).map_err(|err| format!("Error parsing JSON: {}", err))?;
        println!("{:?}", config);
        Ok(config)
    } else { 
        Err("could not find config file".to_string())
    }
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


#[tauri::command]
pub fn get_thread_count() -> String {
    match get_config() {
        Ok(config) => config.thread_count.to_string(),
        Err(_) => "UNDEFINED".to_string(),
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn set_thread_count(threadCount: u32){
    println!("{}", threadCount);
    match get_config() {
        Ok(mut config) => {
            config.thread_count = threadCount;
            write_config(config)
        },
        Err(_) => { println!("err"); return } 
    };
}


#[allow(non_snake_case)]
#[tauri::command]
pub fn set_credentials( clientId: &str, clientSecret: &str){
    match get_config() { 
        Ok(mut config) => {
            config.client_id = clientId.to_string();
            config.client_secret = clientSecret.to_string();
            write_config(config);
        }
        Err(_) => { return }
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


#[tauri::command]
pub async fn download_ytdlp(){
    let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
    let response = reqwest::get(url).await.expect("couldn't get download url");
    let mut file = std::fs::File::create("yt-dlp.exe").expect("could not create yt-dlp file");
    let mut content =  Cursor::new(response.bytes().await.expect("couldn't write to yt-dlp"));
    println!("{:?}", content);
    let _ = std::io::copy(&mut content, &mut file);
}

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
