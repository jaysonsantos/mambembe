use std::time::SystemTime;

use data_encoding::{Encoding, Specification};
use itertools::Itertools;
use lazy_static::lazy_static;
use slauth::oath::{
    hotp::{HOTPBuilder, HOTPContext},
    HashesAlgorithm,
};

use crate::{client::TimeSync, error::InternalResult};

const DEFAULT_OTP_DIGITS: usize = 7;
const AUTHY_DEFAULT_PERIOD: u64 = 10;
const OTHERS_DEFAULT_PERIOD: u64 = 30;

lazy_static! {
    static ref BASE32_NOPAD: Encoding = {
        let mut spec = Specification::new();
        // authy can also have non rfc compliant base32 data
        spec.check_trailing_bits = false;
        spec.symbols.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ234567");
        spec.encoding().unwrap()
    };
}

#[tracing::instrument]
pub(crate) fn calculate_token(
    seed: &[u8],
    digits: usize,
    time_sync: Option<&TimeSync>,
) -> InternalResult<String> {
    let seed = BASE32_NOPAD.decode(seed)?;
    let time = get_time(time_sync);
    let s = build_slauth_context(&seed, digits, time / OTHERS_DEFAULT_PERIOD);
    Ok(s.gen())
}

#[tracing::instrument]
pub(crate) fn build_slauth_context(seed: &[u8], digits: usize, padded_time: u64) -> HOTPContext {
    HOTPBuilder::new()
        .algorithm(HashesAlgorithm::SHA1)
        .secret(seed)
        .counter(padded_time)
        .digits(digits)
        .build()
}

#[tracing::instrument]
pub(crate) fn calculate_future_tokens(
    seed: &[u8],
    time_sync: Option<&TimeSync>,
) -> (String, String, String) {
    let timestamp = get_time(time_sync);
    (0..3)
        .map(|i| timestamp + AUTHY_DEFAULT_PERIOD * i)
        .map(|t| t / AUTHY_DEFAULT_PERIOD)
        .map(|padded_time| build_slauth_context(seed, DEFAULT_OTP_DIGITS, padded_time))
        .map(|ctx| ctx.gen())
        .collect_tuple()
        .expect("should not happen")
}

#[tracing::instrument]
pub(crate) fn get_time(time_sync: Option<&TimeSync>) -> u64 {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    match time_sync {
        Some(time_sync) => time_sync.correct_time(time),
        None => time,
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::calculate_token;

    #[test]
    fn calculate_token_works_with_unpaded_seed() {
        assert!(!calculate_token(b"NBSXS", 6, None).unwrap().is_empty())
    }

    #[test]
    fn non_rfc_conformant_seed() {
        assert!(!calculate_token(
            b"DZQS7RJ3CX7FP4RFZNHMGOH64UIJBSDTLO67TRCGMWG6GDS2IPKT",
            6,
            None
        )
        .unwrap()
        .is_empty())
    }
}
