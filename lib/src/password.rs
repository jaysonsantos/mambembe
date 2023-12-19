use std::num::NonZeroU32;

use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha1::Sha1;

const AUTHY_ITERATIONS: Option<NonZeroU32> = NonZeroU32::new(1000);

pub(crate) fn derive_key(backup_password: &str, salt: &str) -> Vec<u8> {
    let mut derived_key = [0u8; 32];
    pbkdf2::<Hmac<Sha1>>(
        backup_password.as_bytes(),
        salt.as_bytes(),
        AUTHY_ITERATIONS.unwrap().into(),
        &mut derived_key,
    ).expect("failed to derive key");

    derived_key.to_vec()
}

#[cfg(test)]
mod tests {
    use crate::password::derive_key;

    #[test]
    fn test_derive_key_implementation() {
        let expected = [
            84, 238, 29, 216, 57, 143, 244, 224, 255, 82, 192, 61, 32, 22, 16, 55, 101, 165, 19,
            21, 21, 89, 206, 233, 116, 212, 54, 78, 196, 147, 85, 132,
        ];
        assert_eq!(derive_key("test", "salty"), &expected);
    }
}
