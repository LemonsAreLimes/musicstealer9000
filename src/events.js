const { invoke } = window.__TAURI__.tauri;
import pages from './pages.js'

export default {

    //USED ACROSS PAGES

    switch_page(e){ 
        let new_page = e.srcElement.id
        let old_page = document.querySelector(".tab_switcher").dataset.active
      
        if (old_page == new_page ){return}
        document.querySelector(".tab_switcher").dataset.active = new_page
        document.getElementById(old_page).className = "tab"
        document.getElementById(new_page).className = "tab_active"
      
        switch (new_page) {
          case "tab_download": 
            pages.download()
            return
          case "tab_edit":  
            pages.edit()
            return
          case "tab_settings": 
            pages.settings()
            return
          default: 
            console.log("unkown page selected!")
            return
        }
    },

    //CREDS

    async test_credentials(){
        console.log("test_credentials")

        //get the credentials from the input feilds
        let client_id = document.querySelector("#client_id").value
        let client_secret = document.querySelector("#client_secret").value
      
        //test it
        try{
          console.log("test_client_credentials")
      
          await invoke("set_credentials", {clientId: client_id, clientSecret: client_secret})
          let token = await invoke("get_token")
          console.log(token)
      
          document.querySelector("body").dataset.token = token
          open_main_page()
        } catch { 
          //if the credentials are invalid 
          document.querySelector("#client_id").style = "outline: 1px solid red;"
          document.querySelector("#client_secret").style = "outline: 1px solid red;"
        }
    },

    //DOWNLOAD

    async add_playlist() {
        let token = document.querySelector("body").dataset.token
        let playlist_id = document.querySelector("#playlist_link").value
        if ( playlist_id == "" || playlist_id == " " ) { return }
        await invoke("set_playlist", {playlistId: playlist_id, token: token});
    },

    show_playlist_options(e){
        let options_elem = e.srcElement.getElementsByClassName("playlist_options")[0]
        options_elem.style = "opacity: 100; transition: opacity 0.15s ease-in-out"
    },

    hide_playlist_options(e){
        let options_elem = e.srcElement.getElementsByClassName("playlist_options")[0]
        options_elem.style = "opacity: 0; transition: opacity 0.15s ease-in-out"
    },

    async playlist_buttons(e){
        console.log(e.srcElement.dataset.id)
        if (e.srcElement.dataset.type != "trash"){
          console.log("unknown event!")
          return
        }
        console.log("playlist remove")
        await invoke("remove_playlist", {playlistId: e.srcElement.dataset.id})
    },

    async playlist_click(id){
        let srcElem = document.getElementById(id)
        let is_selected = srcElem.dataset.selected
      
        //select
        if (!parseInt(is_selected) && srcElem.className=="playlist"){
          srcElem.style = "background-color: #272727;"
          srcElem.dataset.selected = 1
      
          //deslect any already selected items
          let prev_selected_id = document.querySelector("#playlist_select").dataset.selected 
          if (prev_selected_id && prev_selected_id!="None"){
            let prev_elem = document.getElementById(prev_selected_id)
            prev_elem.dataset.selected = "None"
            prev_elem.style = ""
          }
      
          document.querySelector("#playlist_select").dataset.selected = id
          document.querySelector("#playlist_link").value = id
        } 
      
        //deslect
        else if (parseInt(is_selected) && srcElem.className=="playlist"){
          srcElem.style = ""
          srcElem.dataset.selected = 0
          document.getElementById(prev_selected_id).dataset.selected = "None"
        }
    },

    async download_button(){
        let button = document.getElementById("download_button")

        if ( button.dataset.status == "disabled" ){return}

        //if we are not already downloading, then download
        if( button.dataset.status == "standby" ){ 
            console.log("standby")
            
            //make sure the link is correct
            let token = document.querySelector("body").dataset.token
            let url = document.querySelector("#playlist_link").value
            if(url=="" || url==" "){ return false; }

            let is_valid = await invoke("check_link", {url: url, token: token});
            console.log(is_valid)
            if (!is_valid) { 
            console.log("invalid link!")
            return false; 
            }

            console.log("downloading: "+url)

            //switch download button to stop button
            button.dataset.status = "downloading"
            button.innerText = "Stop"

            let x = await invoke("start_download", {url: url, token: token});
            console.log(x)
            console.log("d")
        }
        
        //if we are downloading and stop button is clicked
        else if (button.dataset.status == "downloading"){
            console.log("stopping download")
            button.dataset.status = "standby"
            button.innerText = "Download!"
            await invoke("stop_download")
            document.getElementById("playlist_items").innerHTML = ""
        }
    },
    
    async change_config_value(e){
        let config = await invoke("get_config")
        console.log(config)
      
        let id = e.srcElement.id
        let value = e.srcElement.checked
      
        //thread count ticker thing
        if (id == "thread_count"){
          config["thread_count"] = parseInt(e.srcElement.value)
          await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
          return
        }
      
        // disable options related to image download, send back-end respective information
        if (id == "image_download"){
          document.querySelector("#image_options")
            .querySelectorAll("input")
            .forEach(async elem => {
              elem.disabled = !value
              elem.checked = value
              config[elem.id] = value
            });
      
            config["image_download"] = false
            await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
            return
        }
      
        // disable options related to ID3, send back-end respective information
        if (id == "ID3"){
          document.querySelector("#ID3_options")
            .querySelectorAll("input")
            .forEach(elem => {
              elem.disabled = !value
              elem.checked = value
              let id_parsed = elem.id.replace("ID3_", "").toLowerCase()
              config.id3_options[id_parsed] = value
          });
      
          await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
          return
        }
      
        //any ID3 standalone option 
        if(id.startsWith("ID3_")){
          let id_parsed = id.replace("ID3_", "").toLowerCase()
          config.id3_options[id_parsed] = value
          await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
          return
        }
      
        //and outher standalone option
        config[id] = value
        await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
    },


}

