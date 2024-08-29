const leavebtn = document.getElementById("leavebtn");
const sendinput = document.getElementById("send-input");
const mesgs = document.getElementById("mesgs");
const pending_mesgs = document.getElementById("pending-mesgs");
const username_field = document.getElementById("name-input");
const connectbtn = document.getElementById("connectbtn");
const err_info_mesg = document.getElementById("err-info-mesg");

const login_popup=document.getElementById("login");

const STICKERS=["404", "arch", "tux", "smpp"]; // avail stickers (used to prevent unneeded 404s to the server)

function ui_show_login(show) {
  if (show){
    login_popup.style="";
    sendinput.disabled=true;
    ui_clear_messages();
  }else{
    login_popup.style="display:none";
    sendinput.disabled=false;
    sendinput.focus();
  }
}


function ui_error(error) {
  if (error != ""){
    ui_show_login(true);
  }
  err_info_mesg.className="err";
  err_info_mesg.innerText=error;
}
function ui_info(info){
  if (info != ""){
    ui_show_login(true);
  }
  err_info_mesg.className="info";
  err_info_mesg.innerText=info;
}

function ui_set_name(name) {
  username_field.value = name;
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
function mka(link, parent_el) {
    let a = document.createElement("a");
    a.href=link;
    a.innerText=link;
    parent_el.appendChild(a);
}
function mksticker(name, parent_el) {
    let img = document.createElement("img");
    img.width=50;
    img.dataset.sticker=name
    img.src=ROOT_URL+"/static/stickies/"+name+".webp";
    parent_el.appendChild(img);
}


// convert urls into html tags
function format_urls(message, parent_el) {
  const find_link_regex = /(https?:\/\/[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b[-a-zA-Z0-9()@:%_\+.~#?&//=]*)|(:[a-z0-9_-]{3,10}:)/g;

  const matches = message.matchAll(find_link_regex);
  let last_index = 0;
  for (const match of matches){
    let skip=false;
    mkspan(message.substring(last_index, match.index), parent_el);

    if (match[1] !== undefined){
      mka(match[1], parent_el)
    }
    if (match[2] !== undefined){
      let name = match[2].substring(1, match[2].length-1);
      if (STICKERS.includes(name)){
        mksticker(name, parent_el);
      }else{
        skip=true;
      }
    }
    if (!skip){
      last_index = match.index+match[0].length;
    }

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
