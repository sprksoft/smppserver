use rocket::{get, Responder, State};
use std::{borrow::Cow, sync::Arc, time::Instant};

use log::*;
use rocket_db_pools::Connection;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    Channel, WebSocket,
};
use tokio::sync::{broadcast::error::RecvError, Mutex};

use crate::{
    chat::{
        usernamemgr::{NameLeaseError, UserId},
        Chat,
    },
    db,
    mesg_filter::{self, Cmd, FilterResult},
    OfflineConfig,
};

#[derive(Responder)]
pub enum SocketV1Responder {
    #[response(status = 503)]
    Offline(&'static str),
    #[response(status = 500)]
    Error(&'static str),
    #[response(status = 200)]
    Channel(Channel<'static>),
}

#[get("/socket/v1?<username>&<key>")]
pub async fn socket_v1(
    username: &str,
    key: Option<&str>,
    ws: WebSocket,
    offline_config: &State<OfflineConfig>,
    chat: &State<Arc<Mutex<Chat>>>,
    mut db: Connection<db::Db>,
) -> SocketV1Responder {
    if offline_config.offline {
        return SocketV1Responder::Offline("smppgc offline");
    }
    let key = if let Some(user_id) = key {
        UserId::parse_str(user_id)
    } else {
        Some(UserId::new())
    };

    let chat: Arc<Mutex<Chat>> = chat.inner().clone();
    let name_lease = match key.clone() {
        Some(key) => {
            let mut chat = chat.lock().await;
            chat.unmgr_mut().lease_name(username, key, &mut **db).await
        }
        None => Err(NameLeaseError::Invalid),
    };
    if let Err(NameLeaseError::Db(ref e)) = name_lease {
        error!("db error: {:?}", e);
    }

    SocketV1Responder::Channel(ws.channel(move |mut stream| {
        Box::pin(async move {
            let Some(key) = key else {
                stream
                    .close(Some(CloseFrame {
                        code: CloseCode::Error,
                        reason: Cow::Borrowed("INT: Ongeldige sleutel."),
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
                            reason: Cow::Owned(e.to_string()),
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
                        let Some(mesg) = mesg? else { continue; };
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
                        match mesg_filter::filter(mesg){
                            FilterResult::Cmd(Cmd::BlockMe) => {
                                blockme=true;
                            },
                            FilterResult::Cmd(Cmd::KillMe) => {
                                return Ok(());
                            }
                            FilterResult::Invalid => {},
                            FilterResult::Message(mesg) => {
                                if !blockme{
                                    trace!("got message from {}: {}", mesg.sender, mesg.content);
                                    let _ = messages_sender.send(mesg);
                                }
                            }
                        }


                    }
                    mesg = messages_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                client.forward(&mesg).await?;
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
    }))
}
