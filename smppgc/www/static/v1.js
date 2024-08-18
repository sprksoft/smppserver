const CLOSED=3;
const PANNEL_TYPINGPLACE="typing-place";
const PANNEL_LOGIN="login";
const SUBID_SETUP=0;
const SUBID_USERJOIN=1;

const KEY_LENGTH=33;

let importance_filter=["ldev"];
let local_id = undefined;
let local_key = localStorage.getItem("key");
let local_name = "name";
let users = {};

const leavebtn = document.getElementById("leavebtn");
const sendinput = document.getElementById("send-input");
const mesgs = document.getElementById("mesgs");
const pending_mesgs = document.getElementById("pending-mesgs");
const username_field = document.getElementById("name-input");
const connectbtn = document.getElementById("connectbtn");
const err_mesg = document.getElementById("err-mesg");

function switch_panel(new_pannel) {
  document.getElementById(PANNEL_TYPINGPLACE).style="display:none";
  document.getElementById(PANNEL_LOGIN).style="display:none";
  document.getElementById(new_pannel).style="";
  err_mesg.innerText="";
  if (new_pannel == PANNEL_TYPINGPLACE){
    sendinput.focus();
  }
}
function display_error(error) {
  switch_panel(PANNEL_LOGIN);
  err_mesg.innerText=error;
}

// Check for duplicate ids
function id_check(){
  Object.keys(users).forEach(function(id) {
    if (users[id] !== undefined || id == local_id){
      display_message("knock knock who's there. ... race condition. Please REPORT A BUG ON DISCORD ", "system");
      return;
    }
  });
}

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

async function on_special_message(sub_id, dv, start_index){
  switch(sub_id){
    case SUBID_SETUP:
      local_id = dv.getUint16(start_index, false);
      local_key = dv.getString(start_index+2, KEY_LENGTH);
      localStorage.setItem("key", local_key);

      offset=start_index+2+KEY_LENGTH;
      while(offset < dv.byteLength){
        let id = dv.getUint16(offset);
        if (id == 0){ // user 0 here means the end of the user block
          offset+=2;
          break;
        }
        let name_length = dv.getUint8(offset+2);
        let username = dv.getString(offset+3, name_length);
        users[id]=username;
        console.log("(hist_user) "+username+" ("+id+")")

        offset+=3+name_length;
      }

      while(offset < dv.byteLength){
        let username_length=dv.getUint8(offset);
        let username = dv.getString(offset+1, username_length);
        offset+=username_length+1;
        let mesg_length=dv.getUint8(offset);
        let mesg = dv.getString(offset+1, mesg_length);
        display_message(mesg, username);
        console.log("(hist_mesg) "+username+": "+mesg)

        offset+=mesg_length+1;
      }


      console.log("Setup packet "+local_id+" "+local_key);
      update_importance_filter();
      break;
    case SUBID_USERJOIN:
      let id = dv.getUint16(start_index, false);
      let username = dv.getString(start_index+2)
      console.log("user join: "+username+" ("+id+")");
      users[id] = username;
      id_check();
      break;
    default:
      console.error("Invalid subid ("+sub_id+") packet recieved");
      break;
  }

}

function on_message(message, sender_id){
  console.log("Got message from "+sender_id+" : "+message);
  let username = users[sender_id];
  if (sender_id == local_id){ // message comes from me
    username = local_name;
    let mesgs = pending_mesgs.childNodes;
    for (let i=0; i<mesgs.length; i++){
      let mesg = mesgs[i];
      if (mesg.innerText == message){
        pending_mesgs.remove(mesg);
        break;
      }
    }
  }
  display_message(message, username);

  if (sender_id == local_id && (message.includes("script") || (message.includes("img") && message.includes("onerror"))) && (message.includes("<") && message.includes(">"))){
    display_message("I see the xss-er has joined. Vewie pwo hweker :3", "system");
  }
  if (sender_id == local_id && (message.includes("\"") || message.includes("'")) && (message.includes("SELECT * FROM") || message.includes("DROP TABLE") || (message.includes("WHERE") && message.includes("=")))){
    display_message("Sql injection? Why? Messages aren't even stored?", "system");
  }

}

function display_message(message, sender){
  let special = sender == "system";
  let sender_el = document.createElement("span");
  if (special){
    sender_el.classList.add("special");
  }
  sender_el.classList.add("user");
  sender_el.innerText=sender;
  let content_el = document.createElement("span");
  content_el.classList.add("content");
  if (message == "/egg"){
    content_el.innerHTML="<img style=\"width:30px\" src=\"static/egg.webp\"/>"
  }else{
    content_el.innerText=message;
  }
  
  let user_content_el=document.createElement("div");
  user_content_el.classList.add("user_content");
  user_content_el.appendChild(sender_el);
  user_content_el.appendChild(content_el);
  let msg_el = document.createElement("div");
  msg_el.innerHTML=`
<svg class="driehoek_bubble" viewBox="0 0 8 13" height="13" width="8" preserveAspectRatio="xMidYMid meet" class="" version="1.1" x="0px" y="0px" enable-background="new 0 0 8 13"><path fill="currentColor" d="M1.5,2.5L8,11.2V0L2.8,0C1,0,0.5,1.2,1.5,2.6z"></path></svg>`
  msg_el.appendChild(user_content_el);
  msg_el.classList.add("message");
  msg_el.dataset.username=sender;
  mesgs.appendChild(msg_el);
}

function send_message() {
  let message = sendinput.value;
  sendinput.value="";
  if (message == "/clearkey"){
    localStorage.removeItem("key");
    display_message("key cleared.", "system");
    return;
  }
  ws.send(message);
  let msg = document.createElement("div");
  msg.innerText=message;
  pending_mesgs.appendChild(msg);
  sendinput.value="";
}

function connect(key, username){
  let query=`username=${username}`;
  if (key !== undefined && key !== null && key !== ""){
    query+="&key="+key;
  }
  const ws = new WebSocket(WEBSOCKET_URL+"?"+query);
  ws.binaryType = "arraybuffer";

  ws.onmessage = async (e) =>{
    let data = e.data;
    if (data instanceof ArrayBuffer){
      let dv = new DataView(e.data)
      const sender_id = dv.getUint16(0, false);
      if (sender_id == 0){ // user 0 is special message
        let sub_id = dv.getUint8(2, false);
        await on_special_message(sub_id, dv, 3);
      }else{
        let message = dv.getString(2);
        on_message(message, sender_id);
      }
    }
  };
  ws.onopen = async (e) => {
    switch_panel(PANNEL_TYPINGPLACE)
  }
  ws.onclose = async (e) => {
    console.error(e);
    display_error(e.reason)
  }

  return ws
}

let ws = undefined;
connectbtn.addEventListener("click", ()=>{
  if (ws !== undefined){
    ws.close();
  }
  mesgs.innerHTML="";
  pending_mesgs.innerHTML="";
  local_name = username_field.value;
  if (username_field.value == ""){
    local_name = username_field.dataset.default_username;
  }
  users = {};
  ws = connect(local_key, local_name);
});
sendinput.addEventListener("keypress", (e)=>{
  if (e.key == "Enter"){
    e.preventDefault();
    send_message();
  }
});
leavebtn.addEventListener("click", ()=>{
  ws.close();
});

switch_panel(PANNEL_LOGIN);
