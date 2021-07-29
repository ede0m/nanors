use crate::account;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub const SIG_PREAMBLE: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6,
];

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NanoBlock {
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub kind: String,
    pub account: String,
    pub previous: String,
    pub representative: String,
    pub balance: String,
    pub link: String,
    pub signature: Option<String>,
    pub work: Option<String>,
}

impl NanoBlock {
    pub fn new(
        addr: &str,
        prev: &[u8; 32],
        rep: &str,
        new_balance: u128,
        link: &str,
        work: Option<&str>,
    ) -> Result<NanoBlock, Box<dyn Error>> {
        let work = if work.is_some() {
            Some(String::from(work.unwrap()))
        } else {
            None
        };
        Ok(NanoBlock {
            kind: String::from("state"),
            account: String::from(addr),
            previous: hex::encode_upper(prev),
            representative: String::from(rep),
            balance: new_balance.to_string(),
            link: link.to_string(),
            signature: None,
            work: work,
        })
    }
}
