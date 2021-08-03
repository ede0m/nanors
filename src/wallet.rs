use crate::account;
use crate::encoding;
use hex::FromHex;
use std::convert::TryInto;
use std::error::Error;
use std::fs::OpenOptions;
use std::fs;
use std::io::{prelude::*, BufReader};
use std::str;

pub const WALLET_FILE_PATH: &str = "nanors.wal";

pub struct Wallet {
    pub name: String,
    pub accounts: Vec<account::Account>,
}

impl Wallet {
    pub fn new(name: &str, pw: &str) -> Result<Wallet, Box<dyn Error>> {
        let name = String::from(name);
        if find_local_wallet(&name).is_some() {
            return Err(format!("wallet {} already exists", name).into());
        }
        let seed = encoding::generate_nano_seed();
        let mut accounts = Vec::new();
        accounts.push(account::Account::new(0, &seed)?);
        let wallet = Wallet { name, accounts };
        wallet.save_wallet(pw, &seed)?;
        Ok(wallet)
    }

    pub fn add_account(&mut self, pw: &str) -> Result<(), Box<dyn Error>> {
        let (_, n_acct, seed) = get_wallet_data(&self.name, pw)?;
        self.accounts
            .push(account::Account::new(n_acct, &seed)?);
        self.save_wallet(pw, &seed)?;
        Ok(())
    }

    pub fn load(w_name: &str, pw: &str) -> Result<Wallet, Box<dyn Error>> {
        let (name, n_acct, seed) = get_wallet_data(w_name, pw)?;
        if !name.is_empty() && n_acct > 0 {
            let mut accounts = Vec::new();
            for i in 0..n_acct {
                accounts.push(account::Account::new(i, &seed)?);
            }
            Ok(Wallet { name, accounts })
        } else {
            Err("something went wrong".into())
        }
    }

    fn save_wallet(&self, pw: &str, seed: &[u8]) -> Result<(), Box<dyn Error>> {
        let (ciphertext, nonce) =
            encoding::aes_gcm_encrypt(pw.as_bytes(), seed, &self.name.as_bytes());
        let new_wal_str = format!(            
            "{}|{}|{}|{}",
            self.name,
            self.accounts.len(),
            hex::encode_upper(&ciphertext),
            hex::encode_upper(&nonce)
        );
        let mut lines : Vec<String> = fs::read_to_string(WALLET_FILE_PATH)?.lines().map(|l| l.to_string()).collect();
        if let Some((_, line_index)) = find_local_wallet(&self.name) {
            // remove old wallet if we are overwriting
            lines.remove(line_index);
        }
        lines.push(new_wal_str);
        let lines = lines.join("\n");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(WALLET_FILE_PATH)?;     
        writeln!(
            file,
            "{}",
            lines
        )?;
        Ok(())
    }
}

fn find_local_wallet(find_name: &str) -> Option<(String, usize)> {
    let file = OpenOptions::new().read(true).open(WALLET_FILE_PATH).ok()?;
    let reader = BufReader::new(file);
    for (i, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        let name = String::from(line.split("|").next()?);
        if name == find_name {
            return Some((line, i));
        }
    }
    None
}

fn get_wallet_data(w_name: &str, pw: &str) -> Result<(String, u32, [u8; 32]), Box<dyn Error>> {
    let (name, n_acct, seed);
    match find_local_wallet(w_name) {
        Some((wstr, _)) => {
            let mut wal = wstr.split("|");
            name = String::from(wal.next().ok_or("name not found")?);
            let wallet_data = wallet_data_from_str(wal)?;
            n_acct = wallet_data.0;
            let ciphertext = wallet_data.1;
            let nonce = wallet_data.2;
            seed = encoding::aes_gcm_decrypt(pw.as_bytes(), nonce, &ciphertext, name.as_bytes())?
                .as_slice()
                .try_into()?;
        }
        None => return Err(format!("wallet {} not found", w_name).into()),
    }
    Ok((name, n_acct, seed))
}

fn wallet_data_from_str<'a, I>(mut wal_iter: I) -> Result<(u32, Vec<u8>, [u8; 12]), Box<dyn Error>>
where
    I: Iterator<Item = &'a str>,
{
    let n_acct = wal_iter
        .next()
        .ok_or("n_acct not found")?
        .parse::<u32>()
        .unwrap();
    let ciphertext = hex::decode(wal_iter.next().ok_or("ciphertext not found")?)?;
    let nonce = <[u8; 12]>::from_hex(wal_iter.next().ok_or("nonce not found")?)?;
    Ok((n_acct, ciphertext, nonce))
}
