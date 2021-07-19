use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes128Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use bitvec::prelude::*;
use blake2::digest::{Update, VariableOutput};
use blake2::VarBlake2b;
use rand::Rng;


const B32_ENCODING_SIZE: usize = 5;
const ALPHABET_ARR: [char; 32] = [
    '1', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'w', 'x', 'y', 'z',
];

pub fn to_hex_string(bytes: &[u8]) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    strs.join("")
}

// credit to https://github.com/feeless/feeless/blob/main/src/keys/address.rs
pub fn base32_nano_encode(bits: &BitSlice<Msb0, u8>) -> String {
    let mut s = String::new();
    for idx in (0..bits.len()).step_by(B32_ENCODING_SIZE) {
        let chunk = &bits[idx..idx + B32_ENCODING_SIZE];
        let value: u8 = chunk.load_be(); // big endian (msb ordering)
        s.push(ALPHABET_ARR[value as usize]);
    }
    s
}

pub fn generate_nano_seed() -> [u8; 32] {
    let random_bytes = rand::thread_rng().gen::<[u8; 32]>();
    random_bytes
}

pub fn aes_gcm_encrypt(pw: &[u8], data: &[u8], hkdf_info : &[u8]) -> (Vec<u8>, [u8; 12]) {
    let key = hkdf_pw_expand(pw, hkdf_info);
    let key = aes_gcm::Key::from_slice(&key);
    let cipher = Aes128Gcm::new(key);
    let nonce_data = rand::thread_rng().gen::<[u8; 12]>(); // 96 bit. TODO: use a sequence..
    let nonce = Nonce::from_slice(&nonce_data);
    (
        cipher.encrypt(nonce, data).expect("encrypt failure"),
        nonce_data,
    )
}

pub fn aes_gcm_decrypt(pw: &[u8], nonce: [u8; 12], ciphertext: &[u8], hkdf_info : &[u8]) -> Vec<u8> {
    let key = hkdf_pw_expand(pw, hkdf_info);
    let key = aes_gcm::Key::from_slice(&key);
    let cipher = Aes128Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce);
    cipher.decrypt(nonce, ciphertext).expect("decrypt failure")
}

pub fn hkdf_pw_expand(ikm: &[u8], info: &[u8]) -> [u8; 16] { 
    let mut okm = [0u8; 16]; // 128bit AES
    let h = Hkdf::<Sha256>::new(None, ikm);
    h.expand(info, &mut okm).expect("hdkf expand - something went wrong");
    okm
}


/*pub fn pbkdf2_key(key_buffer : [u8; 32], pw : &[u8]) -> [u8; 32] {
    let iters = NonZeroU32::new(100).unwrap();
    derive(PBKDF2_HMAC_SHA256, iters, &SALT, pw, &mut key_buffer);
    key_buffer
}

pub fn sealing_key(key : &[u8; 32]) -> Result<SealingKey, Box<dyn std::error::Error>> {
    let key = UnboundKey::new(&AES_128_GCM, key)?;
    let nonce_data = rand::thread_rng().gen::<[u8; 12]>(); // 96 bit nonces;
    let nonce = Nonce::try_assume_unique_for_key(&nonce_data)?;
    SealingKey::new(key, nonce)
    // would need to implement nonce advance for NOnceSequence trait.. moving to RustCrypto.
    Ok()
}
*/

pub fn blake2b(
    digest_size: usize,
    message: &[u8],
) -> Result<Box<[u8]>, Box<dyn std::error::Error>> {
    let mut hasher = VarBlake2b::new(digest_size)?;
    hasher.update(message);
    Ok(hasher.finalize_boxed())
}
