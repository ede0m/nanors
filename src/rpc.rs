// View other options of Public Nano Nodes: https://publicnodes.somenano.com
// https://docs.nano.org/commands/rpc-protocol/#node-rpcs
use crate::block;
use reqwest::Client;
use serde::{
    de::{DeserializeOwned, IntoDeserializer},
    Deserialize, Deserializer, Serialize,
};
use std::array::IntoIter;
use std::collections::HashMap;
use std::iter::FromIterator;

pub struct ClientRpc {
    server_addr: String,
    client: Box<reqwest::Client>,
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
    //#[serde(deserialize_with = "empty_string_as_none")]
    pub blocks: Option<Vec<String>>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    let opt = opt.as_ref().map(String::as_str);
    match opt {
        None | Some("") => Ok(None),
        Some(s) => T::deserialize(s.into_deserializer()).map(Some),
    }
}

#[derive(Deserialize, Debug)]
pub struct RPCWorkGenResp {
    pub work: String,
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
    pub contents: block::NanoBlock,
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

#[derive(Deserialize, Serialize, Debug)]
pub struct RPCProcessReq {
    action: String,
    json_block: bool,
    subtype: block::SubType,
    block: block::NanoBlock,
}

impl ClientRpc {
    pub fn new(addr: &str) -> Result<ClientRpc, Box<dyn std::error::Error>> {
        let client = Client::builder().build()?;
        Ok(ClientRpc {
            server_addr: String::from(addr),
            client: Box::new(client),
        })
    }

    pub async fn connect(
        &self,
    ) -> std::result::Result<RPCTelemetryResp, Box<dyn std::error::Error>> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([("action", "telemetry")]));
        let v = self
            .rpc_post::<RPCTelemetryResp, HashMap<&str, &str>>(r)
            .await;
        match v {
            Err(e) => {
                return Err(format!(
                    "node connection unsucessful. please try a different node.\nerror: {:?}",
                    e
                )
                .into());
            }
            Ok(v) => {
                //println!("\nconnected to network: {:?}\n", v);
                Ok(v.unwrap())
            }
        }
    }

    pub async fn block_info(&self, hash: &str) -> Option<RPCBlockInfoResp> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([
            ("action", "block_info"),
            ("json_block", "true"),
            ("hash", hash),
        ]));
        match self
            .rpc_post::<RPCBlockInfoResp, HashMap<&str, &str>>(r)
            .await
        {
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
        match self
            .rpc_post::<RPCAccountInfoResp, HashMap<&str, &str>>(r)
            .await
        {
            Err(e) => {
                eprintln!("\nrpc block info failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => {
                //println!("account info: {:?}", v);
                v
            }
        }
    }

    pub async fn process(&self, block: &block::NanoBlock) -> Option<RPCProcessResp> {
        let subtype = block.subtype.expect("block to process missing subtype");
        let r = RPCProcessReq {
            action: String::from("process"),
            json_block: true,
            subtype: subtype,
            block: block.clone(),
        };
        //println!("{:#?}", r);
        match self.rpc_post::<RPCProcessResp, RPCProcessReq>(r).await {
            Err(e) => {
                eprintln!("\nrpc process failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => {
                //println!("processed: {:?}", v);
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
        match self
            .rpc_post::<RPCPendingResp, HashMap<&str, &str>>(r)
            .await
        {
            Err(e) => {
                eprintln!("\nrpc pending failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => v,
        }
    }

    // https://docs.nano.org/commands/rpc-protocol/#pending
    pub async fn work_generate(&self, hash: &str) -> Option<RPCWorkGenResp> {
        let r = HashMap::<_, _>::from_iter(IntoIter::new([
            ("action", "work_generate"),
            ("hash", hash),
        ]));
        match self
            .rpc_post::<RPCWorkGenResp, HashMap<&str, &str>>(r)
            .await
        {
            Err(e) => {
                eprintln!("\nwork gen failed.\n error: {:#?}", e);
                None
            }
            Ok(v) => v,
        }
    }

    async fn rpc_post<T, P>(&self, r: P) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
        P: Serialize,
    {
        let resp = self.client.post(&self.server_addr).json(&r).send().await?;
        let status = resp.status();
        let resp = resp.text().await?;
        if status.is_client_error() || status.is_server_error() {
            return Err(format!("received {} from node. error: {}", status, resp).into());
        }
        println!("\nstatus: {}, body: {}\n", status, resp);
        let resp = match serde_json::from_str(&resp) {
            Ok(t) => Ok(Some(t)),
            Err(e) => {
                //eprintln!("{:?}", e);
                Err(format!("response serialization error: {:?}", e).into())
            }
        };
        resp
    }
}
