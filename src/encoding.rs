use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes128Gcm, Nonce};
use bitvec::prelude::*;
use blake2::digest::{Update, VariableOutput};
use blake2::VarBlake2b;
use hkdf::Hkdf;
use rand::Rng;
use sha2::Sha256;
use std::convert::TryInto;

const B32_ENCODING_SIZE: usize = 5;
const ALPHABET_ARR: [char; 32] = [
    '1', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'w', 'x', 'y', 'z',
];

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
    rand::thread_rng().gen::<[u8; 32]>()
}

pub fn nano_work_hash(prev: &[u8], nonce: &[u8; 8]) -> Result<[u8; 8], Box<dyn std::error::Error>> {
    let to_hash = [nonce, prev].concat();
    // threshold is 8 bytes
    let th_box = blake2b(8, &to_hash)?;
    Ok((*th_box).try_into()?)
}

pub fn aes_gcm_encrypt(pw: &[u8], data: &[u8], hkdf_info: &[u8]) -> (Vec<u8>, [u8; 12]) {
    let key = hkdf_pw_expand(pw, hkdf_info);
    let key = aes_gcm::Key::from_slice(&key);
    let cipher = Aes128Gcm::new(key);
    let nonce_data = rand::thread_rng().gen::<[u8; 12]>(); // 96 bit. todo: use sequence
                                                           //let nonce_data = nonce::<12>();
    let nonce = Nonce::from_slice(&nonce_data);
    (
        cipher.encrypt(nonce, data).expect("encrypt failure"),
        nonce_data,
    )
}

pub fn aes_gcm_decrypt(
    pw: &[u8],
    nonce: [u8; 12],
    ciphertext: &[u8],
    hkdf_info: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let key = hkdf_pw_expand(pw, hkdf_info);
    let key = aes_gcm::Key::from_slice(&key);
    let cipher = Aes128Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce);
    match cipher.decrypt(nonce, ciphertext) {
        Ok(pt) => Ok(pt),
        Err(e) => Err("could not decrypt wallet key".into()),
    }
}

fn hkdf_pw_expand(ikm: &[u8], info: &[u8]) -> [u8; 16] {
    let mut okm = [0u8; 16]; // 128bit AES
    let h = Hkdf::<Sha256>::new(None, ikm);
    h.expand(info, &mut okm)
        .expect("hdkf expand - something went wrong");
    okm
}

pub fn blake2b(
    digest_size: usize,
    message: &[u8],
) -> Result<Box<[u8]>, Box<dyn std::error::Error>> {
    let mut hasher = VarBlake2b::new(digest_size)?;
    hasher.update(message);
    Ok(hasher.finalize_boxed())
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::convert::TryInto;

    #[test]
    fn can_decrypt() {
        let pw = b"strong password";
        let data = b"sensitive";
        let hkdf_info = b"uniqueinfo";
        let (ciphertext, nonce) = aes_gcm_encrypt(pw, data, hkdf_info);
        let og: [u8; 9] = aes_gcm_decrypt(pw, nonce, &ciphertext, hkdf_info)
            .unwrap()
            .as_slice()
            .try_into()
            .expect("failed decrypt content size");
        assert_eq!(*data, og);
    }
}
