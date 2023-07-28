use std::os::windows::process::CommandExt;
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
use tokio;
use image;
use id3::{ Tag, TagLike, Version };
use id3_image::embed_image;
use dirs;

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

struct Downloader {
    window: tauri::Window,
    download_dir: String,
    token: String,
    playlist_id: String,
    playlist_length: Option<usize>,
    data: Vec<Song>,
    thread_count: u32,
} 
unsafe impl Send for Downloader {}

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
async fn download(song: &Song, dir: &str) -> bool{

    //parse out yt-dlp arguments
    let search_arg = format!("ytsearch:'{}'", song.name.to_string() + " " + &song.artist);
    let dir_arg = format!("-P {}", dir);
    let name_arg = format!("-o{}", &song.video_id);

    //run yt-dlp
    let mut cmd = Command::new("yt-dlp");
    let x = cmd.arg(search_arg)
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("mp3")
        .arg(dir_arg)
        .arg(name_arg)
        .status();
    cmd.creation_flags(0x08000000);

    match x { 
        Ok(_) => return true,
        Err(_) => return false
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
        if response.status() != 200 { println!("it did not work"); return None} 
            
        //parse the response
        let res =  response.text().await.expect("could not parse response");
        let json_res = json::parse(&res).expect("could not parse json");

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

        println!("{:?}", temp_song_data.name);

        //return it
        return temp_song_data

    }
    async fn get_batch_data(&mut self, offset: usize){

        //get the batch data
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/tracks?limit=100&offset={}", self.playlist_id, offset))
            .header(AUTHORIZATION, format!("Bearer {}", self.token)) 
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .send().await.unwrap();
    
        if response.status() != 200 { 
            println!("it did not work"); 
            return 
        } 
        
        //parse the response to json
        let res =  response.text().await.expect("could not parse response");
        let json_res = json::parse(&res).expect("could not parse json");
        let res_len = json_res["items"].len();

        //parse out the relivant data
        for i in 0..res_len {
            let track = &json_res["items"][i]["track"];
            let new_song = self.parse_song_data(track).await;
            self.data.push(new_song.to_owned());

            // each time this runs send out a signal to the client to show that this has completed 
            let _ = self.window.emit("nameUpdate",  GenericUpdate{ id: new_song.video_id, text: new_song.name });
        }

        
    }

    async fn start_handler(&mut self){

        //get the length of the playlist
        // self.playlist_length = Some(self.get_length().await);
        // println!("{:?}", self.playlist_length);

        //get all the tracks in the playlist
        let mut offset: usize = 0;
        while offset < self.playlist_length.unwrap() +100 as usize {
            self.get_batch_data(offset).await;
            offset += 100
        }

        // start the download in another thread
        let mut handles = Vec::new();
        let chunk_size = self.data.len() / self.thread_count as usize;

        //mutexify relavant variables
        let window_mutx = Arc::new(Mutex::new(&self.window));
        let dir_mutx = Arc::new(Mutex::new(&self.download_dir));
        let items_mutx = Arc::new(Mutex::new(&self.data));
        let chunks: Vec<_> = items_mutx.lock().unwrap().chunks(chunk_size)
            .map(|chunk| Arc::new(chunk.to_vec()))
            .collect();

        println!("downloading");
        for chunk in chunks {
            let chunk_mutx = Arc::new(chunk);
            let chunk_clone = chunk_mutx.clone();
            
            let dir_clone = dir_mutx.clone();
            let dir_useable = dir_clone.lock().unwrap().clone();

            let window_clone = window_mutx.clone();
            let window_useable = window_clone.lock().unwrap().clone();

            let handle = tokio::spawn(async move {
                for song in chunk_clone.to_vec(){

                    let _ = window_useable.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "Starting".to_string()});

                    let image_path = format!("{}/{}.jpg", dir_useable, song.video_id);
                    get_image(&song.image_url, &image_path).await; 
                    let _ = window_useable.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "Downloading".to_string()});

                    let download_succsess = download(&song, &dir_useable).await;
                    if !download_succsess { 
                        let _ = window_useable.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "THREAD CRASHED!!!".to_string()});
                        continue;
                    }
                    let _ = window_useable.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "Finishing up".to_string()});

                    edit_id3(&song, &dir_useable, &image_path).await;
                    let _ = window_useable.emit("statusUpdate", GenericUpdate {id: song.video_id, text: "done!".to_string()});

                }
            });

            handles.push(handle);
        } 

        futures::future::join_all(handles).await;

    }

}

pub async fn get_playlist_data(playlist_id: String)-> Result<JsonValue, String>{
    match get_token().await {
        Ok(token) => { 
            let client = reqwest::Client::new();                                                                                //haha 69 its the funny number
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
            let res = response.text().await.expect("response could not be parsed");
            let json_res = json::parse(&res).expect("json could not be parsed");
            return Ok(json_res)
        }
        Err(_) => Err("something bad happened".to_string())
    }
}

#[tauri::command]
pub async fn start_download(url: &str, token: &str, window: tauri::Window) -> Result<String, String> {   

    let data = get_playlist_data(url.replace("https://open.spotify.com/playlist/", "")).await;
    println!("download");
    match data { 
        Ok(data) => {

            //get some data
            let playlist_name = &data["name"].to_string();
            let length = &data["tracks"]["total"].as_usize().unwrap();

            //create the download dir
            let mut download_dir = dirs::desktop_dir().expect("err");
            download_dir.push("music");
            download_dir.push(playlist_name);

            //check if the new download dir exists, create it if not
            if !download_dir.exists(){
                let _ = fs::create_dir_all(&download_dir);
            }

            //start the download
            match super::config::get_config() {
                Ok(config) => {
                    let mut downloader = Downloader {
                        window: window,
                        download_dir: download_dir.to_owned().to_string_lossy().to_string(),
                        token: token.to_string(),
                        playlist_id: url.replace("https://open.spotify.com/playlist/", ""),
                        playlist_length: Some(length.to_owned()),
                        data: Vec::new(),
                        thread_count: config.thread_count
                    };
                
                    let _ = downloader.start_handler().await;
                    return Ok("a".to_string());
                }
                Err(err) => { println!("Error: {}", err); return Err(err); }
            }
        }
        Err(_) => return Err("ksjdfhnsdjkfh".to_string())
    }


}

#[tauri::command]
pub async fn check_link(url: &str, token: &str) -> Result<bool, bool>{

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
        println!("could not get playlist length, response code: {:?}", response.status()); 
        return Err(false);
    }

    println!("{:?}" ,response.status());
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
            if response.status() != 200 {println!("invaliad credentals, response code {:?}", response.status()); return Err("credientals invalid!".to_string());}
            
            //parse to json, extract and return token 
            let res =  response.text().await.expect("could not parse response");
            let json_res: AcsessTokenResponse = serde_json::from_str(&res).map_err(|err| format!("Error parsing JSON: {}", err))?;
            return Ok(json_res.access_token)

        },
        Err(_) => return Err("could not get config".to_string())
    };
}