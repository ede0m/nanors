use crate::account;
use crate::encoding;
use hex::FromHex;
use std::convert::TryInto;
use std::error::Error;
use std::fs::OpenOptions;
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

    pub fn load(w_name: &str, pw: &str) -> Result<Wallet, Box<dyn Error>> {
        let seed;
        let (name, n_acct);
        match find_local_wallet(w_name) {
            Some(wstr) => {
                let mut wal = wstr.split("|");
                name = String::from(wal.next().ok_or("name not found")?);
                let wallet_data = wallet_data_from_str(wal)?;
                n_acct = wallet_data.0;
                let ciphertext = wallet_data.1;
                let nonce = wallet_data.2;
                seed =
                    encoding::aes_gcm_decrypt(pw.as_bytes(), nonce, &ciphertext, name.as_bytes())?
                        .as_slice()
                        .try_into()?;
            }
            None => return Err(format!("wallet {} not found", w_name).into()),
        }
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
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(WALLET_FILE_PATH)?;
        writeln!(
            file,
            "{}|{}|{}|{}",
            self.name,
            self.accounts.len(),
            hex::encode_upper(&ciphertext),
            hex::encode_upper(&nonce)
        )?;
        Ok(())
    }
}

fn find_local_wallet(find_name: &str) -> Option<String> {
    let file = OpenOptions::new().read(true).open(WALLET_FILE_PATH).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        let name = String::from(line.split("|").next()?);
        if name == find_name {
            return Some(line);
        }
    }
    None
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
