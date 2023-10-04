use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Playlist {
    pub name: String,
    pub id: String,
    pub download_dir: String,
    pub image_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Id3Options { 
    pub artist: bool,
    pub year: bool,
    pub album: bool,
    pub track_number: bool,
    pub genre: bool
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub thread_count: u32,
    pub playlists: Vec<Playlist>,
    pub image_download: bool,
    pub image_delete: bool,
    pub id3_options: Id3Options,
    pub download_dir: PathBuf,
    pub download_source: String,
    pub audio_format: String,
}

#[derive(Debug, Deserialize)]
pub struct AcsessTokenResponse {
    pub access_token: String,
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct GenericUpdate { 
    pub id: i32,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Song { 
    pub video_id: i32,
    pub name: String,
    pub image_url: Option<String>,
    pub track_number: Option<u32>,
    pub artist: String,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<i32>,
    pub genre: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunOptions { 
    pub download_dir: PathBuf,
    pub yt_dlp_install_type: String,
    pub ytdlp_search_type: String,
    pub image_download: bool,
    pub image_delete: bool,
    pub edit_id3: bool,
    pub id3_artist: bool,
    pub id3_year: bool,
    pub id3_album: bool,
    pub id3_track_number: bool,
    pub id3_genre: bool
} 

//TODO:
//combine run options and id3 options and maybe
//config to one unified type, to use across functions
