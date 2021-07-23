use crate::nano;
use crate::wallet;
use std::error::Error;

const PUBLIC_NANO_NODE_HOST: &str = "https://mynano.ninja/api/node";

pub struct Manager {
    pub rpc: nano::ClientRpc,
    wallet: wallet::Wallet,
}

impl Manager {
    pub async fn new(wallet: wallet::Wallet) -> Manager {
        let rpc =
            nano::ClientRpc::new(PUBLIC_NANO_NODE_HOST).expect("error initalizing node client");

        let mut m = Manager { rpc, wallet };
        m.synchronize().await;
        // replace with receive
        //m.queue_pending().await;
        //m.receive().await;

        m
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

    // query nano node and populate ancillary account info
    async fn synchronize(&mut self) -> Result<(), Box<dyn Error>> {
        for a in &mut self.wallet.accounts {
            if let Some(info) = self.rpc.account_info(&a.addr).await {
                if info.is_some() {
                    let info = info.unwrap();
                    a.load(info.balance.parse()?, info.frontier, info.representative);
                }
            }
        }
        Ok(())
    }

    async fn receive(&self, hash: &str, acct: &wallet::Account) -> Result<(), Box<dyn Error>> {
        if let Some(send_block_info) = self.rpc.block_info(hash).await {
            assert_eq!(send_block_info.subtype, "send");
            let sent_amount: u128 = send_block_info.amount.parse()?;
            // open block case
            if acct.frontier == "0" {
                // todo
            }
            let new_balance = acct.balance + sent_amount;
            let b = acct.create_block(new_balance, hash);
        }
        Ok(())
    }

    async fn queue_pending(&mut self) {
        for a in &mut self.wallet.accounts {
            if let Some(pending_resp) = self.rpc.pending(&a.addr).await {
                //println!("{:?}", hashes);
                a.queue_pending(pending_resp.blocks);
            }
        }
    }
}
