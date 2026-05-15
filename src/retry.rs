//! Exponential backoff with cap.

use crate::config::RetrySpec;

/// Returns the delay (ms) before attempt N (0-indexed: N=0 is the very first retry,
/// which fires `initial_backoff_ms` after the failed attempt).
pub fn backoff_ms(spec: &RetrySpec, retry_index: u32) -> u64 {
    let initial = spec.initial_backoff_ms;
    let max = spec.max_backoff_ms;
    if initial == 0 {
        return 0;
    }
    let shift = retry_index.min(31);
    initial.saturating_mul(1u64 << shift).min(max)
}

/// True if more attempts are allowed (1-indexed attempt counter).
pub fn should_retry(spec: &RetrySpec, attempts_so_far: u32) -> bool {
    attempts_so_far < spec.max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RetrySpec;

    fn spec() -> RetrySpec {
        RetrySpec {
            max_attempts: 5,
            initial_backoff_ms: 100,
            max_backoff_ms: 1000,
        }
    }

    #[test]
    fn exponential_then_capped() {
        let s = spec();
        assert_eq!(backoff_ms(&s, 0), 100);
        assert_eq!(backoff_ms(&s, 1), 200);
        assert_eq!(backoff_ms(&s, 2), 400);
        assert_eq!(backoff_ms(&s, 3), 800);
        assert_eq!(backoff_ms(&s, 4), 1000);
        assert_eq!(backoff_ms(&s, 20), 1000);
    }

    #[test]
    fn retry_budget() {
        let s = spec();
        assert!(should_retry(&s, 0));
        assert!(should_retry(&s, 4));
        assert!(!should_retry(&s, 5));
    }
}
