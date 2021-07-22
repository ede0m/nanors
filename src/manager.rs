use crate::nano;
use crate::wallet;

const PUBLIC_NANO_NODE_HOST: &str = "https://mynano.ninja/api/node";

pub struct Manager {
    pub rpc: nano::ClientRpc,
    wallet: wallet::Wallet,
}

impl Manager {
    pub fn new(wallet: wallet::Wallet) -> Manager {
        let rpc =
            nano::ClientRpc::new(PUBLIC_NANO_NODE_HOST).expect("error initalizing node client");
        Manager { rpc, wallet }
    }

    pub fn curr_wallet_name(&self) -> &str {
        &self.wallet.name
    }

    pub fn accounts_show(&self) -> Vec<String> {
        self.wallet
            .accounts
            .iter()
            .map(|a| format!("  {} : {}", a.index, a.addr))
            .collect()
    }

    async fn queue_pending(&mut self) {
        for a in &mut self.wallet.accounts {
            if let Some(hashes) = self.rpc.pending(&a.addr).await {
                a.queue_pending(hashes);
            }
        }
    }
}
