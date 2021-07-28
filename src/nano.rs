// View other options of Public Nano Nodes: https://publicnodes.somenano.com
// https://docs.nano.org/commands/rpc-protocol/#node-rpcs
use crate::wallet;
use reqwest::*;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use std::array::IntoIter;
use std::collections::HashMap;
use std::iter::FromIterator;

pub struct ClientRpc {
    server_addr: String,
    client: reqwest::Client, // todo: make trait object based on protocol??
}

#[derive(Deserialize, Debug)]
pub struct RPCAccountInfoResp {
    pub frontier: String,
    open_block: String,
    representative_block: String,
    pub representative: String,
    pub balance: String,
    modified_timestamp: String,
    block_count: String,
    account_version: String,
    confirmation_height: String,
    confirmation_height_frontier: String,
}

#[derive(Deserialize, Debug)]
pub struct RPCPendingResp {
    pub blocks: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct RPCProcessResp {
    pub hash: String,
}

#[derive(Deserialize, Debug)]
pub struct RPCBlockInfoResp {
    block_account: String,
    pub amount: String,
    balance: String,
    height: String,
    local_timestamp: String,
    confirmed: String,
    pub subtype: String,
    pub contents: wallet::NanoBlock,
}

#[derive(Deserialize, Debug)]
pub struct RPCTelemetryResp {
    block_count: String,
    peer_count: String,
    major_version: String,
    minor_version: String,
    patch_version: String,
    pub active_difficulty: String,
}

impl ClientRpc {
    pub fn new(addr: &str) -> Result<ClientRpc> {
        let client = reqwest::Client::builder().build()?;
        Ok(ClientRpc {
            server_addr: String::from(addr),
            client: client,
        })
    }

    pub async fn connect(&self) -> Option<RPCTelemetryResp> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([("action", "telemetry")]));
        let v = self.rpc_post::<RPCTelemetryResp>(r).await;
        match v {
            Err(e) => {
                eprintln!(
                    "\n node connection unsucessful. please try a different node.\n error: {:#?}",
                    e
                );
                None
            }
            Ok(v) => {
                println!("connected: {:?}", v);
                v
            }
        }
    }

    pub async fn block_info(&self, hash: &str) -> Option<RPCBlockInfoResp> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([
            ("action", "block_info"),
            ("json_block", "true"),
            ("hash", hash),
        ]));
        match self.rpc_post::<RPCBlockInfoResp>(r).await {
            Err(e) => {
                eprintln!("\nrpc block info failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => {
                //println!("{:?}", v);
                v
            }
        }
    }

    pub async fn account_info(&self, acct: &str) -> Option<RPCAccountInfoResp> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([
            ("action", "account_info"),
            ("representative", "true"),
            ("account", acct),
        ]));
        match self.rpc_post::<RPCAccountInfoResp>(r).await {
            Err(e) => {
                eprintln!("\nrpc block info failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => {
                println!("account info: {:?}", v);
                v
            }
        }
    }

    pub async fn process(
        &self,
        block: &wallet::NanoBlock,
        subtype: &str,
    ) -> Option<RPCProcessResp> {
        let b = serde_json::to_string(block).expect("could not serialize block");
        let r = HashMap::<_, _>::from_iter(IntoIter::new([
            ("action", "process"),
            ("json_block", "true"),
            ("subtype", subtype),
            ("block", &b),
        ]));
        match self.rpc_post::<RPCProcessResp>(r).await {
            Err(e) => {
                eprintln!("\n rpc process failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => {
                println!("processed: {:?}", v);
                v
            }
        }
    }

    // https://docs.nano.org/commands/rpc-protocol/#pending
    pub async fn pending(&self, addr: &str) -> Option<RPCPendingResp> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([
            ("action", "pending"),
            ("account", addr),
            ("include_active", "true"),
        ]));
        match self.rpc_post::<RPCPendingResp>(r).await {
            Err(e) => {
                eprintln!("\n rpc pending failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => v,
        }
    }

    async fn rpc_post<T>(&self, r: HashMap<&str, &str>) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let resp = self.client.post(&self.server_addr).json(&r).send().await?;
        let resp = resp.text().await?;
        //println!("body: {}", resp);
        let resp: Option<T> = match serde_json::from_str(&resp) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("{:?}", e);
                None
            }
        };
        Ok(resp)
    }
}
