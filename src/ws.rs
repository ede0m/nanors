use crate::block;

use futures::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct WSNanoResp {
    topic: Option<String>,
    time: Option<String>,
    ack: Option<String>,
    message: Option<WSConfirmationMessage>,
}

// https://docs.nano.org/integration-guides/websockets/#confirmations
#[derive(Serialize, Deserialize, Debug)]
pub struct WSConfirmationMessage {
    pub account: String,
    pub amount: String,
    pub hash: String,
    pub block: block::NanoBlock,
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

// todo:? generalize to run, and move subscribe logic
pub async fn subscribe_confirmation(
    ws_host: &str,
    accounts: Vec<String>,
    sender: mpsc::Sender<WSConfirmationMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse(ws_host)?;
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, read) = ws_stream.split();
    subscribe_confirmations(&mut write, accounts).await?;
    let out = tokio::select! {
        res = async {
            watch_connection(read, sender).await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        } => {
            res?;
        }
        res = async {
            keep_alive(&mut write).await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        } => {
            res?;
        }
        // oneshot cancel needed?
    };

    Ok(())
}

async fn watch_connection(
    mut stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    sender: mpsc::Sender<WSConfirmationMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Some(msg) = stream.next().await {
        let msg = msg?.into_text()?;
        let nr: WSNanoResp = serde_json::from_str(msg.as_str())?;
        if nr.message.is_some() {
            //println!("\n\nsend conf:\n\n{:#?}", msg);
            sender.send(nr.message.unwrap()).await?;
        }
    }
    Err("ws: confirmation ended".into())
}

async fn subscribe_confirmations(
    stream: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    accounts: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // subscribe to addresses
    let req = WSConfirmationReq {
        action: String::from("subscribe"),
        topic: String::from("confirmation"),
        options: WSConfirmationOptionsReq { accounts },
    };
    let req = match serde_json::to_string(&req) {
        Ok(req) => req,
        Err(e) => return Err(e.into()),
    };
    stream.send(Message::text(req)).await?;
    Ok(())
}

async fn keep_alive(
    stream: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = WSPingReq {
        action: String::from("ping"),
    };
    loop {
        time::sleep(time::Duration::from_millis(40000)).await;
        let req = serde_json::to_string(&req)?;
        //println!("ping!");
        stream.send(Message::text(req)).await?;
    }
}
