use crate::nano;
use crate::wallet;
use std::convert::TryInto;
use std::error::Error;

const PUBLIC_NANO_NODE_HOST: &str = "https://mynano.ninja/api/node";
const RECV_DIFFICULTY: &str = "fffffe0000000000";

pub struct Manager {
    pub rpc: nano::ClientRpc,
    wallet: wallet::Wallet,
    active_network_difficulty: [u8; 8],
}

impl Manager {
    pub async fn new(wallet: wallet::Wallet) -> Result<Manager, Box<dyn std::error::Error>> {
        let rpc = nano::ClientRpc::new(PUBLIC_NANO_NODE_HOST)?;
        let telem = rpc
            .connect()
            .await
            .ok_or("could not connect to nano node")?;
        let active_network_difficulty = hex::decode(telem.active_difficulty)?
            .as_slice()
            .try_into()?;
        let mut m = Manager {
            rpc,
            wallet,
            active_network_difficulty,
        };
        m.synchronize().await;
        Ok(m)
    }

    pub fn curr_wallet_name(&self) -> &str {
        &self.wallet.name
    }

    pub fn accounts_show(&self) -> Vec<String> {
        self.wallet
            .accounts
            .iter()
            .map(|a| format!("  {} : {} : {}", a.index, a.addr, a.balance))
            .collect()
    }

    async fn synchronize(&mut self) -> Result<(), Box<dyn Error>> {
        for a in &mut self.wallet.accounts {
            // query nano node and populate ancillary account info
            if let Some(info) = self.rpc.account_info(&a.addr).await {
                a.load(info.balance.parse()?, info.frontier, info.representative);
            }
        }
        for a in &self.wallet.accounts {
            if let Some(pending) = self.rpc.pending(&a.addr).await {
                println!("{:?}", pending.blocks);
                for hash in pending.blocks {
                    let r = self.receive(&hash, a).await;
                }
            }
        }
        Ok(())
    }

    async fn receive(&self, hash: &str, acct: &wallet::Account) -> Result<(), Box<dyn Error>> {
        let difficulty: [u8; 8] = hex::decode(RECV_DIFFICULTY)?.as_slice().try_into()?;
        if let Some(send_block_info) = self.rpc.block_info(hash).await {
            assert_eq!(send_block_info.subtype, "send");
            let sent_amount: u128 = send_block_info.amount.parse()?;
            let new_balance = acct.balance + sent_amount;
            let b = acct.create_block(new_balance, hash, &difficulty);
        }
        Ok(())
    }

    /*async fn receive_all(&self) {
        for a in &self.wallet.accounts {
            if let Some(pending_resp) = self.rpc.pending(&a.addr).await {
                println!("{:?}", pending_resp.blocks);
                for hash in pending_resp.blocks {
                    let r = self.receive(&hash, a).await;
                }
            }
        }
    }*/
}
