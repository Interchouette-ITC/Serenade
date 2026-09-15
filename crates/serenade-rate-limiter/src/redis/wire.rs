//! Serialize [`WindowState`] with unix-millis timestamps (Instant bridged at the boundary).

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{RateLimiterError, WindowState};

/// Encodes `state` for Redis using wall-clock millis relative to `now_instant`.
pub(super) fn encode_state(state: &WindowState, now_instant: Instant, now_wall_ms: u64) -> String {
    match *state {
        WindowState::TokenBucket {
            tokens_scaled,
            updated_at,
        } => {
            let updated_ms = instant_to_wall_ms(updated_at, now_instant, now_wall_ms);
            format!("tb:{tokens_scaled}:{updated_ms}")
        }
        WindowState::FixedWindow {
            count,
            window_start,
        } => {
            let start_ms = instant_to_wall_ms(window_start, now_instant, now_wall_ms);
            format!("fw:{count}:{start_ms}")
        }
    }
}

/// Decodes a Redis value into [`WindowState`], mapping millis onto `now_instant`.
pub(super) fn decode_state(
    raw: &str,
    now_instant: Instant,
    now_wall_ms: u64,
) -> Result<WindowState, RateLimiterError> {
    let mut parts = raw.splitn(3, ':');
    let kind = parts.next().unwrap_or("");
    let mid = parts.next().ok_or_else(|| corrupt("missing fields"))?;
    let tail = parts.next().ok_or_else(|| corrupt("missing timestamp"))?;
    match kind {
        "tb" => {
            let tokens_scaled = mid.parse::<u128>().map_err(|_| corrupt("bad tokens"))?;
            let updated_ms = tail.parse::<u64>().map_err(|_| corrupt("bad millis"))?;
            Ok(WindowState::TokenBucket {
                tokens_scaled,
                updated_at: wall_ms_to_instant(updated_ms, now_instant, now_wall_ms),
            })
        }
        "fw" => {
            let count = mid.parse::<u32>().map_err(|_| corrupt("bad count"))?;
            let start_ms = tail.parse::<u64>().map_err(|_| corrupt("bad millis"))?;
            Ok(WindowState::FixedWindow {
                count,
                window_start: wall_ms_to_instant(start_ms, now_instant, now_wall_ms),
            })
        }
        _ => Err(corrupt("unknown kind")),
    }
}

pub(super) fn unix_millis_now() -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

fn instant_to_wall_ms(instant: Instant, now_instant: Instant, now_wall_ms: u64) -> u64 {
    if instant <= now_instant {
        let delta = now_instant.duration_since(instant);
        let delta_ms = u64::try_from(delta.as_millis()).unwrap_or(u64::MAX);
        now_wall_ms.saturating_sub(delta_ms)
    } else {
        let delta = instant.duration_since(now_instant);
        let delta_ms = u64::try_from(delta.as_millis()).unwrap_or(u64::MAX);
        now_wall_ms.saturating_add(delta_ms)
    }
}

fn wall_ms_to_instant(stored_ms: u64, now_instant: Instant, now_wall_ms: u64) -> Instant {
    if stored_ms <= now_wall_ms {
        let delta = Duration::from_millis(now_wall_ms - stored_ms);
        now_instant.checked_sub(delta).unwrap_or(now_instant)
    } else {
        let delta = Duration::from_millis(stored_ms - now_wall_ms);
        now_instant.checked_add(delta).unwrap_or(now_instant)
    }
}

fn corrupt(detail: &str) -> RateLimiterError {
    RateLimiterError::Storage {
        message: format!("corrupt rate-limiter redis value: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_state, encode_state, wall_ms_to_instant};
    use crate::WindowState;
    use std::time::{Duration, Instant};

    #[test]
    fn round_trip_token_bucket_and_fixed_window() {
        let now = Instant::now();
        let wall = 1_700_000_000_000_u64;
        let tb = WindowState::TokenBucket {
            tokens_scaled: 2_000_000,
            updated_at: now,
        };
        let encoded = encode_state(&tb, now, wall);
        let decoded = decode_state(&encoded, now, wall).expect("decode tb");
        assert_eq!(decoded, tb);

        let past = now.checked_sub(Duration::from_millis(250)).expect("past");
        let fw = WindowState::FixedWindow {
            count: 3,
            window_start: past,
        };
        let encoded = encode_state(&fw, now, wall);
        let decoded = decode_state(&encoded, now, wall).expect("decode fw");
        assert!(matches!(
            &decoded,
            WindowState::FixedWindow { count: 3, window_start }
            if now.duration_since(*window_start).as_millis().abs_diff(250) <= 2
        ));

        let future = now.checked_add(Duration::from_millis(40)).expect("future");
        let tb_future = WindowState::TokenBucket {
            tokens_scaled: 1,
            updated_at: future,
        };
        let encoded = encode_state(&tb_future, now, wall);
        let decoded = decode_state(&encoded, now, wall).expect("decode future");
        assert!(matches!(
            decoded,
            WindowState::TokenBucket { updated_at, .. } if updated_at > now
        ));

        assert!(decode_state("xx:1:2", now, 1).is_err());
        assert!(decode_state("tb:nope:1", now, 1).is_err());
        assert!(decode_state("fw:1", now, 1).is_err());
        assert!(decode_state("tb:1:bad", now, 1).is_err());
        let mapped = wall_ms_to_instant(wall + 40, now, wall);
        assert!(mapped > now);
    }
}
