use std::{
    process::Command,
    path::{Path, PathBuf},
    fs,
    sync::Arc,
};
use reqwest;
use image;
use id3::{ Tag, TagLike, Version };
use id3_image::embed_image;

use super::static_types::{
    RunOptions,
    Song,
    GenericUpdate
};

//TODO:
//add in checking if yt-dlp found nothing
//alert the use if nothing was found
//not sure exactly how to do this
//maybe searc w/ --no-download then parse out response
//then download the url?? this will impact performace though.

async fn edit_id3(track: &Song, track_path: &PathBuf){
    
    //set the tags
    let mut tag = Tag::new();
    tag.set_title(&track.name);

    if let Some(album) = &track.album {
        tag.set_album(album);
    }
    if let Some(album_artist) = &track.album_artist {
        tag.set_artist(&track.artist);
        tag.set_album_artist(album_artist);
    }
    if let Some(year) = track.year {
        tag.set_year(year);
    }
    if let Some(track_number) = track.track_number {
        tag.set_track(track_number);
    }
    if let Some(genre) = &track.genre {
        tag.set_genre(genre);
    }

    tag.write_to_path(track_path, Version::Id3v24).expect("failed to write id3");

}
async fn download_audio(song: &Song, dir: &PathBuf, install_type: &str, search_type: &str) -> bool{

    let mut yt_dlp_location = "yt-dlp".to_string();
    if install_type == "local" {
        //get our current dir
        let curr_dir = std::env::current_dir()
            .expect("could not get current directory")
            .to_owned().to_string_lossy().to_string();
        
        yt_dlp_location = format!("{}\\yt-dlp.exe", curr_dir);
    }

    //parse out yt-dlp arguments
    let search_arg = format!("{}:'{}'", search_type, song.name.to_string() + " " + &song.artist);
    let dir_arg = format!("-P {}", dir.to_string_lossy());
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
async fn get_image(image_url: &String, download_path: &Path){
    let bytes = reqwest::get(image_url)
        .await
        .expect("could not download image")
        .bytes()
        .await
        .expect("could not parse image to bytes");
    let img = image::load_from_memory(&bytes).expect("could not load image");
    img.save(download_path).expect("could not save image"); 
}

async fn send_update(window: &tauri::Window, video_id: i32, update_message: &str){
    window
        .emit("statusUpdate", GenericUpdate {id: video_id, text: update_message.to_string()})
        .expect(&format!("could not send status update, update message: {}", update_message));
}

pub async fn download_process(window: tauri::Window, songs: Arc<Arc<Vec<Song>>>, options: RunOptions) -> Result<(), String> {

    let dir = options.download_dir;

    for song in songs.to_vec(){

        send_update(&window, song.video_id, "Starting").await;
        let track_path = dir.join(song.video_id.to_string() + ".mp3");
        let image_path = dir.join(song.video_id.to_string() + ".jpg");

        if &song.image_url != &None { 
            println!("IMG PATH: {:?}", &image_path);
            get_image(                                          // this function will only ever run if we have an image and want to download it 
                &song.image_url.clone().unwrap(),
                &image_path
            ).await; 
        }

        send_update(&window, song.video_id, "Downloading").await;

        // audio download, this can never be turned off
        let download_succsess = download_audio(&song, &dir, &options.yt_dlp_install_type, &options.ytdlp_search_type).await;

        if !download_succsess { 
            send_update(&window, song.video_id, "THREAD CRASH!!!").await;
            println!("THREAD CRASHED!!! ALL SONGS DELIGATED TO THIS THREAD WILL NOT BE DOWNLOADED!");
            continue;
        }

        if options.edit_id3 {
            send_update(&window, song.video_id, "Editing metadata").await;
            edit_id3(&song, &track_path).await;

            // logic regarding id3 image
            if &song.image_url != &None { 
                
                //embeds the image
                embed_image(&track_path, &image_path)
                    .expect("could not set image");

                if options.image_delete {        //delete the image if requested
                    fs::remove_file(image_path)
                        .expect("could not remove old image file!");
                    println!("remove old image file");
                } 
                else {                          //rename the image to the same as the audio 
                    let mut renamed_image_path = image_path.clone();
                    renamed_image_path.set_file_name(&song.name);
                    renamed_image_path.set_extension("jpg");

                    fs::rename(image_path, &renamed_image_path)
                        .expect("could not rename the image file!");
                }
            }
        } 

        send_update(&window, song.video_id, "Finishing up").await;

        
        //finally, rename the audio file to the appropreate thing
        let mut renamed_audio_path = track_path.clone();
        renamed_audio_path.set_file_name(&song.name);
        renamed_audio_path.set_extension("mp3");

        fs::rename(track_path, renamed_audio_path)
            .expect("could not rename the audio file!");

        send_update(&window, song.video_id, "done!").await;
    }
    window.emit("threadDone", "")
        .expect("could not send status update (thread finished)");

    println!("THREAD DONE");
    Ok(())

}