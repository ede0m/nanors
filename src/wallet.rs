use std::convert::TryInto;
use rand::Rng;
use blake2::VarBlake2b;
use blake2::digest::{Update, VariableOutput};
use ed25519_dalek::PublicKey;
use ed25519_dalek::SecretKey;
use byteorder::{BigEndian, ByteOrder};

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
        let account = String::from("");
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
    
    fn create_account(pubk: [u8; 32]) -> String {
        return String::from("not implemented");
    }
}

pub fn to_hex_string(bytes: &[u8]) -> String {
    let strs: Vec<String> = bytes.iter()
        .map(|b| format!("{:02X}", b))
        .collect();
    strs.join("")
}
