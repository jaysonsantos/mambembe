use std::time::SystemTime;

use data_encoding::BASE32;
use itertools::Itertools;
use slauth::oath::{
    hotp::{HOTPBuilder, HOTPContext},
    HashesAlgorithm,
};

use crate::{client::TimeSync, error::InternalResult};

const DEFAULT_OTP_DIGITS: usize = 7;
const AUTHY_DEFAULT_PERIOD: u64 = 10;
const OTHERS_DEFAULT_PERIOD: u64 = 30;

pub(crate) fn calculate_token(
    seed: &[u8],
    digits: usize,
    time_sync: Option<&TimeSync>,
) -> InternalResult<String> {
    let seed = BASE32.decode(seed).unwrap_or_else(|_| seed.to_vec());
    // let seed = HEXLOWER.encode(&seed);
    let time = get_time(time_sync);
    let s = build_slauth_context(&seed, digits, time / OTHERS_DEFAULT_PERIOD);
    Ok(s.gen())
}

pub(crate) fn build_slauth_context(seed: &[u8], digits: usize, padded_time: u64) -> HOTPContext {
    HOTPBuilder::new()
        .algorithm(HashesAlgorithm::SHA1)
        .secret(seed)
        .counter(padded_time)
        .digits(digits)
        .build()
}

// pub(crate) fn build_slauth_context(
//     seed: &[u8],
//     digits: usize,
//     period: Option<u64>,
//     time_sync: Option<&TimeSync>,
// ) -> TOTPContext {
//     let (backward, forward) = time_sync
//         .map(|t| t.get_time_offset_for_slauth())
//         .unwrap_or((0, 0));

//     TOTPBuilder::new()
//         .algorithm(HashesAlgorithm::SHA1)
//         .secret(seed)
//         .period(period.unwrap_or(OTHERS_DEFAULT_PERIOD))
//         .re_sync_parameter(backward, forward)
//         .digits(digits)
//         .build()
// }

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
