// This file is generated by gen_js.sh (do not modify)
/* == smppgc/js/ui.js == */
const leavebtn = document.getElementById("leavebtn");
const sendinput = document.getElementById("send-input");
const mesgs = document.getElementById("mesgs");
const pending_mesgs = document.getElementById("pending-mesgs");
const username_field = document.getElementById("name-input");
const connectbtn = document.getElementById("connectbtn");
const err_mesg = document.getElementById("err-mesg");

const login_popup=document.getElementById("login");

function ui_show_login(show) {
  if (show){
    login_popup.style="";
    ui_clear_messages();
    sendinput.disabled=true;
  }else{
    login_popup.style="display:none";
    sendinput.disabled=false;
    sendinput.focus();
  }
}


function ui_error(error) {
  ui_show_login(true);
  err_mesg.innerText=error;
}

function ui_get_name() {
  let local_name = username_field.value;
  if (username_field.value == ""){
    local_name = username_field.dataset.default_username;
  }
  return local_name;
}

// Read the input message and clear it
function ui_get_input() {
  return sendinput.value.trim();
}
function ui_clear_input() {
  sendinput.value="";
}

function ui_clear_messages() {
  mesgs.innerHTML="";
  pending_mesgs.innerHTML="";
}

function ui_add_pending(message) {
  let msg = document.createElement("div");
  msg.innerText=message;
  pending_mesgs.appendChild(msg);
}

function ui_remove_pending(message) {
  let mesgs = pending_mesgs.childNodes;
  for (let i=0; i<mesgs.length; i++){
    let mesg = mesgs[i];
    if (mesg.innerText == message){
      pending_mesgs.removeChild(mesg);
      break;
    }
  }
}

function mkspan(innerText, parent_el){
    let span = document.createElement("span");
    span.innerText=innerText;
    parent_el.appendChild(span);
}

// convert urls into html tags
function format_urls(message, parent_el) {
  const find_link_regex = /https?:\/\/[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b[-a-zA-Z0-9()@:%_\+.~#?&//=]*/g;

  const matches = message.matchAll(find_link_regex);
  let last_index = 0;
  for (const match of matches){
    mkspan(message.substring(last_index, match.index), parent_el);
    
    let a = document.createElement("a");
    a.href=match[0];
    a.innerText=match[0];
    parent_el.appendChild(a);

    last_index = match.index+match[0].length;
  }
  mkspan(message.substring(last_index), parent_el);
}

async function ui_add_message(message, sender){
  let special = sender == "system";
  let sender_el = document.createElement("span");
  if (special){
    sender_el.classList.add("special");
  }
  sender_el.classList.add("user");
  sender_el.innerText=sender;
  let content_el = document.createElement("span");
  content_el.classList.add("content");
  format_urls(message, content_el);
  
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
  msg_el.scrollIntoView();
}
/* == smppgc/js/ws.js == */
const CLOSED=3;
const SUBID_SETUP=0;
const SUBID_USERJOIN=1;
const KEY_LENGTH=33;


class SocketMgr{
  on_message;
  on_leave;
  on_join;
  on_keychange;

  #local_id;
  #users;

  constructor(){
    this.users={};
  }
  // Check for duplicate ids
  #id_check(){
    Object.keys(this.users).forEach(function(id) {
      if (this.users[id] !== undefined || id == this.local_id){
        ui_add_message("knock knock who's there. ... race condition. Please REPORT A BUG ON DISCORD ", "system");
        return;
      }
    });
  }


  #on_special_message(sub_id, dv, start_index){
    switch(sub_id){
      case SUBID_SETUP:
        this.local_id = dv.getUint16(start_index, false);
        this.local_key = dv.getString(start_index+2, KEY_LENGTH);
        this.on_keychange(this.local_key);

        let offset=start_index+2+KEY_LENGTH;
        while(offset < dv.byteLength){
          let id = dv.getUint16(offset);
          if (id == 0){ // user 0 here means the end of the user block
            offset+=2;
            break;
          }
          let name_length = dv.getUint8(offset+2);
          let username = dv.getString(offset+3, name_length);
          this.users[id]=username;
          console.log("(hist_user) "+username+" ("+id+")")

          offset+=3+name_length;
        }

        while(offset < dv.byteLength){
          let username_length=dv.getUint8(offset);
          let username = dv.getString(offset+1, username_length);
          offset+=username_length+1;
          let mesg_length=dv.getUint8(offset);
          let message = dv.getString(offset+1, mesg_length);
          this.on_message(this.local_id, -1, username, message);

          offset+=mesg_length+1;
        }


        console.log("Setup packet "+this.local_id+" "+this.local_key);
        this.on_join();
        break;
      case SUBID_USERJOIN:
        let id = dv.getUint16(start_index, false);
        let username = dv.getString(start_index+2)
        console.log("user join: "+username+" ("+id+")");
        this.users[id] = username;
        this.#id_check();
        break;
      default:
        console.error("Invalid subid ("+sub_id+") packet recieved");
        break;
    }

  }

  async join(key, username){
    if (this.ws !== undefined){
      await this.ws.close();
    }
    let query=`username=${username}`;
    if (key !== undefined && key !== null && key !== ""){
      query+="&key="+key;
    }
    this.ws = new WebSocket(WEBSOCKET_URL+"?"+query);
    this.ws.binaryType = "arraybuffer";

    this.ws.onmessage = async (e) =>{
      let data = e.data;
      if (data instanceof ArrayBuffer){
        let dv = new DataView(e.data)
        const sender_id = dv.getUint16(0, false);
        if (sender_id == 0){ // user 0 is special message
          let sub_id = dv.getUint8(2, false);
          this.#on_special_message(sub_id, dv, 3);
        }else{
          let message = dv.getString(2);
          let sender_username = this.users[sender_id];
          let me = this.local_id == sender_id;
          if (me){
            sender_username = username;
          }
          this.on_message(me, sender_id, sender_username, message);
        }
      }
    };
    this.ws.onclose = async (e) => {
      console.error(e);
      this.users={};
      this.on_leave(e.reason);
    }
  }

  async send(message){
    if (this.ws.bufferedAmount > 2){
      return false;
    }
    await this.ws.send(message);
    return true;
  }

  async leave(){
    await this.ws.close();
  }

}
/* == smppgc/js/index.js == */

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
