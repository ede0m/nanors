// View other options of Public Nano Nodes: https://publicnodes.somenano.com
// https://docs.nano.org/commands/rpc-protocol/#node-rpcs
use crate::wallet;
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
            Ok(v) => Some({
                println!("connected: {:?}", v);
                v.unwrap()
            }),
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
                Some(v.unwrap())
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
            Ok(v) => match v {
                Some(v) => Some(v),
                None => None,
            },
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
            Ok(v) => Some(v.unwrap()),
        }
    }

    async fn rpc_post<T>(&self, r: HashMap<&str, &str>) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let resp = self.client.post(&self.server_addr).json(&r).send().await?;
        match resp.json::<T>().await {
            Ok(t) => Ok(Some(t)),
            Err(_) => Ok(None),
        }
    }
}
