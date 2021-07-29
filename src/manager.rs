use crate::account;
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
const RECV_DIFFICULTY: &str = "fffffe0000000000";
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
                println!("{:?}", info);
                a.load(info.balance.parse()?, info.frontier, info.representative);
            }
        }
        // todo: run receive in same loop above (immutable borrow when already borrowed mut...)
        for a in &self.wallet.accounts {
            if let Some(pending) = self.rpc.pending(&a.addr).await {
                println!("pending: {:?}", pending.blocks);
                for hash in pending.blocks {
                    let r = self.receive(&hash, a).await;
                }
            }
        }
        Ok(())
    }

    async fn receive(&self, hash: &str, acct: &account::Account) -> Result<(), Box<dyn Error>> {
        let mut subtype = "receive";
        let difficulty: [u8; 8] = hex::decode(RECV_DIFFICULTY)?.as_slice().try_into()?;
        let previous: [u8; 32];
        if acct.frontier == [0u8; 32] {
            previous = acct.pk.clone(); // open block
            subtype = "open";
        } else {
            previous = hex::decode(&acct.frontier)?
                .try_into()
                .expect("frontier malformed in pow");
        }
        let work = hex::encode(Manager::pow_local(previous, &difficulty)?);
        if let Some(send_block_info) = self.rpc.block_info(hash).await {
            let sent_amount: u128 = send_block_info.amount.parse()?;
            let new_balance = acct.balance + sent_amount;
            let b = acct.create_block(new_balance, hash, Some(&work))?;
            if let Some(hash) = self.rpc.process(&b, subtype).await {
                println!("{}", hash.hash);
            }
        }
        Ok(())
    }

    //https://docs.nano.org/integration-guides/work-generation/#work-calculation-details
    fn pow_local(previous: [u8; 32], threshold: &[u8; 8]) -> Result<[u8; 8], Box<dyn Error>> {
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
