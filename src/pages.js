const { invoke } = window.__TAURI__.tauri;

import events from './events.js'

async function build_playlist_container(){
    let playlists_data = await invoke("get_playlist")
    let playlist_container_html = ""

    playlists_data.forEach(playlist_data => { 
      let playlist_html = `
      <div class="playlist" data-selected="0" id="${playlist_data.id}">
        <img class="playlist_image" src="${playlist_data.image_url}">
        <div class="playlist_content_container">
          <p>${playlist_data.name}</p>
        </div>
        <div class="playlist_options" style="opacity: 0;">
          <div class="playlist_button"><i class="fa-solid fa-trash" data-type="trash" data-id="${playlist_data.id}"></i></div>
        </div>
      </div>
      `
      playlist_container_html += playlist_html
    });

    console.log("Built playlist container")
    return playlist_container_html //events are applied in the build_main_page function
}

export default {

    async default(){

        //get playlists
        let playlists_html = await build_playlist_container() 
    
        //initalize main page
        let body_html = `
        <div class="body_container">
          <div class="tab_switcher" data-active="tab_download">
            <div id="tab_download" class="tab_active">Download</div>
            <div id="tab_edit" class="tab">Edit</div>
            <div id="tab_settings" class="tab">Settings</div>
          </div>
    
          <div class="grid">
            <input id="playlist_link" placeholder="playlist link" >
            <div class="button_container">
              <button id="add_button""><i class="fa-solid fa-plus"></i></button>
              <button id="download_button" data-status="standby">Download!</button>
            </div>
            <div id="playlist_items"></div>
            <div id="config">
              <div id="playlist_select" data-selected="None">${playlists_html}</div>
              </div>
            </div>
        </div>`
    
        //add it all to the doc
        document.querySelector("body").innerHTML = body_html
    
        //setup events 
        document.querySelector("#download_button").addEventListener("click", events.download_button)
        document.querySelector("#add_button").addEventListener("click", events.add_playlist)
        let tabs = document.querySelector(".tab_switcher").children
        for (let i = 0; i < tabs.length; i++){
            tabs[i].addEventListener("click", events.switch_page)
        }

        document.querySelectorAll(".playlist").forEach(playlist =>{
            playlist.addEventListener("mouseenter", events.show_playlist_options)
            playlist.addEventListener("mouseleave", events.hide_playlist_options)
    
            playlist.querySelector(".playlist_button")
                .addEventListener("click", events.playlist_buttons)
        });
    
        //this needs to be different because of child elements
        //adds in click functionality to playlists
        let playlists = document.getElementsByClassName("playlist")
        for (let i = 0; i <= playlists.length; i++){
            try {
                playlists[i].addEventListener("click", () => {
                    events.playlist_click(playlists[i].getAttribute("id"))
                })
            } catch {
              console.log("not an element")
            }
        }
    },

    async edit(){
        console.log("to be implemented")  
    },

    async settings(){
        
        //get config, only used for thread count
        let config = await invoke("get_config")
        console.log(config)

        //cringe stupid dumb way to do this
        let image_download=""
        let image_delete=""
        let ID3_ARTIST=""
        let ID3_YEAR=""
        let ID3_ALBUM=""
        let ID3_TRACK_NUMBER=""
        let ID3_GENRE=""
        let ID3=""
        if(config.image_download){image_download="checked"}
        if(config.image_download && config.image_delete){image_delete="checked"}
        if(config.id3_options.artist){ID3_ARTIST="checked"}
        if(config.id3_options.year){ID3_YEAR="checked"}
        if(config.id3_options.album){ID3_ALBUM="checked"}
        if(config.id3_options.track_number){ID3_TRACK_NUMBER="checked"}
        if(config.id3_options.genre){ID3_GENRE="checked"}
        if (config.id3_options.artist && config.id3_options.year && config.id3_options.album && config.id3_options.track_number && config.id3_options.genre){
            ID3="checked"
        }

        let settings_html = `
        <div class="body_container">
            <div class="tab_switcher" data-active="tab_settings">
                <div id="tab_download" class="tab">Download</div>
                <div id="tab_edit" class="tab">Edit</div>
                <div id="tab_settings" class="tab_active">Settings</div>
            </div>

            <form>
                <input type="checkbox" id="image_download" value="image_download" ${image_download}>
                <label for="image_download"> Set image</label><br>

                <div class="indented" id="image_options">
                <input type="checkbox" id="image_delete" value="image_delete" ${image_delete}>
                <label for="image_delete"> Delete image</label><br>
                </div>

                <input type="checkbox" id="ID3" value="ID3" ${ID3}>
                <label for="ID3"> Edit metadata</label><br>

                <div class="indented" id="ID3_options">
                <input type="checkbox" id="ID3_ARTIST" value="ID3_ARTIST" ${ID3_ARTIST}>
                <label for="ID3_ARTIST"> Artist(s)</label><br>

                <input type="checkbox" id="ID3_YEAR" value="ID3_YEAR" ${ID3_YEAR}>
                <label for="ID3_YEAR"> Year</label><br>

                <input type="checkbox" id="ID3_ALBUM" value="ID3_ALBUM" ${ID3_ALBUM}>
                <label for="ID3_ALBUM"> Album</label><br>

                <input type="checkbox" id="ID3_TRACK_NUMBER" value="ID3_TRACK_NUMBER" ${ID3_TRACK_NUMBER}>
                <label for="ID3_TRACK_NUMBER"> Track number</label><br>

                <input type="checkbox" id="ID3_GENRE" value="ID3_GENRE" ${ID3_GENRE}>
                <label for="ID3_GENRE"> Genre</label><br>
                </div>

                <input type="number" id="thread_count" value="${config.thread_count}" min="1" max="100">
                <label for="thread_count"> Thread count</label><br>

                <select id="audio_format">
                  <option value="mp3">MP3</option>
                  <option value="flac">FLAC</option>
                  <option value="ogg">OGG</option>
                  <option value="wav">WAV</option>
                  <option value="m4a">M4A</option>
                </select>
                <label for="audio_format">Format (NON FUNCTIONAL)</label><br>

                <select id="download_src">
                  <option value="ytsearch">Youtube</option>
                  <option value="scsearch">Soundcloud</option>
                </select>
                <label for="download_src">Download source</label><br>
              </form>
            <div style="display:flex; flex-direction: column; width: 50vw; gap: 10px; margin-top: 10px;">
              <input id="client_id" placeholder="client id" />
              <input id="client_secret" type="password" placeholder="client secret" />
              <button id="creds_button" type="button">Test and set</button>
            </div>
        </div>`

        document.querySelector('body').innerHTML = settings_html

        //events for download dir (i plan to add this)
        // document.getElementById("download_dir").addEventListener("change", (e)=>{
        //   console.log(e)
        // })

        //set the current value of the select boxes
        document.getElementById("audio_format").value = config.audio_format
        document.getElementById("download_src").value = config.download_source
        console.log(config.download_source)

        //select menu events
        document.querySelectorAll("select").forEach(elem => {
          elem.addEventListener("change", events.do_thing)
        })

        //generic input events
        document.querySelectorAll("input").forEach(input_elem => {

          //creds button is input but should not be caught in this
          //it would be faster but just did not work.
          if (input_elem.type == "checkbox" || input_elem.type == "number"){
            input_elem.addEventListener("click", events.change_config_value)
          }
            
        })

        //creds button event
        document.querySelector("#creds_button").addEventListener("click", await events.test_and_set_creds)
        
        //tab swticher
        let tabs = document.querySelector(".tab_switcher").children
        for (let i = 0; i < tabs.length; i++){
            tabs[i].addEventListener("click", events.switch_page)
        }

    }
}
