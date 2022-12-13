library lib;

use std::u128::U128;

pub fn sqrt_by_digit(n: U128) -> U128 {
    let mut x = n;
    let mut c = U128::min();
    let mut d = U128::from((0, 1)) << U128::bits() - 2;

    while d > n {
        d = d >> 2;
    }

    while d != U128::min() {
        if x > c + d || x == c + d { // TODO: gte
            x = x - (c + d);
            c = (c >> 1) + d;
        } else {
            c = c >> 1;
        }
        d  = d >> 2;
    }

    c
}
