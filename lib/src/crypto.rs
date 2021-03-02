use aes::Aes256;
use block_modes::{block_padding::NoPadding, BlockMode, Cbc};
use data_encoding::BASE64;

use crate::error::InternalResult;

type Aes256Cbc = Cbc<Aes256, NoPadding>;

/// This will return copied data so the lib does not know how to handle
/// decryption.
pub(crate) fn decrypt_data(key: &[u8], data: &str) -> InternalResult<Vec<u8>> {
    // IV on authy is always empty
    let iv = [0u8; 16];
    let cipher = Aes256Cbc::new_var(key, &iv).unwrap();

    let mut buffer = BASE64
        .decode(data.as_bytes())
        .expect("data is not valid base64");

    let output = cipher.decrypt(&mut buffer)?;

    let padding_length = *output.last().unwrap() as usize;
    // Sometimes the padding is used
    let content_size = output
        .len()
        .checked_sub(padding_length)
        .unwrap_or_else(|| output.len());

    let without_padding = output[0..content_size]
        .iter()
        .map(|chr| ascii_uppercase(*chr))
        .collect::<Vec<u8>>();

    Ok(without_padding)
}

fn ascii_uppercase(chr: u8) -> u8 {
    if chr >= 97 && chr <= 122 {
        chr - 32
    } else {
        chr
    }
}
