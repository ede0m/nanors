use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

pub struct ClientWS {
    server_addr: String,
    stream: Box<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    // todo: accounts list?
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WSConfirmationResp {
    topic: String,
    time: String,
    message: WSConfirmationMessage,
}

// https://docs.nano.org/integration-guides/websockets/#confirmations
#[derive(Serialize, Deserialize, Debug)]
pub struct WSConfirmationMessage {
    pub account: String,
    pub amount: String,
    pub hash: String,
}

#[derive(Serialize, Deserialize)]
struct WSConfirmationReq {
    action: String,
    topic: String,
    options: WSConfirmationOptionsReq,
}

#[derive(Serialize, Deserialize)]
struct WSConfirmationOptionsReq {
    accounts: Vec<String>,
}

impl ClientWS {
    pub async fn new(host: &str) -> Result<ClientWS, Box<dyn std::error::Error>> {
        let url = Url::parse(host)?;
        let ws_stream = match connect_async(url).await {
            Ok(s) => Box::new(s.0),
            Err(e) => return Err(format!("failed to connect to stream: {:?}", e).into()),
        };
        let ws = ClientWS {
            server_addr: String::from(host),
            stream: ws_stream,
        };
        Ok(ws)
    }

    pub async fn subscribe_confirmation(
        &mut self,
        accounts: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let req = WSConfirmationReq {
            action: String::from("subscribe"),
            topic: String::from("confirmation"),
            options: WSConfirmationOptionsReq { accounts: accounts },
        };
        let req = match serde_json::to_string(&req) {
            Ok(req) => req,
            Err(e) => return Err(e.into()),
        };
        self.stream.send(Message::text(req)).await?;
        Ok(())
    }

    pub async fn watch_confirmation(
        &mut self,
        sender: mpsc::Sender<WSConfirmationMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(msg) = self.stream.next().await {
            let msg = msg?.into_text()?;
            let c: WSConfirmationResp = serde_json::from_str(msg.as_str())?;
            sender.send(c.message).await?;
        }
        Err("ws: confirmation ended".into())
    }

    pub async fn update_confirmation(&self) {
        unimplemented!();
    }
}
