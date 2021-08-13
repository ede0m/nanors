use crate::account;
use crate::block;
use crate::rpc;
use crate::wallet;
use crate::work;
use crate::ws;

use futures::lock::Mutex;
use std::convert::TryInto;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

//const PUBLIC_NANO_RPC_HOST: &str = "https://mynano.ninja/api/node";
const PUBLIC_NANO_RPC_HOST: &str = "https://proxy.nanos.cc/proxy";
const PUBLIC_NANO_WS_HOST: &str = "wss://ws.mynano.ninja/";
const WORK_LOCAL: bool = false;

pub struct Manager {
    wallet: Option<wallet::Wallet>,
    rpc: rpc::ClientRpc,
    ws: ws::ClientWS,
    ws_handle: Option<tokio::task::JoinHandle<()>>,
    cancel_ws: Option<Arc<Mutex<bool>>>,
}

impl Manager {
    pub async fn new() -> Result<Manager, Box<dyn std::error::Error>> {
        let rpc = rpc::ClientRpc::new(PUBLIC_NANO_RPC_HOST)?;
        let ws = ws::ClientWS::new(PUBLIC_NANO_WS_HOST).await?;
        Ok(Manager {
            wallet: None,
            rpc,
            ws,
            ws_handle: None,
            cancel_ws: None,
        })
    }

    pub fn has_wallet(&self) -> bool {
        self.wallet.is_some()
    }

    pub async fn set_wallet(&mut self, wallet: wallet::Wallet) -> Result<(), Box<dyn Error>> {
        //let telem = self.rpc.connect().await?;
        //println!("\nwallet connected to network: {:?}\n", telem);
        if self.ws_handle.is_some() {
            *self.cancel_ws.as_mut().unwrap().lock().await = true;
            self.ws_handle.as_mut().unwrap().await?;
        }
        self.wallet = Some(wallet);
        self.synchronize().await?;
        self.ws_observe_accounts().await?;
        Ok(())
    }

    pub fn curr_wallet_name(&self) -> Option<&str> {
        if self.wallet.is_none() {
            return None;
        }
        Some(&self.wallet.as_ref().unwrap().name)
    }

    pub async fn get_accounts_info(&self) -> Vec<account::AccountInfo> {
        if self.wallet.is_none() {
            return vec![];
        }
        let accounts = self.get_accounts().lock().await;
        accounts
            .iter()
            .map(|a| account::AccountInfo {
                index: a.index,
                addr: a.addr.clone(),
                balance: a.balance,
            })
            .collect()
    }

    pub async fn account_add(&mut self, pw: &str) -> Result<(), Box<dyn Error>> {
        if self.wallet.is_some() {
            self.wallet.as_mut().unwrap().add_account(pw).await?;
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
        let accounts = &mut self.wallet.as_mut().unwrap().accounts.lock().await;
        let from = match accounts.iter_mut().find(|a| a.addr == from) {
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

    fn get_accounts(&self) -> &Arc<Mutex<Vec<account::Account>>> {
        &self.wallet.as_ref().unwrap().accounts
    }

    async fn synchronize(&mut self) -> Result<(), Box<dyn Error>> {
        let mut accounts = self.get_accounts().lock().await;
        for a in accounts.iter_mut() {
            // query nano node and populate ancillary account info
            if let Some(info) = self.rpc.account_info(&a.addr).await {
                a.load(info.balance.parse()?, info.frontier, info.representative);
            }
            if let Some(pending) = self.rpc.pending(&a.addr).await {
                if pending.blocks.is_some() {
                    for hash in pending.blocks.unwrap() {
                        if let Some(send_block_info) = self.rpc.block_info(&hash).await {
                            let sent_amount: u128 = send_block_info.amount.parse()?;
                            Manager::receive(&self.rpc, sent_amount, &hash, a).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn ws_observe_accounts(&mut self) -> Result<(), Box<dyn Error>> {
        let accounts = self.get_accounts();
        // websocket watch confirmations in background
        let addrs = accounts
            .lock()
            .await
            .iter()
            .map(|a| a.addr.clone())
            .collect();
        let cancel = Arc::new(Mutex::new(false));
        let (tx, mut rx) = mpsc::channel::<ws::WSConfirmationMessage>(20);
        let (accounts, cx) = (accounts.clone(), cancel.clone());
        self.ws_handle = Some(tokio::spawn(async move {
            let mut ws = ws::ClientWS::new(PUBLIC_NANO_WS_HOST).await.unwrap();
            let rpc = rpc::ClientRpc::new(PUBLIC_NANO_RPC_HOST).unwrap();
            println!("here in ws handle");
            let watch_handle = tokio::spawn(async move {
                ws.subscribe_confirmation(&addrs).await.unwrap();
                if let Err(e) = ws.watch_confirmation(tx).await {
                    eprintln!("\nrecv error! {:?}", e);
                }
            });

            let recv_handle = tokio::spawn(async move {
                println!("in recv handle!");
                while let Some(msg) = rx.recv().await {
                    // TODO: not receiving message!
                    println!("{:#?}", msg);
                    let amount = msg.amount.parse::<u128>().unwrap();
                    let hash = msg.hash.as_str();
                    if let block::SubType::Send = msg.block.subtype.unwrap() {
                        let addr = msg.block.link_as_account.unwrap();
                        let accounts = &mut *accounts.lock().await;
                        let account = accounts.iter_mut().find(|a| a.addr == addr).unwrap();
                        Manager::receive(&rpc, amount, hash, account).await.unwrap();
                    }
                }
                println!("receiver was None!");
            });
            println!("after recv handle");
            loop {
                // wait for cancellation
                if *cx.lock().await {
                    println!("cancel init");
                    watch_handle.abort();
                    recv_handle.abort();
                    break;
                }
            }
        }));
        self.cancel_ws = Some(cancel);
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
