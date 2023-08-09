use std::{
    process::Command,
    path::Path,
    fs,
    sync::{ Arc, Mutex },
};
use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use reqwest::header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE };
use json::JsonValue;
use rand::Rng;
use image;
use id3::{ Tag, TagLike, Version };
use id3_image::embed_image;
use dirs;
use tauri::State;

#[derive(Debug, Deserialize)]
struct AcsessTokenResponse {
    access_token: String,
}

#[derive(Clone, serde::Serialize, Debug)]
struct GenericUpdate { 
    id: i32,
    text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Song { 
    video_id: i32,
    name: String,
    image_url: String,
    track_number: u32,
    artist: String,
    album: String,
    album_artist: String,
    year: i32,
    genre: String,
}

#[derive(Debug)]
pub struct Downloader {
    window: Option<tauri::Window>,
    download_dir: String,
    token: String,
    playlist_id: String,
    data: Vec<Song>,
    thread_count: u32,
    threads: Option<Vec<tokio::task::JoinHandle<()>>>,
    yt_dlp_install_type: String,
} 

async fn edit_id3(track: &Song, track_path: &str, image_path: &str){

    let formatted_track_path = format!("{}/{}.mp3", track_path, track.video_id);
    let track_full_path = Path::new(&formatted_track_path);
    let image_full_path = Path::new(image_path);
    println!("IMG PATH: {:?}", &image_full_path);
    println!("TRACK FULL: {:?}", &track_full_path);

    //set the tags
    let mut tag = Tag::new();
    tag.set_title(&track.name);
    tag.set_artist(&track.artist);
    tag.set_album(&track.album);
    tag.set_album_artist(&track.album_artist);
    tag.set_year(track.year);
    tag.set_genre(&track.genre);
    tag.set_track(track.track_number);
    tag.write_to_path(track_full_path, Version::Id3v24).expect("failed to write id3");
    embed_image(track_full_path, image_full_path).expect("could not set image");

    //strip charicters that are invalid for filenames, rename
    let mut file_name = track.name.to_owned();
    file_name = track_path.to_owned() +"/"+ &file_name.replace(&['>', '<', '/', '|', '?', '*'], "") + ".mp3";
    println!("FN: {:?}", file_name);
    fs::rename(track_full_path, file_name).expect("could not rename the mp3 file!");

    //remove the left over image
    fs::remove_file(image_full_path).expect("could not remove old image file!");


}
async fn download_audio(song: &Song, dir: &str, install_type: &str) -> bool{

    let mut yt_dlp_location = "yt-dlp".to_string();
    if install_type == "local" {
        //get our current dir
        let curr_dir = std::env::current_dir()
            .expect("could not get current directory")
            .to_owned().to_string_lossy().to_string();
        
        yt_dlp_location = format!("{}\\yt-dlp.exe", curr_dir);
    }

    //parse out yt-dlp arguments
    let search_arg = format!("ytsearch:'{}'", song.name.to_string() + " " + &song.artist);
    let dir_arg = format!("-P {}", dir);
    let name_arg = format!("-o{}", &song.video_id);

    //run yt-dlp
    println!("{:?}", yt_dlp_location);
    let mut cmd = Command::new(yt_dlp_location);
    let x = cmd.arg(search_arg)
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("mp3")
        .arg(dir_arg)
        .arg(name_arg)
        .status();
    // cmd.creation_flags(0x08000000);

    match x { 
        Ok(_) => return true,
        Err(e) => {println!("{:?}", e); return false}
    }

}
async fn get_image(image_url: &str, download_path: &str){
    let bytes = reqwest::get(image_url)
        .await
        .expect("could not download image")
        .bytes()
        .await
        .expect("could not parse image to bytes");
    let img = image::load_from_memory(&bytes).expect("could not load image");
    img.save(download_path).expect("could not save image");   
}
async fn download_process(window: tauri::Window, songs: Arc<Arc<Vec<Song>>>, dir: String, install_type: String) -> Result<(), String> {

    for song in songs.to_vec(){

        window.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "Starting".to_string()})
            .expect("could not send status update (START)");

        let image_path = format!("{}/{}.jpg", dir, song.video_id);
        println!("IMG PATH: {:?}", image_path);

        get_image(&song.image_url, &image_path).await; 
        window.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "Downloading".to_string()})
            .expect("could not send status update (IMAGE)");

        let download_succsess = download_audio(&song, &dir, &install_type).await;

        if !download_succsess { 
            window.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "THREAD CRASHED!!!".to_string()})
                .expect("could not send status update (AUDIO DOWNLOAD)");
            println!("THREAD CRASHED!!! ALL SONGS DELIGATED TO THIS THREAD WILL NOT BE DOWNLOADED!");
            continue;
        }

        window.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "Finishing up".to_string()})
            .expect("could not send status update (ID3 EDIT)");

        edit_id3(&song, &dir, &image_path).await;
        window.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "done!".to_string()})
            .expect("could not send status update (DONE)");

    }
    window.emit("threadDone", "")
        .expect("could not send status update (thread finished)");

    println!("THREAD DONE");
    Ok(())

}

impl Downloader {
    async fn get_genre(&mut self, track: &JsonValue) -> Option<String>{
        let artist_id = track["album"]["artists"][0]["id"].as_str().unwrap().to_string();

        //get genre
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/artists/{}", artist_id))
            .header(AUTHORIZATION, format!("Bearer {}", self.token)) 
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .send().await.unwrap();

        //make sure it went though
        if response.status() != 200 { println!("COULD NOT GET SONG GENRE"); return None} 
            
        //parse the response
        let res =  response.text().await.expect("could not parse GENRE response");
        let json_res = json::parse(&res).expect("could not parse GENRE json");

        //extract le genre
        let mut genre = "null".to_string();
        if !json::JsonValue::is_null(&json_res["genres"][0]) {
            genre = json_res["genres"][0].as_str().unwrap().to_string();
        }

        return Some(genre);

    }
    async fn parse_song_data(&mut self, track: &JsonValue) -> Song {

        //get album year
        let mut year = track["album"]["release_date"].as_str().unwrap().to_string();
        year.replace_range(4..year.len(), "");
        let int_year = year.parse::<i32>().unwrap();

        //get track artists
        let mut artists = track["artists"][0]["name"].as_str().unwrap().to_string();
        if track["artists"].len() > 0 {
            for i in 1..track["artists"].len() {
                let second_artist = ", ".to_string() + &track["artists"][i]["name"].as_str().unwrap().to_string();
                artists += &second_artist; 
            }
        }

        //get album artists
        let mut album_artists = track["album"]["artists"][0]["name"].as_str().unwrap().to_string();
        for i in 1..track["album"]["artists"].len() {
            let second_artist = ", ".to_string() + &track["album"]["artists"][i]["name"].as_str().unwrap().to_string();
            album_artists += &second_artist 
        }

        //get album name, set to single if single
        let mut album_name = "single".to_string();
        if track["album"]["album_type"].as_str().unwrap() != "single" {
            album_name = track["album"]["name"].as_str().unwrap().to_string();
        }
        
        //generate an id for the song
        let rng = rand::thread_rng().gen_range(0..1000000);

        //set the data
        let temp_song_data = Song {
            video_id: rng,
            name: track["name"].as_str().unwrap().to_string(),
            image_url: track["album"]["images"][0]["url"].as_str().unwrap().to_string(),
            track_number: track["track_number"].as_u32().unwrap(),
            artist: artists,
            album: album_name,
            album_artist: album_artists,
            year: int_year,
            genre: self.get_genre(track).await.unwrap(),
        };

        println!("SONG FOUND: {:?}", temp_song_data.name);

        //return it
        return temp_song_data

    }
    async fn get_batch_data(&mut self, offset: usize) -> Result<Vec<Song>, String>{

        //get the batch data
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/tracks?limit=100&offset={}", self.playlist_id, offset))
            .header(AUTHORIZATION, format!("Bearer {}", self.token)) 
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .send().await.unwrap();
    
        if response.status() != 200 { 
            println!("COULD NOT GET BATCH SONG DATA"); 
            return Err("COULD NOT GET BATCH SONG DATA".to_string())
        } 
        
        //parse the response to json
        let res =  response.text().await.expect("could not parse song data response");
        let json_res = json::parse(&res).expect("could not parse song data json");
        let res_len = json_res["items"].len();

        //parse out the relivant data
        let mut songs = Vec::new();
        for i in 0..res_len {
            let track = &json_res["items"][i]["track"];
            let new_song = self.parse_song_data(track).await;
            songs.push(new_song.to_owned());

            // each time this runs send out a signal to the client to show that this has completed 
            match &self.window { 
                Some(window) => {
                    window.emit("nameUpdate",  GenericUpdate{ id: new_song.video_id, text: new_song.name })
                        .expect("could not send name update");
                },
                None => {
                    println!("NO WINDOW");
                }
            }
        }

        return Ok(songs);
        
    }

    fn get_filenames(&self) -> Vec<String>{
        let mut file_names = Vec::new(); 

        for entry in fs::read_dir(&self.download_dir).expect("failed to read") {
            let entry = entry.expect("err reading entry");
            let os_string = entry.file_name();
            let filename = os_string.to_str().unwrap();
            let parsed_filename = str::replace(filename, ".mp3", "").to_string();

            println!("FOUND SONG ALREADY DOWNLOADED: {:?}", &parsed_filename);
            file_names.push(parsed_filename);
        }

        file_names

    }
    async fn get_playlist_length(&self) -> Result<usize, String>{
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/", self.playlist_id))
            .header(AUTHORIZATION, format!("Bearer {}", self.token)) 
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .send().await.unwrap();
    
        //make sure the request went through
        if response.status() != 200 { 
            println!("could not get playlist length, response code: {:?}", response.status());
            return Err("could not get playlist length".to_string())
        }
            
        //parse the response, get the length
        let res = response.text().await.expect("playlist data response could not be parsed");
        let json_res = json::parse(&res).expect("playlist data json could not be parsed");

        let length = &json_res["tracks"]["total"].as_usize().unwrap();
        return Ok(*length as usize);
    }
    async fn get_playlist_items(&mut self) -> Result<Vec<Song>, String> {

        //get all the iterms in the download dir
        let already_downloaded_songs = self.get_filenames();

        //get the length
        match self.get_playlist_length().await { 
            Ok(length) => {

                //collect all the items in the playlist
                let mut songs_collected = Vec::new();
                let mut offset: usize = 0;
                while offset < length +100 as usize {
                    let song_batch = self.get_batch_data(offset).await
                        .expect("get batch data failed");

                    if song_batch.is_empty() {
                        println!("SONG BATCH IS EMPTY");
                    }

                    println!("BATCH LEN: {:?}", &song_batch.len());

                    //check if we already have the song downloaded
                    for song in song_batch {
                        if already_downloaded_songs.contains(&song.name){
                            println!("soung already downloaded");
                            self.window.as_ref().expect("no window found").emit("statusUpdate", GenericUpdate {id: song.video_id, text: "done!".to_string()})
                                .expect("could not send status update (DONE)");
                        } else { 
                            songs_collected.push(song);
                            println!("soung NOT already downloaded")
                        }
                    }

                    offset += 100
                }

                println!("TOTAL COLLECTED LEN {:?}", songs_collected.len());
                return Ok(songs_collected)
            }
            Err(_) => {
                println!("PLAYLIST LENGTH ERROR");
                return Err("something went wrong with discovery".to_string());
            }
        }
    }

    pub async fn start_handler(&mut self) -> Result<(), ()> {

        //send event to client, disableing the download/stop button
        self.window.as_ref().expect("no window available")
            .emit("disableStop", "")
            .expect("could not emit event");

        //get all the tracks in the playlist
        let songs = self.get_playlist_items().await.expect("get_playlist_items failed");
        if songs.len() == 0 { 
            println!("evreythings already downloaded");
            return Ok(());
        }
        
        self.data = songs;

        //get an appropriate thread ammount
        let mut chunk_size = 1;
        if self.data.len() < self.thread_count as usize { 
            chunk_size = self.data.len();
            println!("songs less then playlist count")
        } else { 
            chunk_size = self.data.len() / self.thread_count as usize;
            println!("songs more then playlist count, using thread count")
        }

        //re-enable stop signal
        self.window.as_ref().expect("no window available")
            .emit("enableStop", "")
            .expect("could not emit event");

        //mutexify relavant variables
        let window_mutx = Arc::new(Mutex::new(&self.window));
        let dir_mutx = Arc::new(Mutex::new(&self.download_dir));
        let items_mutx = Arc::new(Mutex::new(&self.data));
        let install_type_mutx = Arc::new(Mutex::new(&self.yt_dlp_install_type));

        let chunks: Vec<_> = items_mutx.lock().unwrap().chunks(chunk_size)
            .map(|chunk| Arc::new(chunk.to_vec()))
            .collect();

        println!("DOWNLOAD STARTING");
        for chunk in chunks {
            let install_type_clone = install_type_mutx.lock().unwrap().clone();

            let chunk_mutx = Arc::new(chunk);
            let chunk_clone = chunk_mutx.clone();
            
            let dir_clone = dir_mutx.clone();
            let dir_useable = dir_clone.lock().unwrap().clone();

            let window_clone = window_mutx.clone();
            let window_useable = match window_clone.lock().unwrap().clone() {
                Some(window) => window,
                None => panic!("window not found")
            };

            let handle = tokio::spawn(async move {
                download_process(window_useable, chunk_clone, dir_useable, install_type_clone)
                    .await
                    .expect("download process failed");
            });
            
            if self.threads.is_none() { 
                let mut list = Vec::new();
                list.push(handle);
                self.threads = Some(list);
            } else { 
                self.threads.as_mut().unwrap().push(handle);
            }
            
        } 

        // self.stop_handler().await;
        Ok(())

    }
    pub async fn stop_handler(&mut self){
        match self.threads.as_mut() {
            Some(threads) => {
                for handle in threads { 
                    handle.abort();
                }
                self.window.as_ref().expect("window stuff")
                    .emit("downloadFinish", "")
                    .expect("no");

                self.download_dir = "".to_string();
                self.playlist_id = "".to_string();
                self.token = "".to_string(); 
            }
            None => {
                println!("no threads to stop")
            }
        } 
    }

    pub fn set_window(&mut self, window: tauri::Window){
        self.window = Some(window);
    }
    pub fn set_token(&mut self, token: String){
        self.token = token
    }
    pub fn set_url(&mut self, url: String){
        self.playlist_id = url.replace("https://open.spotify.com/playlist/", "")
    }
    pub async fn set_download_dir(&mut self) -> Result<(), String>{
        
        //get the playlist name
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/", self.playlist_id))
            .header(AUTHORIZATION, format!("Bearer {}", self.token)) 
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .send().await.unwrap();
    
        //make sure the request went through
        if response.status() != 200 { 
            println!("could not get playlist name, response code: {:?}", response.status());
            return Err("could not get playlist name".to_string())
        }
            
        //parse the response, get the name
        let res = response.text().await.expect("playlist data response could not be parsed");
        let json_res = json::parse(&res).expect("playlist data json could not be parsed");
        let playlist_name = &json_res["name"].to_string();

        //create the download dir
        let mut download_dir = dirs::desktop_dir().expect("COULD NOT FIND USERS DESKTOP");
        download_dir.push("music");
        download_dir.push(playlist_name);

        //check if the new download dir exists, create it if not
        if !download_dir.exists(){
            let _ = fs::create_dir_all(&download_dir);
        }

        self.download_dir = download_dir.to_owned().to_string_lossy().to_string();

        return Ok(())

    }
    pub fn set_config(&mut self) -> Result<(), String>{
        match super::config::get_config() {
            Ok(config) => {
                self.thread_count = config.thread_count;
                return Ok(())
            }
            Err(e) => {
                return Err(e)
            }
        }; 
    }
    pub fn set_install_type(&mut self){
        let install_type = super::config::ytdlp_check().expect("not installed");
        self.yt_dlp_install_type = install_type;
    }

    pub fn new() -> Self {
        return Downloader {
            window: None,
            download_dir: "".to_string(),
            token: "".to_string(),
            playlist_id: "".to_string(),
            data: Vec::new(),
            thread_count: 0,
            threads: None,
            yt_dlp_install_type: "".to_string(),
        };
    }

}

pub async fn get_playlist_data(playlist_id: String, token: &str)-> Result<JsonValue, String>{

    let client = reqwest::Client::new();
    let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/", playlist_id))
        .header(AUTHORIZATION, format!("Bearer {}", token)) 
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .send().await.unwrap();

    //make sure the request went through
    if response.status() != 200 { 
        println!("could not get playlist length, response code: {:?}", response.status());
        return Err("could not get playlist length".to_string())
    }
        
    //parse the response, get the length
    let res = response.text().await.expect("playlist data response could not be parsed");
    let json_res = json::parse(&res).expect("playlist data json could not be parsed");
    return Ok(json_res)
}

#[tauri::command]
pub async fn start_download(handler: State<'_, tokio::sync::Mutex<Downloader>>, window: tauri::Window, url: String, token: String) -> Result<(), ()>{
    println!("starting download");
    let mut useable_hander = handler.lock().await;

    //these vars dont need to change
    if useable_hander.window == None { 
        useable_hander.set_window(window);
    }

    if useable_hander.thread_count == 0 { 
        useable_hander.set_config().expect("err");
    }

    if useable_hander.yt_dlp_install_type == "" {
        useable_hander.set_install_type();
    }

    //these do though
    useable_hander.set_token(token.to_string());
    useable_hander.set_url(url.to_string());
    useable_hander.set_download_dir().await.expect("COULD NOT CREATE OR FIND DOWNLOAD DIRECTORY");

    //start the download
    let _ = useable_hander.start_handler().await;
    println!("done");
    Ok(())
}

#[tauri::command]
pub async fn stop_download(handler: State<'_, tokio::sync::Mutex<Downloader>>) -> Result<(), ()> {
    let mut useable_hander = handler.lock().await;
    useable_hander.stop_handler().await;
    Ok(())
}

#[tauri::command]
pub async fn check_link(url: &str, token: &str) -> Result<bool, String>{
    println!("check url {}", url);

    let url_parsed = url
        .replace("https://open.spotify.com/playlist/", "");

    let client = reqwest::Client::new();                                                                                //haha 69 its the funny number
    let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/tracks/", url_parsed))
        .header(AUTHORIZATION, format!("Bearer {}", token)) 
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .send().await.unwrap();
    
    //make sure the request went through
    if response.status() != 200 { 
        println!("invalid playlist url!, response code: {:?}", response.status()); 
        return Err("invalid playlist url!".to_string());
    }

    println!("playlist link OK");
    Ok(true)
}

#[tauri::command]
pub async fn get_token() -> Result<String, String>{
    match super::config::get_config() {
        Ok(config) => {

            let client = reqwest::Client::new();
            let response = client.post("https://accounts.spotify.com/api/token")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(format!("grant_type=client_credentials&client_id={}&client_secret={}", config.client_id, config.client_secret))
                .send().await.unwrap();
        
            //make sure it was successful
            if response.status() != 200 {
                println!("invaliad credentals, response code {:?}", response.status()); 
                return Err("credientals invalid!".to_string());
            }
            
            //parse to json, extract and return token 
            let res =  response.text().await.expect("could not parse response");
            let json_res: AcsessTokenResponse = serde_json::from_str(&res).map_err(|err| format!("Error parsing JSON: {}", err))?;
            return Ok(json_res.access_token)

        },
        Err(_) => return Err("could not get config".to_string())
    };
}
