use log::*;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::handshake::server::{Request, Response},
    tungstenite::http::StatusCode,
};

use crate::chat::Chat;

pub async fn handle(stream: TcpStream, chat: &mut Chat) {
    let mut query = None;
    match accept_hdr_async(stream, |request: &Request, response: Response| {
        if request.uri().path() == "/smpp/gc/v1/socket" {
            query = request.uri().query().map(|v| v.to_string());
            return Ok(response);
        }

        let resp = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Some("smppgc: Not Found.".into()))
            .unwrap();
        Err(resp)
    })
    .await
    {
        Ok(ws) => {
            chat.handle_ws(ws, query).await;
        }
        Err(err) => {
            error!("Failed to accept WebSocket connection: {}", err)
        }
    };
}
