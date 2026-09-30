//! Currency comes from reported totals; plan windows remain account levels.

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::tracking::Unavailable;

/// One reported account window, independent of a session's spend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanWindow {
    /// The reported window's duration.
    pub duration_minutes: u64,
    /// The account's reported percentage, retaining fractional precision.
    #[schema(value_type = f64)]
    pub used_percent: Number,
    /// The reported reset instant in milliseconds since the Unix epoch.
    pub resets_at_ms: u64,
}

/// A nonnegative decimal rounded to the nearest scaled integer, with overflow refused.
pub fn scaled(number: &Number, places: i32) -> Option<u64> {
    projection(number, places, true)
}

/// A boundary amount must be represented exactly, rather than rounded into a limit.
pub fn scaled_exact(number: &Number, places: i32) -> Option<u64> {
    projection(number, places, false)
}

fn projection(number: &Number, places: i32, rounded: bool) -> Option<u64> {
    let text = number.to_string();
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (text.as_str(), 0),
    };
    if mantissa.starts_with('-') {
        return None;
    }
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = whole
        .bytes()
        .chain(fraction.bytes())
        .try_fold(0_u128, |held, digit| {
            if !digit.is_ascii_digit() {
                return None;
            }
            held.checked_mul(10)?.checked_add(u128::from(digit - b'0'))
        })?;
    let shift = exponent
        .checked_add(places)?
        .checked_sub(i32::try_from(fraction.len()).ok()?)?;
    let amount = if shift >= 0 {
        digits.checked_mul(10_u128.checked_pow(u32::try_from(shift).ok()?)?)?
    } else if shift < -38 {
        if rounded || digits == 0 {
            0
        } else {
            return None;
        }
    } else {
        let divisor = 10_u128.checked_pow(shift.unsigned_abs())?;
        if rounded {
            digits.checked_add(divisor / 2)?.checked_div(divisor)?
        } else if digits % divisor == 0 {
            digits / divisor
        } else {
            return None;
        }
    };
    u64::try_from(amount).ok()
}

/// Validate the provider's ordinary numeric percentage without inventing a missing figure.
pub fn percent(value: &Value) -> Option<Number> {
    let percent = value.as_f64()?;
    if !(0.0..=100.0).contains(&percent) {
        return None;
    }
    value.as_number().cloned()
}

/// The status line's reported cost, projected to the nearest microdollar.
pub fn dollars(input: &Value, notes: &mut Vec<Unavailable>) -> Option<u64> {
    let given = input
        .get("cost")
        .and_then(|cost| cost.get("total_cost_usd"));
    let figure = given
        .and_then(Value::as_number)
        .and_then(|number| scaled(number, 6));
    if figure.is_none() {
        notes.push(Unavailable {
            figure: "dollars_micros".to_owned(),
            reason: if given.is_none_or(Value::is_null) {
                "reported_cost_absent"
            } else {
                "reported_cost_invalid"
            }
            .to_owned(),
        });
    }
    figure
}

/// The status line names these windows by duration, never by a guessed plan.
pub fn windows(input: &Value, notes: &mut Vec<Unavailable>) -> Vec<PlanWindow> {
    let limits = input.get("rate_limits");
    let mut windows = Vec::new();
    for (name, duration_minutes) in [("five_hour", 300), ("seven_day", 10_080)] {
        let given = limits.and_then(|limits| limits.get(name));
        let window = given.and_then(|given| {
            Some(PlanWindow {
                duration_minutes,
                used_percent: percent(given.get("used_percentage")?)?,
                resets_at_ms: given.get("resets_at")?.as_u64()?.checked_mul(1000)?,
            })
        });
        if let Some(window) = window {
            windows.push(window);
        } else {
            notes.push(Unavailable {
                figure: "plan_windows".to_owned(),
                reason: format!(
                    "{name}_{}",
                    if given.is_none_or(Value::is_null) {
                        "unreported"
                    } else {
                        "invalid"
                    }
                ),
            });
        }
    }
    windows
}

/// Rollout windows carry their duration explicitly; neither slot determines it.
pub fn codex_windows(payload: &Value, notes: &mut Vec<Unavailable>) -> Vec<PlanWindow> {
    let limits = payload.get("rate_limits");
    let mut windows = Vec::new();
    for name in ["primary", "secondary"] {
        let given = limits.and_then(|limits| limits.get(name));
        let window = given.and_then(|given| {
            Some(PlanWindow {
                duration_minutes: given
                    .get("window_minutes")?
                    .as_u64()
                    .filter(|duration| *duration > 0)?,
                used_percent: percent(given.get("used_percent")?)?,
                resets_at_ms: given.get("resets_at")?.as_u64()?.checked_mul(1000)?,
            })
        });
        if let Some(window) = window {
            windows.push(window);
        } else {
            notes.push(Unavailable {
                figure: "plan_windows".to_owned(),
                reason: format!(
                    "{name}_{}",
                    if given.is_none_or(Value::is_null) {
                        "unreported"
                    } else {
                        "invalid"
                    }
                ),
            });
        }
    }
    windows
}

/// Retain only live reported windows and explain every expired one.
pub fn live(windows: &mut Vec<PlanWindow>, now: u64, notes: &mut Vec<Unavailable>) {
    windows.retain(|window| {
        if window.resets_at_ms > now {
            return true;
        }
        notes.push(Unavailable {
            figure: "plan_windows".to_owned(),
            reason: format!("{}_minute_window_expired", window.duration_minutes),
        });
        false
    });
}

/// Keep the cumulative baseline; a restarted counter contributes its new total.
pub fn cost_delta(
    source: &mut crate::tracking_store::SourceState,
    figures: &mut crate::tracking::Figures,
    notes: &mut Vec<Unavailable>,
) {
    let Some(total) = figures.dollars_micros else {
        return;
    };
    let previous = source.reported_cost_micros.replace(total);
    if let Some(previous) = previous {
        figures.dollars_micros = total.checked_sub(previous);
        if figures.dollars_micros.is_none() {
            figures.dollars_micros = Some(total);
            notes.push(Unavailable {
                figure: "dollars_micros".to_owned(),
                reason: "reported_session_cost_reset".to_owned(),
            });
        }
    }
}

/// Compare the provider's ordinary numeric percentage levels.
///
/// # Errors
/// Refuses a percentage that cannot be represented as a finite number.
pub fn compare(left: &Number, right: &Number) -> Result<std::cmp::Ordering, String> {
    let left = left
        .as_f64()
        .ok_or("reported percentage is not representable")?;
    let right = right
        .as_f64()
        .ok_or("percentage threshold is not representable")?;
    left.partial_cmp(&right)
        .ok_or_else(|| "reported percentage is not finite".to_owned())
}
