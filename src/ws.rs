use crate::block;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

pub struct ClientWS {
    stream: Box<WebSocketStream<MaybeTlsStream<TcpStream>>>,
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
    pub block: block::NanoBlock
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

#[derive(Serialize, Deserialize)]
struct WSPingReq {
    action: String,
}

impl ClientWS {
    pub async fn new(host: &str) -> Result<ClientWS, Box<dyn std::error::Error>> {
        let url = Url::parse(host)?;
        let ws_stream = match connect_async(url).await {
            Ok(s) => Box::new(s.0),
            Err(e) => return Err(format!("failed to connect to stream: {:?}", e).into()),
        };
        let ws = ClientWS {
            stream: ws_stream,
        };
        Ok(ws)
    }

    pub async fn subscribe_confirmation(
        &mut self,
        accounts: Vec<String>,
        sender: mpsc::Sender<WSConfirmationMessage>,
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
        self.watch_confirmations(sender).await?;
        // TODO: keep alive
        /*
        let out = tokio::select! {
            res = async {
                self.watch_confirmations(sender).await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            } => {
                res?;
            }
            res = async {
                self.ping_forver().await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            } => {
                res?;
            }
            // todo: oneshot cancel?
        };
        */
        
        Ok(())
    }

    async fn watch_confirmations(
        &mut self,
        sender: mpsc::Sender<WSConfirmationMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(msg) = self.stream.next().await {
            println!("from send\n: {:#?}", msg);
            let msg = msg?.into_text()?;
            let c: WSConfirmationResp = serde_json::from_str(msg.as_str())?;
            sender.send(c.message).await?;
        }
        Err("ws: confirmation ended".into())
    }

    async fn ping_forver(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let req = WSPingReq {
            action: String::from("ping")
        };
        loop {
            time::sleep(time::Duration::from_millis(10000)).await;
            let req = serde_json::to_string(&req)?;
            println!("ping!");
            self.stream.send(Message::text(req)).await?;
        }
        Err("ws: ping ended".into())
    }
}
