const { invoke } = window.__TAURI__.tauri;
const { emit, listen } = window.__TAURI__.event; 
import pages from './pages.js'

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
    await invoke("stop_download")
    elem.dataset.finished = 0

  } else { 
    elem.dataset.finished = finished_ammount
  }

})

//onload process
window.addEventListener("DOMContentLoaded", async () => {

  //check for yt-dlp
  //warn the user that its not installed, dont install it.
  try { 
    await invoke("ytdlp_check")
  } catch { 
    alert("yt-dlp is not installed, nothing will download if its not installed!")
  }

  //check if the config exists
  try {
    await invoke("get_config")
  } catch { 
    //nothing really needs to happen here
    console.log('no config found')
  }

  //beyond this point the config should exist

  pages.default()  
});

//TODO: tab switcher breaks page height, will need to be changed or allow for scrolling
//TODO: add the ability to download into diffrent formats
  //non-mp3 formats will not have id3 

//TODO: build out edit page
  //some things that sould be on the edit page:
  //image chager,
  //gain control,
  //manual ID3 editor
  //audio clipper, 
    //this should be delagated to another update as
    //it seems daunting, also might require ffmpeg to be installed
  //compressor? 
  //audio converter, requires ffmpeg 
  
//TODO: custom args might be interesting for yt-dlp, incase you use a proxy or something
