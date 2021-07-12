use std::convert::TryInto;
use rand::Rng;
use blake2::VarBlake2b;
use blake2::digest::{Update, VariableOutput};
use ed25519_dalek::PublicKey;
use ed25519_dalek::SecretKey;
use byteorder::{BigEndian, ByteOrder};
use bitvec::prelude::*;


pub struct Wallet {
    name : String,
    seed : [u8; 32],
    accounts : Vec<Account>
}

pub struct Account {
    index : u32,
    sk : [u8; 32],
    pk : [u8; 32],
    account : String
}

fn generate_seed() -> [u8; 32] {
    let random_bytes = rand::thread_rng().gen::<[u8; 32]>();
    random_bytes
}

impl Wallet {

    pub fn new(name: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
        let name = String::from(name);
        let seed = generate_seed();
        println!("{}", to_hex_string(&seed));
        let mut accounts = Vec::new();
        accounts.push(Account::new(0, &seed)?);
        Ok(Wallet {name, seed, accounts})
    }
}

impl Account {
    
    pub fn new(index: u32, seed : &[u8; 32]) -> Result<Account, Box<dyn std::error::Error>> {
        let sk = Account::create_sk(&index, seed).unwrap();
        println!("{}", to_hex_string(&sk));
        let pk = Account::create_pk(sk).unwrap();
        println!("{}", to_hex_string(&pk));
        let account = Account::create_addr(&pk).unwrap();
        println!("{}", account);
        Ok(Account {index, sk, pk, account})
    }

    //https://docs.nano.org/integration-guides/the-basics/#seed
    fn create_sk(index: &u32, seed: &[u8; 32]) -> Result<[u8; 32], Box<dyn std::error::Error>>{
        let mut hasher = VarBlake2b::new(32)?;
        let mut i_buf = [0; 4];
        BigEndian::write_u32(&mut i_buf, *index); // index as bytes
        let input : Vec<u8> = seed.iter().chain(&i_buf).cloned().collect();
        hasher.update(input); // blake2b hash seed+index
        let sk_box = hasher.finalize_boxed();
        let sk = (*sk_box).try_into()?;
        Ok(sk)
    }

    //https://docs.nano.org/integration-guides/the-basics/#account-public-key
    fn create_pk(sk: [u8; 32]) -> Result<[u8; 32], Box<dyn std::error::Error>> {         
        // the secret key of the ed25519 pair is the nano sk.
        let ed25519_sk = SecretKey::from_bytes(&sk)?;
        /* ed25519-dalek hardcoded sha512.. so i patch a local version 
        that overwrites impl From <SecretKey> for PrivateKey to use Blake2512. 
        https://docs.rs/ed25519-dalek/1.0.1/src/ed25519_dalek/public.rs.html#54-68
        https://github.com/dalek-cryptography/ed25519-dalek/pull/65/commits/d81d43e3ae957e4c707560d7aaf9f7326a96eaaa */
        let ed25519_pk : PublicKey = (&ed25519_sk).into();
        Ok(ed25519_pk.to_bytes().try_into()?)
    }
    
    fn create_addr(pk: &[u8; 32]) -> Result<String, Box<dyn std::error::Error>> {
        let mut s = String::new();
        
        // checksum of 5 bytes of pk
        let mut hasher = VarBlake2b::new(5)?;
        hasher.update(pk);
        let mut cs_box = hasher.finalize_boxed();
        (*cs_box).reverse(); // reverse the byte order as blake2b outputs in little endian
        let cs_bits = (*cs_box).view_bits::<Msb0>();
        let cs_nb32 = base32_nano_encode(&cs_bits);
        
        // 260 % 5 (base32 represented by 5 bits) = 0
        let mut pk_bits : BitVec<Msb0, u8> = BitVec::with_capacity(260);
        // 4 bits of padding in the front of the public key when encoding.
        let pad = bitvec![Msb0, u8; 0; 4]; 
        pk_bits.extend_from_bitslice(&pad);
        pk_bits.extend_from_raw_slice(pk);
        let pk_nb32 = base32_nano_encode(&pk_bits);
        
        s.push_str("nano_");
        s.push_str(&pk_nb32);
        s.push_str(&cs_nb32);
        Ok(s)
    }
}

pub fn to_hex_string(bytes: &[u8]) -> String {
    let strs: Vec<String> = bytes.iter()
        .map(|b| format!("{:02X}", b))
        .collect();
    strs.join("")
}

const B32_ENCODING_SIZE : usize = 5; 
const ALPHABET_ARR : [char; 32] = 
['1','3','4','5','6','7','8','9','a','b',
     'c','d','e','f','g','h','i','j','k','m',
     'n','o','p','q','r','s','t','u','w','x',
     'y','z'];

// credit to https://github.com/feeless/feeless/blob/main/src/keys/address.rs   
pub fn base32_nano_encode(bits: &BitSlice<Msb0, u8>) -> String {
    let mut s = String::new();
    for idx in (0..bits.len()).step_by(B32_ENCODING_SIZE) {
        let chunk = &bits[idx..idx+B32_ENCODING_SIZE];
        let value : u8 = chunk.load_be(); // big endian (msb ordering)
        s.push(ALPHABET_ARR[value as usize]);
    }
    s
}
