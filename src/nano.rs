

// View other options of Public Nano Nodes: https://publicnodes.somenano.com
// https://docs.nano.org/commands/rpc-protocol/#node-rpcs

use reqwest::*;
use std::collections::HashMap;
use std::array::IntoIter;
use std::iter::FromIterator;

pub struct ClientRpc {
    server_addr : String,
    client : reqwest::Client // todo: make trait object based on protocol??
}

impl ClientRpc {
    
    pub fn new(addr : &str) -> Result<ClientRpc> {
        let client = reqwest::Client::builder().build()?;
        Ok(ClientRpc {
            server_addr: String::from(addr), 
            client : client
        })
    }

    pub async fn connect(&self) -> Result<()> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([("action", "version")]));
        let v = self.rpc_post(r).await;
        match v {
            Err(e) => eprintln!("\n node connection unsucessful. please try a different node.\n error: {:#?}", e),
            Ok(v) => println!("\n node connection successful:\n {:#?}\n", v)
        }
        Ok(())
    }

    async fn rpc_post(&self, r : HashMap<&str, &str>) -> Result<HashMap<String, String>> {
        let v = self.client.post(&self.server_addr)
            .json(&r)
            .send()
            .await?
            .json::<HashMap<String, String>>()
            .await?;
        Ok(v)
    }
}    

