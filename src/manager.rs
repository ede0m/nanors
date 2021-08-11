use crate::account;
use crate::block;
use crate::rpc;
use crate::wallet;
use crate::work;
use crate::ws;

use futures_util::StreamExt;
use std::convert::TryInto;
use std::error::Error;
use tokio::sync::mpsc;

const WORK_LOCAL: bool = false;

pub struct Manager {
    rpc: Box<rpc::ClientRpc>,
    ws: Box<ws::ClientWS>,
    wallet: Option<wallet::Wallet>,
}

impl Manager {
    pub fn new(
        rpc: Box<rpc::ClientRpc>,
        ws: Box<ws::ClientWS>,
    ) -> Result<Manager, Box<dyn std::error::Error>> {
        Ok(Manager {
            rpc,
            ws,
            wallet: None,
        })
    }

    pub fn has_wallet(&self) -> bool {
        self.wallet.is_some()
    }

    pub async fn set_wallet(&'static mut self, wallet: wallet::Wallet) -> Result<(), Box<dyn Error>> {
        self.wallet = Some(wallet);
        let telem = self.rpc.connect().await?;
        println!("\nconnected to network: {:?}\n", telem);
        self.synchronize().await?;
        Ok(())
    }

    pub fn curr_wallet_name(&self) -> Option<&str> {
        if self.wallet.is_none() {
            return None;
        }
        Some(&self.wallet.as_ref().unwrap().name)
    }

    pub fn get_accounts(&self) -> Vec<account::AccountInfo> {
        if self.wallet.is_none() {
            return vec![];
        }
        self.wallet
            .as_ref()
            .unwrap()
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
        if self.wallet.is_some() {
            self.wallet.as_mut().unwrap().add_account(pw)?;
        } else {
            return Err("no wallet set".into());
        }
        Ok(())
    }

    pub async fn send(
        &mut self,
        amount: u128,
        from: &str,
        to: &str,
    ) -> Result<String, Box<dyn Error>> {
        if self.wallet.is_none() {
            return Err("no wallet set".into());
        }
        let from = match self
            .wallet
            .as_mut()
            .unwrap()
            .accounts
            .iter_mut()
            .find(|a| a.addr == from)
        {
            Some(a) => a,
            None => return Err("from address not found".into()),
        };
        if !from.has_work() {
            Manager::cache_work(
                from,
                &self.rpc,
                from.frontier.clone(),
                work::DEFAULT_DIFFICULTY,
            )
            .await?;
        }
        let block = from.send(amount, to)?;
        if let Some(hash) = self.rpc.process(&block).await {
            // todo: just do this in acct.create_block.
            // do a rollback somehow..?
            from.accept_block(&block)?;
            Manager::cache_work(
                from,
                &self.rpc,
                from.frontier.clone(),
                work::DEFAULT_DIFFICULTY,
            )
            .await?;
            return Ok(hash.hash);
        }
        Err("could not process send block".into())
    }

    async fn synchronize(&'static mut self) -> Result<(), Box<dyn Error>> {
        let mut acct_addrs = vec![];
        for a in &mut self.wallet.as_mut().unwrap().accounts {
            acct_addrs.push(a.addr.clone());
            // query nano node and populate ancillary account info
            if let Some(info) = self.rpc.account_info(&a.addr).await {
                a.load(info.balance.parse()?, info.frontier, info.representative);
            }
            if let Some(pending) = self.rpc.pending(&a.addr).await {
                for hash in pending.blocks {
                    if let Some(send_block_info) = self.rpc.block_info(&hash).await {
                        let sent_amount: u128 = send_block_info.amount.parse()?;
                        Manager::receive(&self.rpc, sent_amount, &hash, a).await?;
                    }
                }
            }
        }
        // websocket recv confirmations in background   
        self.ws.subscribe_confirmation(acct_addrs).await?;
        let handle = tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel::<tokio_tungstenite::tungstenite::Message>(20);
            tokio::spawn(async move {
                if let Err(e) = self.ws.watch_confirmation(tx).await {
                    panic!(format!("{:?}",e));
                }
            });
            while let Some(msg) = rx.recv().await {
                //Manager::receive(&self.rpc, sent_amount, &hash, a).await.unwrap();
                println!("{}", msg);
            }
        });
        //handle.await.unwrap();
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
            if !account.has_work() {
                Manager::cache_work(account, rpc, account.pk.clone(), work::RECV_DIFFICULTY)
                    .await?;
            }
            block = account.open(amount, link)?;
        } else {
            if !account.has_work() {
                Manager::cache_work(
                    account,
                    rpc,
                    account.frontier.clone(),
                    work::RECV_DIFFICULTY,
                )
                .await?;
            }
            block = account.receive(amount, link)?;
        }
        if let Some(hash) = rpc.process(&block).await {
            // todo: just do this in acct.create_block.
            // do a rollback somehow..?
            account.accept_block(&block)?;
            Manager::cache_work(
                account,
                rpc,
                account.frontier.clone(),
                work::DEFAULT_DIFFICULTY,
            )
            .await?;
            return Ok(hash.hash);
        }
        Err("could not process receive block".into())
    }

    async fn cache_work(
        account: &mut account::Account,
        rpc: &rpc::ClientRpc,
        previous: [u8; 32],
        difficulty: &str,
    ) -> Result<(), Box<dyn Error>> {
        let work = Manager::gen_work(rpc, previous, difficulty).await?;
        account.cache_work(work);
        Ok(())
    }

    async fn gen_work(
        rpc: &rpc::ClientRpc,
        previous: [u8; 32],
        difficulty: &str,
    ) -> Result<String, Box<dyn Error>> {
        // https://docs.nano.org/integration-guides/work-generation/#work-calculation-details
        let difficulty = hex::decode(difficulty)?.as_slice().try_into()?;
        let work;
        if WORK_LOCAL {
            work = hex::encode(work::pow_local(previous, &difficulty)?)
        } else {
            let prev = hex::encode(previous);
            work = match rpc.work_generate(&prev).await {
                Some(w) => w.work,
                None => return Err("failed to generate work".into()),
            };
        }
        Ok(work)
    }
}
