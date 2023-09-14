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
    //because calling futures::future::join_all(self.threads.take().unwrap()).await
    //prevents the threads from being aborted
    await invoke("stop_download")
    elem.dataset.finished = 0

  } else { 
    elem.dataset.finished = finished_ammount
  }

})

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
    pages.download()
    return
  } catch { 
    pages.credential()
    return
  }
});



// let set_value = async (e) => {
//   let config = await invoke("get_config")
//   console.log(config)

//   let id = e.srcElement.id
//   let value = e.srcElement.checked

//   //thread count ticker thing
//   if (id == "thread_count"){
//     config["thread_count"] = parseInt(e.srcElement.value)
//     await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
//     return
//   }

//   // disable options related to image download, send back-end respective information
//   if (id == "image_download"){
//     document.querySelector("#image_options")
//       .querySelectorAll("input")
//       .forEach(async elem => {
//         elem.disabled = !value
//         elem.checked = value
//         config[elem.id] = value
//       });

//       config["image_download"] = false
//       await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
//       return
//   }

//   // disable options related to ID3, send back-end respective information
//   if (id == "ID3"){
//     document.querySelector("#ID3_options")
//       .querySelectorAll("input")
//       .forEach(elem => {
//         elem.disabled = !value
//         elem.checked = value
//         let id_parsed = elem.id.replace("ID3_", "").toLowerCase()
//         config.id3_options[id_parsed] = value
//     });

//     await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
//     return
//   }

//   //any ID3 standalone option 
//   if(id.startsWith("ID3_")){
//     let id_parsed = id.replace("ID3_", "").toLowerCase()
//     config.id3_options[id_parsed] = value
//     await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
//     return
//   }

//   //and outher standalone option
//   config[id] = value
//   await invoke("write_config_from_string", {newConfig: JSON.stringify(config)})
// }


