use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use data_encoding::Encoding;
use lazy_static::lazy_static;

use crate::error::{InternalError, InternalResult};

type Aes256Cbc = cbc::Decryptor<aes::Aes256>;

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
    let cipher = Aes256Cbc::new(key.into(), &iv.into());

    let buffer = BASE64
        .decode(data.as_bytes())
        .expect("data is not valid base64");

    cipher
        .decrypt_padded_vec_mut::<Pkcs7>(&buffer)
        .map_err(|_| InternalError::DecryptionError)
}

#[cfg(test)]
mod tests {
    use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};

    use super::BASE64;
    use crate::{crypto::decrypt_data, password::derive_key};

    type Aes256CbcEncryptor = cbc::Encryptor<aes::Aes256>;

    #[test]
    fn test_base64_encoder_decoder() {
        let expected = "hello world";
        let line_break = "aGVsbG8g\nd29ybGQ=";
        let windows_line_break = "aGVsbG8g\r\nd29ybGQ=";
        let cases = [line_break, windows_line_break];

        for case in &cases {
            let decoded = BASE64
                .decode(case.as_bytes())
                .unwrap_or_else(|_| panic!("data {:?} is not valid base64", case));
            let parsed = String::from_utf8_lossy(&decoded);
            assert_eq!(&parsed, expected);
        }
    }

    #[cfg(test)]
    fn encrypt_data(key: &[u8], data: &[u8]) -> String {
        let iv = [0u8; 16];
        let cipher = Aes256CbcEncryptor::new(key.into(), &iv.into());
        let encrypted = cipher.encrypt_padded_vec_mut::<Pkcs7>(data);
        BASE64.encode(&encrypted)
    }

    #[test]
    fn test_decrypt_data() {
        let key = derive_key("123456", "salty");
        let data_to_encrypt = b"my secret seed01";
        let encrypted = encrypt_data(&key, data_to_encrypt);
        let decrypted = decrypt_data(&key, &encrypted).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&decrypted),
            String::from_utf8_lossy(&data_to_encrypt[..])
        );
    }
}
