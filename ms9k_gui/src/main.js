const { invoke } = window.__TAURI__.tauri;
const { emit, listen } = window.__TAURI__.event; 

//event handlers

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

listen("downloadFinish", (e)=>{
  console.log("Download finished")
  let button = document.getElementById("download_button")
  document.getElementById("playlist_items").innerHTML = " "
  button.dataset.status = "standby"
  button.innerText = "Download!"
})

listen("disableStop", ()=> {
  console.log("Disable Stop")
  let elem = document.getElementById("download_button")
  elem.dataset.status = "disabled"
  elem.style = "cursor: default; color: #776f6f; outline: 1px solid red"

})

listen("enableStop", ()=>{
  console.log("Enable Stop")
  let elem = document.getElementById("download_button")
  elem.dataset.status = "downloading"
  elem.style = ""
})

listen("threadDone", async ()=> {

  let elem = document.getElementById("threadCount")
  let thread_ammount = parseInt(elem.dataset.count)
  let finished_ammount = parseInt(elem.dataset.finished) + 1 
  if (finished_ammount >= thread_ammount) {
    
    console.log("all threads finished")

    //kill all the threads (this is a weird workaround)
    //because calling futures::future::join_all(self.threads.take().unwrap()).await
    //prevents the threads from being aborted
    await invoke("stop_download")
    elem.dataset.finished = 0

  } else { 
    elem.dataset.finished = finished_ammount
  }

})

//evreything else

let download_button = async () => {
  console.log("download func")
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

}

async function open_main_page(){
  let threadcount = await invoke("get_thread_count");

  document.querySelector("body").innerHTML = `
  <div class="grid">
  <input id="playlist_link" placeholder="playlist link" >
  <div class="button_container">
    <button id="add_button""><i class="fa-solid fa-plus"></i></button>
    <button id="download_button" data-status="standby" >Download!</button>
  </div>
  <div id="playlist_items">
  </div>
  <div id="config">
    <div id="threadCount" data-count="${threadcount}" data-finished="0">
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
  });

  //this needs to be different because of child elements
  //adds in click functionality to playlists
  let x = document.getElementsByClassName("playlist")
  console.log(x.length)
  for (let i = 0; i <= x.length; i++){
    try {

      x[i].addEventListener("click", () => {
        playlist_click(x[i].getAttribute("id"))
      })

    } catch {
      console.log("not an element")
    }
  }

  document.querySelectorAll(".playlist_button").forEach(button=>{ 
    button.addEventListener("click", playlist_buttons)
  })

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

//playlist stuff

let add_playlist = async () => {
  let token = document.querySelector("body").dataset.token
  let playlist_id = document.querySelector("#playlist_link").value
  if ( playlist_id == "" || playlist_id == " " ) { return }
  await invoke("set_playlist", {playlistId: playlist_id, token: token});
}

let show_playlist_options = async (e) => {
  let options_elem = e.srcElement.getElementsByClassName("playlist_options")[0]
  options_elem.style = "opacity: 100; transition: opacity 0.15s ease-in-out"
}

let hide_playlist_options = async (e) => {
  let options_elem = e.srcElement.getElementsByClassName("playlist_options")[0]
  options_elem.style = "opacity: 0; transition: opacity 0.15s ease-in-out"
}

let playlist_buttons = async (e)=>{
  console.log(e.srcElement.dataset.id)
  if (e.srcElement.dataset.type != "trash"){
    console.log("unknown event!")
    return
  }
  console.log("playlist remove")
  await invoke("remove_playlist", {playlistId: e.srcElement.dataset.id})
}

function playlist_click(id) {
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
}

//onload process

window.addEventListener("DOMContentLoaded", async () => {

  //check for yt-dlp 
  try {
    let is_installed = await invoke("ytdlp_check")
    console.log(is_installed)
  }
  catch { 
    console.log("Please install YTDLP")
    let install_response = await window.confirm("automatically install YTDLP?")
   
    if(install_response){
      await invoke("download_ytdlp")
    }
  }

  //check for valid credentials
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
