//! Exact dollar amounts at command-line boundaries.
//!
//! A budget is authority to spend, so rounding it is widening. Values are parsed from their
//! decimal spelling into integer millionths of a dollar and are either exact or refused.

use anyhow::{bail, Context as _, Result};

/// Millionths in one US dollar.
pub(crate) const MICRO_USD: u64 = 1_000_000;

/// Parses a decimal number of US dollars as millionths of one.
///
/// A leading dollar sign is accepted. Exponents, signs and more than six fractional digits are
/// refused so no caller can silently round authority up or down.
pub(crate) fn micro_usd(written: &str) -> Result<u64> {
    let text = written.trim().trim_start_matches('$');
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (text, ""),
    };
    let readable = !whole.is_empty()
        && whole.chars().all(|character| character.is_ascii_digit())
        && fraction.chars().all(|character| character.is_ascii_digit())
        && fraction.len() <= 6;
    if !readable {
        bail!(
            "`{written}` is not an amount in US dollars, such as `10` or `0.25`. Six decimal \
             places at most, and no exponent: a cost that cannot be converted exactly is one this \
             runner will not use as authority to spend"
        );
    }
    let whole: u64 = whole.parse().context("the dollars part is too large")?;
    let padded = format!("{fraction:0<6}");
    let millionths: u64 = if padded.is_empty() {
        0
    } else {
        padded.parse().context("the cents part is too large")?
    };
    whole
        .checked_mul(MICRO_USD)
        .and_then(|dollars| dollars.checked_add(millionths))
        .context("the amount is too large to count in millionths of a dollar")
}

/// Renders millionths of a dollar without losing trailing precision.
pub(crate) fn dollars(micro: u64) -> String {
    format!("${}.{:06}", micro / MICRO_USD, micro % MICRO_USD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_dollar_amounts_are_exact_or_refused() {
        assert_eq!(micro_usd("0.0714").expect("an exact amount"), 71_400);
        assert_eq!(micro_usd("$1.00").expect("a dollar sign"), 1_000_000);
        assert_eq!(dollars(71_400), "$0.071400");

        for written in ["-1", "1e-7", "0.0000001", "", "one"] {
            assert!(
                micro_usd(written).is_err(),
                "`{written}` cannot become spending authority"
            );
        }
    }
}
