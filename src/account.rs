use crate::block;
use crate::encoding;
use bitvec::prelude::*;
use byteorder::{BigEndian, ByteOrder};
use ed25519_dalek_blake2b::{Keypair, PublicKey, SecretKey, Signer};
use std::convert::TryInto;
use std::error::Error;

const DEFUALT_REP: &str = "nano_1center16ci77qw5w69ww8sy4i4bfmgfhr81ydzpurm91cauj11jn6y3uc5y";

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
        pk_bits.extend_from_raw_slice(pk);
        let pk_nb32 = encoding::base32_nano_encode(&pk_bits)?;
        s.push_str("nano_");
        s.push_str(&pk_nb32);
        s.push_str(&cs_nb32);
        Ok(s)
    }

    pub fn create_block(
        &self,
        new_balance: u128,
        link: &str,
        work: Option<&str>,
    ) -> Result<block::NanoBlock, Box<dyn Error>> {
        let mut b = block::NanoBlock::new(
            &self.addr,
            &self.frontier,
            &self.rep,
            new_balance,
            link,
            work,
        )?;
        self.sign(&mut b);
        Ok(b)
    }

    fn sign(&self, block: &mut block::NanoBlock) -> Result<(), Box<dyn Error>> {
        let prev = &hex::decode(&block.previous)?[..];
        let pk_acct = decode_addr(&block.account)?;
        let pk_rep = decode_addr(&block.representative)?;
        let bal: [u8; 16] = block.balance.parse::<u128>()?.to_be_bytes();
        let link = &hex::decode(&block.link)?[..];
        let blk_data = [&block::SIG_PREAMBLE, &pk_acct, prev, &pk_rep, &bal, &link].concat();
        println!(
            "\nblk_data size:\t{}\n pre:\t{:02X?}\n acct:\t{:02X?}\n prev:\t{:02X?}\n rep:\t{:02X?}\n bal:\t{:02X?}\n link:\t{:02X?}\n",
            blk_data.len(),
            &block::SIG_PREAMBLE,
            pk_acct,
            prev,
            pk_rep,
            bal,
            link
        );
        //println!("{:02X?}", &self.kp.to_bytes()[0..SECRET_KEY_LENGTH]);
        let sig = self
            .kp
            .sign(
                [&block::SIG_PREAMBLE, &pk_acct, prev, &pk_rep, &bal, &link]
                    .concat()
                    .as_slice(),
            )
            .to_bytes();
        //sig.reverse();
        block.signature = Some(hex::encode_upper(sig));
        Ok(())
    }
}

//https://docs.nano.org/integration-guides/the-basics/#account-public-address
pub fn decode_addr(addr: &str) -> Result<[u8; 32], Box<dyn Error>> {
    let mut addr_bits = encoding::base32_nano_decode(&addr[5..57])?;
    // remove 4 bits of padding in front
    addr_bits.drain(0..4);
    let addr_bytes = addr_bits.as_raw_slice();
    let addr_bytes: [u8; 32] = addr_bytes.try_into()?;
    Ok(addr_bytes)
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
        let pk = decode_addr(addr).unwrap();
        assert_eq!(
            "30878ECBB5119B0FE4E986589ECFD2BD915D3A6CBA4843C3EE547DE649AD2BC0",
            hex::encode_upper(&pk)
        );
    }
}
