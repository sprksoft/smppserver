use rocket::{get, State};
use std::{
    borrow::Cow,
    sync::Arc,
    time::{Duration, Instant},
};

use log::*;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    Channel, WebSocket,
};
use tokio::sync::{broadcast::error::RecvError, Mutex};

use crate::chat::{
    usernamemgr::{Key, NameLeaseError},
    Chat,
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
                stream
                    .close(Some(CloseFrame {
                        code: CloseCode::Error,
                        reason: Cow::Borrowed("Invalid key"),
                    }))
                    .await?;
                return Ok(());
            };
            let name_lease = match name_lease {
                Ok(name_lease) => name_lease,
                Err(e) => {
                    stream
                        .close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::Borrowed(match e {
                                NameLeaseError::Taken => "username taken",
                                NameLeaseError::Invalid => "username is invalid",
                            }),
                        }))
                        .await?;
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
            let (mut messages_receiver, messages_sender, mut join_reciever) =
                chat.subscribe_events();
            let rate_limit = chat.config().rate_limit.clone();
            drop(chat);

            let mut blockme = false;
            let mut burst = 0;
            let mut last_message_instant = Instant::now();
            loop {
                tokio::select! {
                    mesg = client.try_recv() => {
                        let Some(mesg) = mesg? else { return Ok(())};
                        let last_mesg_sec : isize = last_message_instant.elapsed().as_millis().try_into().unwrap_or(isize::MAX);
                        last_message_instant = Instant::now();

                        if last_mesg_sec < rate_limit.min_message_time_hard{
                            client.ratelimit_kick().await?;
                            return Ok(());
                        }
                        burst+=rate_limit.min_message_time_hard.saturating_sub(last_mesg_sec);
                        if burst < 0{
                            burst=0;
                        }
                        if burst > rate_limit.kick_burst{
                            client.ratelimit_kick().await?;
                            return Ok(());
                        }
                        if last_mesg_sec < rate_limit.min_message_time_soft{
                            burst+=rate_limit.min_message_time_soft.saturating_sub(last_mesg_sec)*2.clamp(0, isize::MAX);
                        }
                        if mesg.is_valid(){
                            if mesg.content == "/blockme".into(){
                                blockme=true;
                            }
                            if !blockme{
                                trace!("got message from {}: {}", mesg.sender, mesg.content);
                                let _ = messages_sender.send(mesg);
                            }
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
                                return Ok(());
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
                                return Ok(());
                            }
                        }
                    }
                }
            }
        })
    })
}
