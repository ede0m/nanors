use crate::account;
use crate::block;
use crate::rpc;
use crate::wallet;

use std::convert::TryInto;
use std::error::Error;

const PUBLIC_NANO_NODE_HOST: &str = "https://mynano.ninja/api/node";

pub struct Manager {
    pub rpc: rpc::ClientRpc,
    wallet: wallet::Wallet,
    active_network_difficulty: [u8; 8],
}

impl Manager {
    pub async fn new(wallet: wallet::Wallet) -> Result<Manager, Box<dyn std::error::Error>> {
        let rpc = rpc::ClientRpc::new(PUBLIC_NANO_NODE_HOST)?;
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
        m.synchronize().await?;
        Ok(m)
    }

    pub async fn send(
        &mut self,
        amount: u128,
        from: &str,
        to: &str,
    ) -> Result<String, Box<dyn Error>> {
        let from = match self.wallet.accounts.iter_mut().find(|a| a.addr == from) {
            Some(a) => a,
            None => return Err("from address not found".into()),
        };
        let block = from.send(amount, to)?;
        if let Some(hash) = self.rpc.process(&block).await {
            // todo: just do this in acct.create_block.
            // do a rollback somehow..?
            from.accept_block(&block)?;
            return Ok(hash.hash);
        }
        Err("could not process send block".into())
    }

    pub fn curr_wallet_name(&self) -> &str {
        &self.wallet.name
    }

    pub fn get_accounts(&self) -> Vec<account::AccountInfo> {
        self.wallet
            .accounts
            .iter()
            .map(|a| account::AccountInfo {
                index: a.index,
                addr: a.addr.clone(),
                balance: a.balance,
            })
            .collect()
    }

    pub fn account_add(&mut self, pw: &str) -> Result<(), Box<dyn Error>> {
        self.wallet.add_account(pw)?;
        Ok(())
    }

    async fn synchronize(&mut self) -> Result<(), Box<dyn Error>> {
        for a in &mut self.wallet.accounts {
            // query nano node and populate ancillary account info
            if let Some(info) = self.rpc.account_info(&a.addr).await {
                a.load(info.balance.parse()?, info.frontier, info.representative);
            }
            if let Some(pending) = self.rpc.pending(&a.addr).await {
                for hash in pending.blocks {
                    if let Some(send_block_info) = self.rpc.block_info(&hash).await {
                        let sent_amount: u128 = send_block_info.amount.parse()?;
                        let processed_hash =
                            Manager::receive(&self.rpc, sent_amount, &hash, a).await?;
                        //println!("receive processed: {:?}", processed_hash);
                    }
                }
            }
        }

        Ok(())
    }

    async fn receive(
        rpc: &rpc::ClientRpc,
        amount: u128,
        link: &str,
        account: &mut account::Account,
    ) -> Result<String, Box<dyn Error>> {
        let block: block::NanoBlock;
        if account.frontier == [0u8; block::BLOCK_HASH_SIZE] {
            block = account.open(amount, link)?; // open case
        } else {
            block = account.receive(amount, link)?;
        }
        if let Some(hash) = rpc.process(&block).await {
            // todo: just do this in acct.create_block.
            // do a rollback somehow..?
            account.accept_block(&block)?;
            return Ok(hash.hash);
        }
        Err("could not process receive block".into())
    }

}
