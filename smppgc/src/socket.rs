use futures_util::StreamExt;
use rocket::{get, State};
use std::{borrow::Cow, sync::Arc};

use log::*;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    Channel, WebSocket,
};
use tokio::sync::{broadcast::error::RecvError, Mutex};

use crate::{
    chat2::Chat,
    client::PacketError,
    usernamemgr::{Key, NameLeaseError},
};

#[get("/socket/v1?<username>&<key>")]
pub async fn socket_v1(
    username: &str,
    key: Option<&str>,
    ws: WebSocket,
    chat: &State<Arc<Mutex<Chat>>>,
) -> Channel<'static> {
    let key = if let Some(key) = key {
        Key::parse_str(key)
    } else {
        Some(Key::new())
    };

    let chat: Arc<Mutex<Chat>> = chat.inner().clone();
    let name_lease = match key.clone() {
        Some(key) => {
            let mut chat = chat.lock().await;
            chat.unmgr_mut().lease_name(username, key)
        }
        None => Err(NameLeaseError::Invalid),
    };

    ws.channel(move |mut stream| {
        Box::pin(async move {
            let Some(key) = key else {
                let _ = stream.close(Some(CloseFrame {
                    code: CloseCode::Error,
                    reason: Cow::Borrowed("Invalid key"),
                }));
                return Ok(());
            };
            let name_lease = match name_lease {
                Ok(name_lease) => name_lease,
                Err(e) => {
                    let _ = stream.close(Some(CloseFrame {
                        code: CloseCode::Error,
                        reason: Cow::Borrowed(match e {
                            NameLeaseError::Taken => "username taken",
                            NameLeaseError::Invalid => "username is invalid",
                        }),
                    }));
                    info!("invalid or taken name");
                    return Ok(());
                }
            };

            let mut chat = chat.lock().await;
            let mut client = match chat.new_client(stream, key, name_lease).await {
                Ok(c) => c,
                Err(e) => {
                    info!("Closing connection: {:?}", e);
                    return Ok(());
                }
            };

            let (mut messages_receiver, messages_sender, mut join_reciever) = chat.subscribe();
            loop {
                tokio::select! {
                    mesg = client.try_recv() => {
                        let mesg = mesg?;
                        if mesg.is_valid(){
                            trace!("got message from {}: {}", mesg.sender, mesg.content);
                            let _ = messages_sender.send(mesg);
                        }

                    }
                    mesg = messages_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                client.forward(&mesg).await                                ?;
                            }
                            Err(RecvError::Lagged(count)) => {
                                error!("{} Messages lost", count);
                            },
                            Err(RecvError::Closed)=>{
                                break;
                            }
                        }
                    }
                    joined_client = join_reciever.recv() => {
                        match joined_client{
                            Ok(joined_client) => {
                                info!("user join {}", joined_client.id());
                                client.forward_client(&joined_client).await?;
                            },
                            Err(RecvError::Lagged(count)) => {
                                error!("{} Join messages lost", count);
                            }, Err(RecvError::Closed)=>{
                                break;
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    })
}
