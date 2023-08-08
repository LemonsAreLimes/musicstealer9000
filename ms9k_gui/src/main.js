const { invoke } = window.__TAURI__.tauri;
const { emit, listen } = window.__TAURI__.event; 

listen("statusUpdate", (e)=>{
  if (e.payload.text == "done!"){
    document.getElementById(e.payload.id).remove()
  } else {
    document.getElementById(`${e.payload.id}_status`).textContent = e.payload.text
  }  
})

listen("nameUpdate", (e)=>{
  document.querySelector("#playlist_items").innerHTML += `
  <div class="track" id="${e.payload.id}">
    <p class="trackName">${e.payload.text}</p>
    <p class="trackStatus" id="${e.payload.id}_status">Discovered</p>
  </div>
  `
})

let download_button = async () => {
  console.log("download func")

  let button = document.getElementById("download_button")
  console.log(button.dataset.status)

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
  }

}

let add_playlist = async () => {
  let token = document.querySelector("body").dataset.token
  let playlist_id = document.querySelector("#playlist_link").value
  if ( playlist_id == "" || playlist_id == " " ) { return }
  await invoke("set_playlist", {playlistId: playlist_id, token: token});
}
async function open_main_page(){
  let threadcount = await invoke("get_thread_count");

  document.querySelector("body").innerHTML = `
  <div class="grid">
  <input id="playlist_link" placeholder="playlist link" >
  <div class="button_container">
    <button id="add_button""><i class="fa-solid fa-plus"></i></button>
    <button id="download_button"" data-status="standby" >Download!</button>
  </div>
  <div id="playlist_items">
  </div>
  <div id="config">
    <div id="threadCount" data-count="${threadcount}">
      <p id="threadText">Threads: ${threadcount}</p>
      <div id="threadUpButton">
        <i class="fa-solid fa-chevron-up "></i>
      </div>
      <div id="threadDownButton">
        <i class="fa-solid fa-chevron-down"></i>
      </div>
    </div>
    <div id="playlist_select" data-selected="None"></div>
    </div>
  </div>`
  // <button id="folderSelect" onclick="openFileInput()">Chose a directory</button>
  document.querySelector("#download_button").addEventListener("click", download_button)
  document.querySelector("#threadDownButton").addEventListener("click", decreceThreadCount)
  document.querySelector("#threadUpButton").addEventListener("click", increceThreadCount)
  document.querySelector("#add_button").addEventListener("click", add_playlist)

  //add playlists from config
  let playlists = await invoke("get_playlist")
  playlists.forEach(playlist => { 
    let elem = `
    <div class="playlist" data-selected="0" id="${playlist.id}">
      <img class="playlist_image" src="${playlist.image_url}">
      <div class="playlist_content_container">
        <p>${playlist.name}</p>
      </div>
      <div class="playlist_options" style="opacity: 0;">
        <div class="playlist_button"><i class="fa-solid fa-trash" data-type="trash" data-id="${playlist.id}"></i></div>
      </div>
    </div>
    `
    document.querySelector("#playlist_select").innerHTML += elem
  })

  document.querySelectorAll(".playlist").forEach(playlist =>{
    playlist.addEventListener("mouseenter", show_playlist_options)
    playlist.addEventListener("mouseleave", hide_playlist_options)
    playlist.addEventListener("mousedown", toggle_playlist)
  });

  document.querySelectorAll(".playlist_button").forEach(button=>{ 
    button.addEventListener("click", playlist_buttons)
  })

}

let playlist_buttons = async (e)=>{
  console.log(e.srcElement.dataset.id)
  if (e.srcElement.dataset.type == "trash"){
    console.log("trash")
    await invoke("remove_playlist", {playlistId: e.srcElement.dataset.id})
  } else if (e.srcElement.dataset.type == "settings"){
    console.log("settings")
  }
}

let test_client_credentials = async () => {
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
}

async function open_credential_page(){
  
  // add the stuff to the dom
  document.querySelector("body").innerHTML = `
  <video src="assets/a.mp4" type="video/mp4" autoplay muted loop id="lol"></video> 
  <div class="container">
    <h1>Welcome to music stealer 9000!</h1>
    <p>no client credientals could be found!</p>
    <p>please visit developer.spotify.com/dashboard to generate your client credentials</p>
    <div class="credsPage">
      <div>
        <input id="client_id" placeholder="client id" />
        <input id="client_secret" type="password" placeholder="client secret" />
        <button id="creds_button" type="button">Lets go!</button>
      </div>
    </div>
  </div>
  `

  //set the event handler for the button
  document.querySelector("#creds_button").addEventListener("click", test_client_credentials)
  return
}

let decreceThreadCount = async ()=>{
  let count = document.querySelector("#threadCount").dataset.count
  if( count == 1 ){ 
    await invoke("set_thread_count", {threadCount: parseInt(count)})  
    return 
  }
  document.querySelector("#threadCount").dataset.count = parseInt(count) - 1
  document.querySelector("#threadText").textContent = `Threads: ${parseInt(count)-1}`
  await invoke("set_thread_count", {threadCount: parseInt(count)-1})
}

let increceThreadCount = async ()=>{
  let count = document.querySelector("#threadCount").dataset.count
  document.querySelector("#threadCount").dataset.count = parseInt(count) + 1
  document.querySelector("#threadText").textContent = `Threads: ${parseInt(count)+1}`
  await invoke("set_thread_count", {threadCount: parseInt(count)+1})
}

let show_playlist_options = async (e) => {
  let options_elem = e.srcElement.getElementsByClassName("playlist_options")[0]
  options_elem.style = "opacity: 100; transition: opacity 0.15s ease-in-out"
}

let hide_playlist_options = async (e) => {
  let options_elem = e.srcElement.getElementsByClassName("playlist_options")[0]
  options_elem.style = "opacity: 0; transition: opacity 0.15s ease-in-out"
}

let toggle_playlist = async (e) => {
  let is_selected = e.srcElement.dataset.selected

  //select
  if (!parseInt(is_selected) && e.srcElement.className=="playlist"){
    e.srcElement.style = "background-color: #272727;"
    e.srcElement.dataset.selected = 1

    //deslect any already selected items
    let prev_selected_id = document.querySelector("#playlist_select").dataset.selected 
    if (prev_selected_id && prev_selected_id!="None"){
      document.querySelector(`#${prev_selected_id}`).dataset.selected = "None"
      document.querySelector(`#${prev_selected_id}`).style = ""
    }

    document.querySelector("#playlist_select").dataset.selected = e.srcElement.id
    document.querySelector("#playlist_link").value = e.srcElement.id
  } 

  //deslect
  else if (parseInt(is_selected) && e.srcElement.className=="playlist"){
    e.srcElement.style = ""
    e.srcElement.dataset.selected = 0
    document.querySelectorAll(`#${prev_selected_id}`).dataset.selected = "None"
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  try {
    let token = await invoke("get_token");
    console.log(token)
    document.querySelector("body").dataset.token = token
    await open_main_page()
    return
  } catch { 
    open_credential_page()
    return
  }
});
