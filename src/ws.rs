use tokio_tungstenite::{
    connect_async,
    WebSocketStream,
    MaybeTlsStream,
    tungstenite::{Message},
};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};


pub struct ClientWS {
    server_addr: String,
    agent: String,
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    // todo: accounts list?
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

    pub async fn new(addr: &str, agent: &str) -> Result<ClientWS, Box<dyn std::error::Error>> {
        let (mut ws_stream, _) = match connect_async(addr).await {
            Ok(s) => s,
            Err(e) => return Err("failed to connect to stream".into()),
        };
        Ok(ClientWS {
            server_addr: String::from(addr),
            agent: String::from(agent), 
            stream: ws_stream,
        })
    }

    pub async fn confirmation(&mut self) -> Result<(), Box<dyn std::error::Error>> {

        //Stream recv block confirmations...
        unimplemented!();
    }
    
    pub async fn sub_confirmation(&mut self, accounts: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let req = WSConfirmationReq {
            action: String::from("subscribe"),
            topic: String::from("confirmation"),
            options: WSConfirmationOptionsReq {
                accounts: accounts,
            },
        };
        let req = match serde_json::to_string(&req){
            Ok(req) => req,
            Err(e) => return Err(e.into()),
        };
        self.stream.send(Message::text(req)).await?;
        /*
        while let Some(msg) = self.stream.next().await {
            let msg = msg?;
        }

        self.stream.send()
        */

        Ok(())
    }

    pub async fn update_confirmation(&self) {
        unimplemented!();
    }
 

}

