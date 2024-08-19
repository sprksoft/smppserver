
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

socketmgr.on_join = () => {
  ui_show_login(false);
}

socketmgr.on_leave = (reason) => {
  ui_clear_messages();
  ui_error(reason);
}

socketmgr.on_message = (me, sender_id, sender_username, message) => {
  console.log("Got message from "+sender_id+" : "+message);
  if (me){ // message comes from me
    ui_remove_pending(message);
  }
  ui_add_message(message, sender_username);

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
  let message = ui_read_input();
  if (message == "/clearkey"){
    localStorage.removeItem("key");
    ui_add_message("key cleared.", "system");
    return;
  }
  ui_add_pending(message);
  socketmgr.send(message);
}

connectbtn.addEventListener("click", ()=>{
  let local_name = ui_get_name();
  socketmgr.join(localStorage.getItem("key"), local_name);
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

ui_show_login(true);
