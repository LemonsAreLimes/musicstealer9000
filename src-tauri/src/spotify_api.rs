use std::{
    path::PathBuf,
    fs,
    sync::{ Arc, Mutex }
};

use serde_json;
use reqwest;
use reqwest::header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE };
use json::JsonValue;
use rand::Rng;
use tauri::State;
use super::downloader::download_process;

//TODO: maybe create a file just for these structs, outhers 
//for structs with an impl. may reduce memory usage? also performance
//as it will not haft to convert all the data (Downloader::new())
use super::static_types::{
    RunOptions,
    Song,
    GenericUpdate,
    AcsessTokenResponse,
    Playlist
};

#[derive(Debug)]
pub struct Downloader {
    pub window: Option<tauri::Window>,
    pub token: String,
    pub playlist: Playlist,
    pub data: Vec<Song>,
    pub thread_count: u32,
    pub threads: Option<Vec<tokio::task::JoinHandle<()>>>,
    pub options: RunOptions,
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

        let rng = rand::thread_rng().gen_range(0..1000000);
        let mut temp_song_data = Song {
            video_id: rng,
            name: track["name"].as_str().unwrap().to_string(),
            image_url: None,
            track_number: None,
            artist: "".to_string(),
            album: None,
            album_artist: None,
            year: None,
            genre: None,
        };

        if self.options.id3_year {

            //get album year
            let mut year = track["album"]["release_date"].as_str().unwrap().to_string();
            year.replace_range(4..year.len(), "");
            let int_year = year.parse::<i32>().unwrap();
            temp_song_data.year = Some(int_year);
        }

        if self.options.id3_artist { 

            //get track artists
            let mut artists = track["artists"][0]["name"].as_str().unwrap().to_string();
            if track["artists"].len() > 0 {
                for i in 1..track["artists"].len() {
                    let second_artist = ", ".to_string() + &track["artists"][i]["name"].as_str().unwrap().to_string();
                    artists += &second_artist; 
                }
            }
            temp_song_data.artist = artists;

            //get album artists
            let mut album_artists = track["album"]["artists"][0]["name"].as_str().unwrap().to_string();
            for i in 1..track["album"]["artists"].len() {
                let second_artist = ", ".to_string() + &track["album"]["artists"][i]["name"].as_str().unwrap().to_string();
                album_artists += &second_artist 
            }
            temp_song_data.album_artist = Some(album_artists);
        }

        if self.options.id3_album {

            //get album name, set to single if single
            let mut album_name = "single".to_string();
            if track["album"]["album_type"].as_str().unwrap() != "single" {
                album_name = track["album"]["name"].as_str().unwrap().to_string();
            }
            temp_song_data.album = Some(album_name);
        }

        if self.options.image_download { 
            temp_song_data.image_url = Some(track["album"]["images"][0]["url"].as_str().unwrap().to_string())
        }

        if self.options.id3_track_number {  //TODO: fix this in the event that its not found
                                            //would result in the number being Some(None) 
                                            //this also goes for all the outher ones here
            temp_song_data.track_number = Some(track["track_number"].as_u32().unwrap())
        }


        if self.options.id3_genre {
            temp_song_data.genre = Some(self.get_genre(track).await.unwrap())
        }

        println!("SONG FOUND: {:?}", temp_song_data.name);

        //return it
        return temp_song_data

    }
    async fn get_batch_data(&mut self, offset: usize) -> Result<Vec<Song>, String>{

        //get the batch data
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/tracks?limit=100&offset={}", self.playlist.id, offset))
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

    fn get_filenames(&self) -> Option<Vec<String>>{
        let mut file_names = Vec::new(); 
        match fs::read_dir(&self.options.download_dir) {
            Ok(entries) => { 

                for entry in entries {
                    let entry = entry.expect("err reading entry");
                    let os_string = entry.file_name();
                    let filename = os_string.to_str().unwrap();
                    let parsed_filename = str::replace(filename, ".mp3", "").to_string();
        
                    println!("FOUND SONG ALREADY DOWNLOADED: {:?}", &parsed_filename);
                    file_names.push(parsed_filename);
                };
                
                return Some(file_names)
            },
            Err(_) => {
                println!("no dir found");
                let parsed_path = self.options.download_dir.join(&self.playlist.name);
                let _ = fs::create_dir_all(parsed_path);
                return None
            }
        }


    }
    async fn get_playlist_length(&self) -> Result<usize, String>{
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/", self.playlist.id))
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
                        if let Some(songs) = &already_downloaded_songs {
                            if songs.contains(&song.name) {
                                println!("soung already downloaded");
                                self.window.as_ref().expect("no window found").emit("statusUpdate", GenericUpdate {id: song.video_id, text: "done!".to_string()})
                                    .expect("could not send status update (DONE)");
                                
                                continue;
                            }
                        }
                        
                        songs_collected.push(song);
                        println!("soung NOT already downloaded")
                        
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

        //check if the download dir exists (it hast to)
        // match fs::read_dir(&self.options.download_dir){
        //     Ok(_)=>{
        //         println!("download dir was found!")
        //     },
        //     Err(_)=>{
        //         //the dir was not found, create it.



        //     }
        // }

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
        let items_mutx = Arc::new(Mutex::new(&self.data));
        let options_mutx = Arc::new(Mutex::new(&self.options));

        let chunks: Vec<_> = items_mutx.lock().unwrap().chunks(chunk_size)
            .map(|chunk| Arc::new(chunk.to_vec()))
            .collect();

        println!("DOWNLOAD STARTING");
        for chunk in chunks {

            let chunk_mutx = Arc::new(chunk);
            let chunk_clone = chunk_mutx.clone();

            let window_clone = window_mutx.clone();
            let window_useable = match window_clone.lock().unwrap().clone() {
                Some(window) => window,
                None => panic!("window not found")
            };

            let options_clone = options_mutx.lock().unwrap().clone();
            
            
            let handle = tokio::spawn(async move {
                println!("{:?}", options_clone);
                download_process(window_useable, chunk_clone, options_clone)
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

                self.playlist.id = "".to_string();
                self.token = "".to_string(); 
            }
            None => {
                println!("no threads to stop")
            }
        } 
    }

    pub async fn set_download_dir(&mut self) -> Result<(), ()>{
        
        //get the playlist name
        let client = reqwest::Client::new();
        let response = client.get(format!("https://api.spotify.com/v1/playlists/{}/", self.playlist.id))
            .header(AUTHORIZATION, format!("Bearer {}", self.token)) 
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .send().await.unwrap();
    
        //make sure the request went through
        if response.status() != 200 { 
            println!("could not get playlist name, response code: {:?}", response.status());
            return Err(())
        }
            
        //parse the response, get the name
        let res = response.text().await.expect("playlist data response could not be parsed");
        let json_res = json::parse(&res).expect("playlist data json could not be parsed");
        let playlist_name = &json_res["name"].to_string();

        //get the download dir from the config
        match super::config::get_config() { 
            Ok(conf) => {
                let dir = conf.download_dir.join(playlist_name.to_string());
                self.options.download_dir = dir;
                return Ok(())
            },
            Err(_) => {return Err(())},
        };

    }
    pub async fn set_playlist(&mut self){
        let playlist_id = &self.playlist.id;

        let raw_playlist = get_playlist_data(playlist_id.to_string(), &self.token).await.expect("playlist no");
        let playlist_parsed = Playlist {
            name: raw_playlist["name"].to_string(),
            id: playlist_id.to_string(),
            image_url: raw_playlist["images"][0]["url"].to_string(),
            download_dir: "Default".to_string()
        };
        self.playlist = playlist_parsed
    }

    pub fn new() -> Self {

        //this function needs to by sync.
        //only operations that initalize and are sync are ran here
        let install_type = super::config::ytdlp_check().expect("not installed");

        let config = super::config::get_config().unwrap();
        let op = RunOptions { 
            ytdlp_search_type: config.download_source,
            yt_dlp_install_type: install_type,
            download_dir: PathBuf::new(),
            image_download: config.image_download,
            image_delete: config.image_delete,
            edit_id3: false,
            id3_artist: config.id3_options.artist,
            id3_year: config.id3_options.year,
            id3_album: config.id3_options.album,
            id3_track_number: config.id3_options.track_number,
            id3_genre: config.id3_options.genre,
        };

        return Downloader {
            window: None,
            token: "".to_string(),
            playlist: Playlist { name: "".to_string(), id: "".to_string(), download_dir: "".to_string(), image_url: "".to_string() },
            data: Vec::new(),
            thread_count: config.thread_count,
            threads: None,
            options: op
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
        useable_hander.window = Some(window);
    }

    let parsed_id = playlist_url_to_id(url).expect("invalid url");
    println!("{:?}", parsed_id);
    
    //these do though
    useable_hander.token = token.to_string();
    useable_hander.playlist.id = parsed_id.to_string();

    useable_hander.set_download_dir().await.expect("COULD NOT CREATE OR FIND DOWNLOAD DIRECTORY");
    useable_hander.set_playlist().await;

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
pub async fn get_token(client_id: Option<String>, client_secret: Option<String>) -> Result<String, ()>{ 

    let client_id_cpy: String;
    let client_secret_cpy: String;

    //use whatevers in the config if we are testing 
    if client_id == None || client_secret == None { 
        let config = super::config::get_config().unwrap();
        client_id_cpy = config.client_id;
        client_secret_cpy = config.client_secret;
    } else {
        client_id_cpy = client_id.unwrap();
        client_secret_cpy = client_secret.unwrap();
    }

    let client = reqwest::Client::new();
    let response = client.post("https://accounts.spotify.com/api/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!("grant_type=client_credentials&client_id={}&client_secret={}", client_id_cpy, client_secret_cpy))
        .send().await.unwrap();

    //make sure it was successful
    if response.status() != 200 {
        println!("invaliad credentals, response code {:?}", response.status()); 
        return Err(());
    }
    
    //parse to json, extract and return token 
    let res =  response.text().await.expect("could not parse response");
    let json_res: AcsessTokenResponse = serde_json::from_str(&res).map_err(|err| format!("Error parsing JSON: {}", err)).unwrap();
    return Ok(json_res.access_token)
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn playlist_url_to_id(url: String) -> Result<String, ()>{
    let mut playlist_id = url.replace("https://open.spotify.com/playlist/", "");

    if playlist_id.contains("?si=") {
        playlist_id.replace_range(22..playlist_id.len(), "");
    }

    if playlist_id.len() != 22 {return Err(());}
    Ok(playlist_id.to_string())
}
