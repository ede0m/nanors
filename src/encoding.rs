use bitvec::prelude::*;
use blake2::digest::{Update, VariableOutput};
use blake2::VarBlake2b;

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

pub fn blake2b(
    digest_size: usize,
    message: &[u8],
) -> Result<Box<[u8]>, Box<dyn std::error::Error>> {
    let mut hasher = VarBlake2b::new(digest_size)?;
    hasher.update(message);
    Ok(hasher.finalize_boxed())
}
