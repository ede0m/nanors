use crate::encoding;
use bitvec::prelude::*;
use byteorder::{BigEndian, ByteOrder};
use ed25519_dalek_blake2b::{Keypair, PublicKey, SecretKey, Signer, SECRET_KEY_LENGTH};
use hex::FromHex;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{prelude::*, BufReader};
use std::str;

const SIG_PREAMBLE: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6,
];
const DEFUALT_REP: &str = "nano_1center16ci77qw5w69ww8sy4i4bfmgfhr81ydzpurm91cauj11jn6y3uc5y";
pub const WALLET_FILE_PATH: &str = "nanors.wal";

pub struct Wallet {
    pub name: String,
    pub accounts: Vec<Account>,
}

pub struct Account {
    pub index: u32,
    pub addr: String,
    pub balance: u128,
    pub frontier: [u8; 32], // option??
    pub rep: String,
    pub pk: [u8; 32],
    sk: [u8; 32],
    kp: Keypair,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NanoBlock {
    #[serde(rename(serialize = "type", deserialize = "type"))]
    kind: String,
    account: String,
    previous: String,
    representative: String,
    balance: String,
    link: String,
    signature: String,
    work: String,
}

impl Wallet {
    pub fn new(name: &str, pw: &str) -> Result<Wallet, Box<dyn Error>> {
        let name = String::from(name);
        if find_local_wallet(&name).is_some() {
            return Err(format!("wallet {} already exists", name).into());
        }
        let seed = encoding::generate_nano_seed();
        let mut accounts = Vec::new();
        accounts.push(Account::new(0, &seed)?);
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
                accounts.push(Account::new(i, &seed)?);
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

impl Account {
    pub fn new(index: u32, seed: &[u8; 32]) -> Result<Account, Box<dyn Error>> {
        let sk = Account::create_sk(&index, seed).unwrap();
        let pk = Account::create_pk(&sk).unwrap();
        let kp = Keypair {
            secret: SecretKey::from_bytes(&sk).map_err(|e| format!("{}", e))?,
            public: PublicKey::from_bytes(&pk).map_err(|e| format!("{}", e))?,
        };
        let addr = Account::create_addr(&pk).unwrap();
        let (frontier, rep, balance) = ([0u8; 32], String::from(DEFUALT_REP), 0);
        Ok(Account {
            index,
            addr,
            balance,
            frontier,
            rep,
            sk,
            pk,
            kp,
        })
    }

    pub fn load(&mut self, balance: u128, frontier: String, rep: String) {
        self.balance = balance;
        self.frontier = match hex::decode(frontier) {
            Ok(f) => f.try_into().unwrap(),
            Err(e) => panic!("account load frontier error"),
        };
        self.rep = rep;
    }

    pub fn create_block(
        &self,
        new_balance: u128,
        link: &str,
        work: &str,
    ) -> Result<NanoBlock, Box<dyn Error>> {
        let sig = self.sign_block(new_balance, link)?;
        Ok(NanoBlock {
            kind: String::from("state"),
            account: String::from(&self.addr),
            previous: hex::encode_upper(&self.frontier),
            representative: String::from(&self.rep),
            balance: new_balance.to_string(),
            link: link.to_string(),
            signature: sig,
            work: String::from(work),
        })
    }

    fn sign_block(&self, new_balance: u128, link: &str) -> Result<String, Box<dyn Error>> {
        let prev = &self.frontier;
        let acct = &self.pk[..];
        let rep = Account::decode_addr(&self.rep)?;
        let bal: [u8; 16] = new_balance.to_be_bytes();
        let link = hex::decode(link)?;
        let blk_data = [&SIG_PREAMBLE, acct, prev, &rep, &bal, &link].concat();
        println!(
            "\nblk_data size:\t{}\n pre:\t{:02X?}\n acct:\t{:02X?}\n prev:\t{:02X?}\n rep:\t{:02X?}\n bal:\t{:02X?}\n link:\t{:02X?}\n",
            blk_data.len(),
            SIG_PREAMBLE,
            acct,
            prev,
            rep,
            bal,
            link
        );
        //println!("{:02X?}", &self.kp.to_bytes()[0..SECRET_KEY_LENGTH]);
        let sig = self
            .kp
            .sign(
                [&SIG_PREAMBLE, acct, prev, &rep, &bal, &link]
                    .concat()
                    .as_slice(),
            )
            .to_bytes();
        //sig.reverse();
        Ok(hex::encode_upper(sig))
    }

    //https://docs.nano.org/integration-guides/the-basics/#seed
    fn create_sk(index: &u32, seed: &[u8; 32]) -> Result<[u8; 32], Box<dyn Error>> {
        let mut i_buf = [0; 4];
        BigEndian::write_u32(&mut i_buf, *index); // index as bytes
        let input: Vec<u8> = seed.iter().chain(&i_buf).cloned().collect();
        let sk_box = encoding::blake2b(32, &input)?;
        let sk = (*sk_box).try_into()?;
        Ok(sk)
    }

    //https://docs.nano.org/integration-guides/the-basics/#account-public-key
    fn create_pk(sk: &[u8; 32]) -> Result<[u8; 32], Box<dyn Error>> {
        // the secret key of the ed25519 pair is the nano sk.
        let ed25519_sk = SecretKey::from_bytes(sk).map_err(|err| format!("{:?}", err))?;
        let ed25519_pk: PublicKey = (&ed25519_sk).into();
        Ok(ed25519_pk.to_bytes().try_into()?)
    }

    //https://docs.nano.org/integration-guides/the-basics/#account-public-address
    fn create_addr(pk: &[u8; 32]) -> Result<String, Box<dyn Error>> {
        let mut s = String::new();
        // checksum of 5 bytes of pk
        let mut cs_box = encoding::blake2b(5, pk)?;
        (*cs_box).reverse(); // reverse the byte order as blake2b outputs in little endian
        let cs_bits = (*cs_box).view_bits::<Msb0>();
        let cs_nb32 = encoding::base32_nano_encode(&cs_bits)?;
        // 260 % 5 (base32 represented by 5 bits) = 0
        let mut pk_bits: BitVec<Msb0, u8> = BitVec::with_capacity(260);
        // 4 bits of padding in the front of the public key when encoding.
        let pad = bitvec![Msb0, u8; 0; 4];
        pk_bits.extend_from_bitslice(&pad);
        println!("{:?}", pk_bits);
        pk_bits.extend_from_raw_slice(pk);
        let pk_nb32 = encoding::base32_nano_encode(&pk_bits)?;
        s.push_str("nano_");
        s.push_str(&pk_nb32);
        s.push_str(&cs_nb32);
        Ok(s)
    }

    //https://docs.nano.org/integration-guides/the-basics/#account-public-address
    fn decode_addr(addr: &str) -> Result<[u8; 32], Box<dyn Error>> {
        let mut addr_bits = encoding::base32_nano_decode(&addr[5..57])?;
        // remove 4 bits of padding in front
        addr_bits.drain(0..4);
        let addr_bytes = addr_bits.as_raw_slice();
        let addr_bytes: [u8; 32] = addr_bytes.try_into()?;
        Ok(addr_bytes)
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

#[cfg(test)]
mod tests {

    use super::*;

    const TEST_SEED: [u8; 32] = [
        137, 197, 104, 229, 75, 120, 185, 178, 9, 190, 248, 22, 140, 246, 140, 143, 247, 174, 97,
        154, 204, 80, 167, 39, 121, 67, 35, 190, 48, 60, 244, 11,
    ];

    #[test]
    fn valid_sk() {
        let index = 0;
        let sk = Account::create_sk(&index, &TEST_SEED).unwrap();
        assert_eq!(
            hex::encode_upper(&sk),
            "0E7EF55A55A33AE9335388ED94A9883EAF7CCC354B9025EAA52CEAA40C741B62"
        );
    }

    #[test]
    fn valid_pk() {
        let index = 0;
        let sk = Account::create_sk(&index, &TEST_SEED).unwrap();
        let pk = Account::create_pk(&sk).unwrap();
        assert_eq!(
            hex::encode_upper(&pk),
            "30878ECBB5119B0FE4E986589ECFD2BD915D3A6CBA4843C3EE547DE649AD2BC0"
        );
    }

    #[test]
    fn valid_addr() {
        let index = 0;
        let sk = Account::create_sk(&index, &TEST_SEED).unwrap();
        let pk = Account::create_pk(&sk).unwrap();
        let addr = Account::create_addr(&pk).unwrap();
        assert_eq!(
            addr,
            "nano_1e69ju7uc6eu3zkgm3krmu9x7hejdnx8sgkaah3ywo5xws6ttcy1g4yeo4bi"
        );
    }

    #[test]
    fn can_decode_addr() {
        let addr = "nano_1e69ju7uc6eu3zkgm3krmu9x7hejdnx8sgkaah3ywo5xws6ttcy1g4yeo4bi";
        let pk = Account::decode_addr(addr).unwrap();
        assert_eq!(
            "30878ECBB5119B0FE4E986589ECFD2BD915D3A6CBA4843C3EE547DE649AD2BC0",
            hex::encode_upper(&pk)
        );
    }
}
