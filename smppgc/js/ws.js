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
          //console.log("(hist_user) "+username+" ("+id+")")

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


        //console.log("Setup packet "+this.local_id+" "+this.local_key);
        this.on_join();
        break;
      case SUBID_USERJOIN:
        let id = dv.getUint16(start_index, false);
        let username = dv.getString(start_index+2)
        //console.log("user join: "+username+" ("+id+")");
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
    let encoded_username = encodeURIComponent(username);
    let query=`username=${encoded_username}`;
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
      this.users={};
      let reason = e.reason;
      if (!e.reason || e.reason.startsWith("INT:")){
        console.error("Reason empty or internal error");
        console.error(e);
        reason="Onverwachte fout.";
      }
      this.on_leave(e.code, reason);
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
    await this.ws.close(1000, "Dag dag ik ga je missen. xxx");
  }

}
