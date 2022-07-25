use aes::Aes256;
use block_modes::{block_padding::NoPadding, BlockMode, Cbc};
use data_encoding::Encoding;
use lazy_static::lazy_static;

use crate::error::InternalResult;

type Aes256Cbc = Cbc<Aes256, NoPadding>;

lazy_static! {
    static ref BASE64: Encoding = {
        let mut spec = data_encoding::BASE64.specification();
        spec.ignore = "\r\n".into();
        spec.encoding()
            .expect("failed to build base64 encoder/decoder")
    };
}

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

    let without_padding = &output[0..content_size];

    Ok(without_padding.to_vec())
}

#[cfg(test)]
mod tests {
    use block_modes::BlockMode;

    use crate::crypto::{Aes256Cbc, decrypt_data};
    use crate::password::derive_key;
    use super::BASE64;

    #[test]
    fn test_base64_encoder_decoder() {
        let expected = "hello world";
        let line_break = "aGVsbG8g\nd29ybGQ=";
        let windows_line_break = "aGVsbG8g\r\nd29ybGQ=";
        let cases = [line_break, windows_line_break];

        for case in &cases {
            let decoded = BASE64
                .decode(case.as_bytes())
                .expect(&format!("data {:?} is not valid base64", case));
            let parsed = String::from_utf8_lossy(&decoded);
            assert_eq!(&parsed, expected);
        }
    }

    #[cfg(test)]
    fn encrypt_data(key: &[u8], data: &[u8]) -> String {
        let iv = [0u8; 16];
        let cipher = Aes256Cbc::new_var(key, &iv).unwrap();
        let mut buffer = data.to_vec();
        let len = buffer.len();
        let encrypted = cipher.encrypt(&mut buffer, len).unwrap();
        BASE64.encode(encrypted)
    }

    #[test]
    fn test_decrypt_data() {
        let key = derive_key("123456", "salty");
        let data_to_encrypt = b"my secret seed01";
        let encrypted = encrypt_data(&key, data_to_encrypt);
        let decrypted = decrypt_data(&key, &encrypted).unwrap();
        assert_eq!(String::from_utf8_lossy(&decrypted), String::from_utf8_lossy(&data_to_encrypt[..]));
    }
}
