// View other options of Public Nano Nodes: https://publicnodes.somenano.com
// https://docs.nano.org/commands/rpc-protocol/#node-rpcs

use reqwest::*;
use serde::{de::DeserializeOwned, Deserialize};
use std::array::IntoIter;
use std::collections::HashMap;
use std::iter::FromIterator;

pub struct ClientRpc {
    server_addr: String,
    client: reqwest::Client, // todo: make trait object based on protocol??
}

#[derive(Deserialize, Debug)]
struct RPCPendingResponse {
    blocks: Vec<String>,
}

impl ClientRpc {
    pub fn new(addr: &str) -> Result<ClientRpc> {
        let client = reqwest::Client::builder().build()?;
        Ok(ClientRpc {
            server_addr: String::from(addr),
            client: client,
        })
    }

    pub async fn connect(&self) -> Result<()> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([("action", "version")]));
        let v = self.rpc_post::<HashMap<String, String>>(r).await;
        match v {
            Err(e) => eprintln!(
                "\n node connection unsucessful. please try a different node.\n error: {:#?}",
                e
            ),
            Ok(v) => println!("\n node connection successful:\n {:#?}\n", v),
        }
        Ok(())
    }

    // https://docs.nano.org/commands/rpc-protocol/#pending
    pub async fn pending(&self, addr: &str) -> Option<Vec<String>> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([
            ("action", "pending"),
            ("account", addr),
            ("include_active", "true"),
        ]));
        let v = self.rpc_post::<RPCPendingResponse>(r).await;
        match v {
            Err(e) => {
                eprintln!("\n rpc pending failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => Some(v.blocks),
        }
    }

    async fn rpc_post<T>(&self, r: HashMap<&str, &str>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let v = self
            .client
            .post(&self.server_addr)
            .json(&r)
            .send()
            .await?
            .json::<T>()
            .await?;
        Ok(v)
    }
}
