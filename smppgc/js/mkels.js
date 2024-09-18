function mksender(sender, parent_el) {
  let special = sender == "system";
  let sender_el = document.createElement("span");
  if (special){
    sender_el.classList.add("special");
  }
  sender_el.classList.add("user");
  sender_el.innerText=sender;
  parent_el.appendChild(sender_el);
}
function mkspace(parent_el) {
  let space = document.createElement("div");
  space.classList.add("space");
  parent_el.appendChild(space);
}

function mktime(time, parent_el) {
  if (time == undefined){ return; }
  let time_el = document.createElement("small");
  time_el.classList.add("message_timestamp")
  time_el.innerText = time.toLocaleString(undefined, {
    dateStyle:"short",
    timeStyle:"short",
  });
  parent_el.appendChild(time_el);
}
