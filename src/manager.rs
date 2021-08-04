use crate::account;
use crate::block;
use crate::encoding;
use crate::rpc;
use crate::wallet;

use std::convert::TryInto;
use std::error::Error;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::time::SystemTime;

const PUBLIC_NANO_NODE_HOST: &str = "https://mynano.ninja/api/node";
const POW_LOCAL_WORKERS: u64 = 8;

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

    //https://docs.nano.org/integration-guides/work-generation/#work-calculation-details
    pub fn pow_local(previous: [u8; 32], threshold: &[u8; 8]) -> Result<[u8; 8], Box<dyn Error>> {
        let threshold = threshold.clone();
        let (tx, rx): (Sender<[u8; 8]>, Receiver<[u8; 8]>) = mpsc::channel();
        let found = Arc::new(Mutex::new(false));
        let now = SystemTime::now();
        let mut handles = vec![];
        // dispatch workers
        for i in 0..POW_LOCAL_WORKERS {
            let (sender, arc) = (tx.clone(), found.clone());
            let handle = std::thread::spawn(move || {
                Manager::pow_local_segment(i, &previous, &threshold, sender, arc);
            });
            handles.push(handle);
        }
        let mut work = rx.recv().unwrap(); // recv will block.
        work.reverse(); // work hex string seems to be LE
        *found.lock().unwrap() = true;
        let elapsed_min = (now.elapsed()?.as_secs()) as f64 / 60.0;
        println!(
            "pow complete in {} minutes. work: {:02x?} -> {:02x?}",
            elapsed_min,
            work,
            encoding::nano_work_hash(&previous, &work)
        );
        for handle in handles {
            handle.join().unwrap();
        }
        Ok(work)
    }

    fn pow_local_segment(
        i: u64,
        previous: &[u8; 32],
        threshold: &[u8; 8],
        sender: Sender<[u8; 8]>,
        found: Arc<Mutex<bool>>,
    ) {
        let seg_size = 0xffffffffffffffff / POW_LOCAL_WORKERS;
        let (low, high) = (seg_size * i, seg_size * (i + 1));
        for nonce in low..high {
            let nonce = nonce.to_be_bytes();
            if let Ok(output) = encoding::nano_work_hash(previous, &nonce) {
                // blake2b output in le
                if u64::from_le_bytes(output) >= u64::from_be_bytes(*threshold) {
                    sender.send(nonce).unwrap();
                    break;
                }
            }
            if *found.lock().unwrap() {
                break;
            }
        }
    }
}
