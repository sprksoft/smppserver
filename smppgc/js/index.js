
let importance_filter=["ldev"];


function update_importance_filter() {
  let css = "";
  let css_driehoek="";
  for (let i=0; i < importance_filter.length; i++){
    let name = importance_filter[i];
    css+=".message[data-username=\""+name+"\"]"
    css_driehoek+=".message[data-username=\""+name+"\"] > .driehoek_bubble";
    if (i !== importance_filter.length-1){
      css+=",";
      css_driehoek+=",";
    }
  }
  css+=`{
  align-self:end;
  text-align:right;
  border-top-left-radius: 10px;
  border-top-right-radius: 0px;
}`;
  css_driehoek+=`{
  right:-18px;
  left:unset;
  order: 2;
  transform: rotateY(180deg);
}`;
  document.getElementById("importance_filter").innerText = css+"\n"+css_driehoek;
}

let socketmgr = new SocketMgr();

let last_retry = 0;

socketmgr.on_join = () => {
  ui_info("");
  ui_show_login(false);
}

socketmgr.on_leave = (code, reason) => {
  switch (code) {
    case 1000: // Normal Closure
      ui_show_login(true);
      return;
    case 1006: // Abnormal Closure
      let now = Date.now();
      if (last_retry == 0 || now-last_retry > 10_000){
        last_retry = now;
        socketmgr.join(localStorage.getItem("key"), localStorage.getItem("username")); //TODO: I don't like to read localStorage here. Socketmgr should auto reconnect maybe?
        return;
      }
      ui_error("Onverwachten fout.");
      return;
  }
  ui_error(reason);
}

socketmgr.on_message = (me, sender_id, sender_username, timestamp, message) => {
  console.log("Got message from "+sender_id+" : "+message);
  if (me){ // message comes from me
    ui_remove_pending(message);
  }
  ui_add_message(message, sender_username, timestamp);

  if (me && (message.includes("script") || (message.includes("img") && message.includes("onerror"))) && (message.includes("<") && message.includes(">"))){
    ui_add_message("I see the xss-er has joined. Vewie pwo hweker :3", "system");
  }
  if (me && (message.includes("\"") || message.includes("'")) && (message.includes("SELECT * FROM") || message.includes("DROP TABLE") || (message.includes("WHERE") && message.includes("=")))){
    ui_add_message("Sql injection? Why? Messages aren't even stored?", "system");
  }
}

socketmgr.on_keychange = (key) => {
  localStorage.setItem("key", key);
}


function send_message() {
  let message = ui_get_input();
  if (message.length == 0){
    return;
  }
  if (message == "/clearkey"){
    localStorage.removeItem("key");
    ui_add_message("key cleared.", "system");
    return;
  }
  if (socketmgr.send(message)){
    ui_add_pending(message);
    ui_clear_input();
  }
}

function join() {
  let local_name = ui_get_name();
  localStorage.setItem("username", local_name);
  ui_info("connecting...");
  socketmgr.join(localStorage.getItem("key"), local_name);
}

connectbtn.addEventListener("click", ()=>{
  join();
});
sendinput.addEventListener("keypress", (e)=>{
  if (e.key == "Enter"){
    e.preventDefault();
    send_message();
  }
});
leavebtn.addEventListener("click", ()=>{
  socketmgr.leave();
});

ui_set_name(localStorage.getItem("username"));
ui_show_login(true);
if (SKIP_LOGIN){
  join();
}

