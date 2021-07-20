use crate::encoding;
use bitvec::prelude::*;
use byteorder::{BigEndian, ByteOrder};
use ed25519_dalek::PublicKey;
use ed25519_dalek::SecretKey;
use hex::FromHex;
use std::convert::TryInto;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{prelude::*, BufReader};
use std::str;

pub struct Wallet {
    pub name: String,
    pub accounts: Vec<Account>,
    seed: [u8; 32],
}

pub struct Account {
    index: u32,
    sk: [u8; 32],
    pk: [u8; 32],
    account: String,
}

impl Wallet {
    pub fn new(name: &str, pw: &str) -> Result<Wallet, Box<dyn Error>> {
        let name = String::from(name);
        let seed = encoding::generate_nano_seed();
        let mut accounts = Vec::new();
        accounts.push(Account::new(0, &seed)?);

        let wallet = Wallet {
            name,
            seed,
            accounts,
        };
        wallet.save_wallet(pw)?;
        Ok(wallet)
    }

    pub fn load(w_name: &str, pw: &str) -> Result<Wallet, Box<dyn Error>> {
        let file = OpenOptions::new().read(true).open("nanors.wal")?;
        let reader = BufReader::new(file);
        let mut seed = [0u8; 32];
        let (mut name, mut n_acct, mut ciphertext, mut nonce) =
            (String::new(), 0, vec![], [0u8; 12]);
        for line in reader.lines() {
            let line = line?;
            let mut wal = line.split("|");
            name = String::from(wal.next().expect("name not found"));
            if name == w_name {
                n_acct = wal
                    .next()
                    .ok_or("n_acct not found")?
                    .parse::<u32>()
                    .unwrap();
                ciphertext = hex::decode(wal.next().ok_or("ciphertext not found")?)?;
                nonce = <[u8; 12]>::from_hex(wal.next().ok_or("nonce not found")?)?;
                seed =
                    encoding::aes_gcm_decrypt(pw.as_bytes(), nonce, &ciphertext, name.as_bytes())
                        .as_slice()
                        .try_into()?;
            }
        }
        if !name.is_empty() && n_acct > 0 {
            let mut accounts = Vec::new();
            for i in 0..n_acct {
                accounts.push(Account::new(i, &seed)?);
            }
            Ok(Wallet {
                name,
                seed,
                accounts,
            })
        } else {
            Err(format!("wallet {} not found", w_name).into())
        }
    }

    fn save_wallet(&self, pw: &str) -> Result<(), Box<dyn Error>> {
        let (ciphertext, nonce) =
            encoding::aes_gcm_encrypt(pw.as_bytes(), &self.seed, &self.name.as_bytes());
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("nanors.wal")?;
        writeln!(
            file,
            "{}|{}|{}|{}",
            self.name,
            self.accounts.len(),
            hex::encode_upper(&ciphertext),
            hex::encode_upper(&nonce)
        );
        Ok(())
    }
}

impl Account {
    pub fn new(index: u32, seed: &[u8; 32]) -> Result<Account, Box<dyn Error>> {
        let sk = Account::create_sk(&index, seed).unwrap();
        let pk = Account::create_pk(&sk).unwrap();
        let account = Account::create_addr(&pk).unwrap();
        Ok(Account {
            index,
            sk,
            pk,
            account,
        })
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
        let ed25519_sk = SecretKey::from_bytes(sk)?;
        /* ed25519-dalek hardcoded sha512.. so i patch a local version
        that overwrites impl From <SecretKey> for PrivateKey to use Blake2512.
        https://docs.rs/ed25519-dalek/1.0.1/src/ed25519_dalek/public.rs.html#54-68
        https://github.com/dalek-cryptography/ed25519-dalek/pull/65/commits/d81d43e3ae957e4c707560d7aaf9f7326a96eaaa */
        let ed25519_pk: PublicKey = (&ed25519_sk).into();
        Ok(ed25519_pk.to_bytes().try_into()?)
    }

    fn create_addr(pk: &[u8; 32]) -> Result<String, Box<dyn Error>> {
        let mut s = String::new();
        // checksum of 5 bytes of pk
        let mut cs_box = encoding::blake2b(5, pk)?;
        (*cs_box).reverse(); // reverse the byte order as blake2b outputs in little endian
        let cs_bits = (*cs_box).view_bits::<Msb0>();
        let cs_nb32 = encoding::base32_nano_encode(&cs_bits);
        // 260 % 5 (base32 represented by 5 bits) = 0
        let mut pk_bits: BitVec<Msb0, u8> = BitVec::with_capacity(260);
        // 4 bits of padding in the front of the public key when encoding.
        let pad = bitvec![Msb0, u8; 0; 4];
        pk_bits.extend_from_bitslice(&pad);
        pk_bits.extend_from_raw_slice(pk);
        let pk_nb32 = encoding::base32_nano_encode(&pk_bits);
        s.push_str("nano_");
        s.push_str(&pk_nb32);
        s.push_str(&cs_nb32);
        Ok(s)
    }
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
}
