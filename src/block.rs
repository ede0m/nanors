use crate::account;
use crate::encoding;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::error::Error;

pub const SIG_PREAMBLE: u8 = 0x6;
pub const BLOCK_HASH_SIZE: usize = 32;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NanoBlock {
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub kind: String,
    pub account: String,
    pub previous: String,
    pub representative: String,
    pub balance: String,
    pub link: String,
    pub link_as_account: Option<String>,
    pub signature: Option<String>,
    pub hash: Option<String>,
    pub subtype: Option<SubType>,
    pub work: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SubType {
    Send,
    Open,
    Receive,
    Change,
}
// todo: need string rep for serializing to process req.

impl NanoBlock {
    pub fn new(
        addr: &str,
        prev: &[u8; 32],
        rep: &str,
        new_balance: u128,
        link: &str,
        subtype: SubType,
        work: String,
    ) -> Result<NanoBlock, Box<dyn Error>> {
        let mut b = NanoBlock {
            kind: String::from("state"),
            account: String::from(addr),
            previous: hex::encode_upper(prev),
            representative: String::from(rep),
            balance: new_balance.to_string(),
            link: link.to_string(),
            link_as_account: None,
            signature: None,
            hash: None,
            subtype: Some(subtype),
            work: work,
        };
        b.set_hash()?;
        Ok(b)
    }

    fn set_hash(&mut self) -> Result<(), Box<dyn Error>> {
        let mut preamble = [0u8; 32];
        preamble[31] = SIG_PREAMBLE;
        let prev = &hex::decode(&self.previous)?[..];
        let pk_acct = account::decode_addr(&self.account)?;
        let pk_rep = account::decode_addr(&self.representative)?;
        let bal: [u8; 16] = self.balance.parse::<u128>()?.to_be_bytes();
        let link = match self.subtype {
            Some(SubType::Send) => account::decode_addr(&self.link)?,
            Some(SubType::Receive) | Some(SubType::Open) => {
                hex::decode(&self.link)?[..].try_into()?
            }
            Some(SubType::Change) => [0u8; 32],
            None => panic!("todo"),
        };

        let blk_data = [&preamble, &pk_acct, prev, &pk_rep, &bal, &link].concat();
        /*println!(
            "\nblk_data size:\t{}\n pre:\t{:02X?}\n acct:\t{:02X?}\n prev:\t{:02X?}\n rep:\t{:02X?}\n bal:\t{:02X?}\n link:\t{:02X?}\n",
            blk_data.len(),
            SIG_PREAMBLE,
            pk_acct,
            prev,
            pk_rep,
            bal,
            link
        );*/
        let hash: [u8; BLOCK_HASH_SIZE] =
            (*encoding::blake2bv(BLOCK_HASH_SIZE, &blk_data)?).try_into()?;
        self.hash = Some(hex::encode_upper(hash));
        Ok(())
    }
}
