library observation_helpers;

use std::{
    math::*,
    u256::U256,
};
use exchange_abi::Observation;

/// @notice comparator for 32-bit timestamps
/// @dev safe for 0 or 1 overflows, a and b _must_ be chronologically before or equal to time
/// @param time A timestamp truncated to 32 bits
/// @param a A comparison timestamp from which to determine the relative position of `time`
/// @param b From which to determine the relative position of `time`
/// @return bool Whether `a` is chronologically <= `b`
pub fn lte(time: u64, a: u64, b: u64) -> bool {
    // if there hasn't been overflow, no need to adjust
    if (a <= time && b <= time) {
        a <= b
    } else {
        let a_adjusted = if (a > time) { a } else { a + 2**32 };
        let b_adjusted = if (b > time) { b } else { b + 2**32 };

        a_adjusted <= b_adjusted
    }
}


/// @notice Transforms a previous observation into a new observation, given the passage of time and the current tick and liquidity values
/// @dev timestamp _must_ be chronologically equal to or greater than last.timestamp, safe for 0 or 1 overflows
/// @param last The specified observation to be transformed
/// @param timestamp The timestamp of the new observation
/// @param tick The active tick at the time of the new observation
/// @param liquidity The total in-range liquidity at the time of the new observation
/// @return Observation The newly populated observation
pub fn transform(
    last: Observation,
    current_timestamp: u64,
    price_0: U256,
    price_1: U256,
) -> Observation {
    let delta = U256::from(0, 0, 0, current_timestamp - last.timestamp);
    Observation {
        timestamp: current_timestamp,
        price_0_cumulative_last: last.price_0_cumulative_last + (price_0 * delta),
        price_1_cumulative_last: last.price_1_cumulative_last + (price_1 * delta),
    }
}
