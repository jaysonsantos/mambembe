use std::num::NonZeroU32;

use ring::pbkdf2;

const AUTHY_ITERATIONS: Option<NonZeroU32> = NonZeroU32::new(1000);

pub(crate) fn derive_key(backup_password: &str, salt: &str) -> Vec<u8> {
    let mut derived_key = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA1,
        AUTHY_ITERATIONS.unwrap(),
        salt.as_bytes(),
        backup_password.as_bytes(),
        &mut derived_key,
    );

    derived_key.to_vec()
}
