use crate::{clients::brightness::Scale, modules::brightness::config::BrightnessScaleControl};

// See the transfer function https://en.wikipedia.org/wiki/SRGB#Transfer_function_(%22gamma%22)
const GAMMA_22: f32 = 2.2;
const GAMMA_24: f32 = 2.4;
const GAMMA_LOW_COEF: f32 = 12.92;
const GAMMA_HIGH_COEF: f32 = 1.055;
const GAMMA_OFFSET: f32 = 0.055;
/// Cutoff for the transfer function
///
/// Bellow this values are scaled linearly with [GAMMA_LOW_COEF].
const GAMMA_LOW_CUTOFF: f32 = 0.0031308;
/// Same as [GAMMA_LOW_CUTOFF] but used when reverting values.
const REVERT_GAMMA_LOW_CUTOFF: f32 = 0.04045;

pub(super) const MIN_PERCENTAGE: f32 = 0.0;
pub(super) const MAX_SETTABLE_PERCENTAGE: f32 = 1.0;

/// Convert `brightness` through a gamma curve.
fn gamma(brightness: i32, max_brightness: i32, gamma: f32) -> f32 {
    let linear = linear(brightness, max_brightness);

    if linear <= GAMMA_LOW_CUTOFF {
        GAMMA_LOW_COEF * linear
    } else {
        GAMMA_HIGH_COEF * linear.powf(1f32 / gamma) - GAMMA_OFFSET
    }
    .max(MIN_PERCENTAGE)
}

/// Convert `percent` back to a brightness using a gamma curve.
fn revert_gamma(percent: f32, max_brightness: i32, gamma: f32) -> i32 {
    let linear = if percent <= REVERT_GAMMA_LOW_CUTOFF {
        percent / GAMMA_LOW_COEF
    } else {
        ((percent + GAMMA_OFFSET) / GAMMA_HIGH_COEF).powf(gamma)
    };

    revert_linear(linear, max_brightness)
}

/// Convert `brightness` through a linear scale.
fn linear(brightness: i32, max_brightness: i32) -> f32 {
    (brightness as f32 / max_brightness as f32).max(MIN_PERCENTAGE)
}

/// Convert `percentage` back to a brightness values using a linear scale.
fn revert_linear(percentage: f32, max_brightness: i32) -> i32 {
    ((percentage * max_brightness as f32) as i32).clamp(0, max_brightness)
}

/// Convert the `brightness` to a percentage from 0.0 to 1.0.
///
/// `strategy` is used for the conversion strategy.
/// If [BrightnessScaleControl::Automatic] is selected then [Scale] is used to chose which function should be used.
/// [Scale::Linear] makes the [gamma] transfer function be used with [GAMMA_24].
/// If [BrightnessScaleControl::Automatic] is not selected then the enforced transfer function is used.
pub(super) fn percent_from(
    brightness: i32,
    max_brightness: i32,
    strategy: &BrightnessScaleControl,
    scale: &Scale,
) -> f32 {
    match (strategy, scale) {
        (BrightnessScaleControl::Automatic, Scale::Unknown)
        | (BrightnessScaleControl::Linear, _) => linear(brightness, max_brightness),
        (BrightnessScaleControl::Gamma2_2, _) => gamma(brightness, max_brightness, GAMMA_22),
        (BrightnessScaleControl::Gamma2_4, _)
        | (BrightnessScaleControl::Automatic, Scale::Linear) => {
            gamma(brightness, max_brightness, GAMMA_24)
        }
    }
}

/// Convert the `percentage` value back into a brightness value.
///
/// `strategy` is used for the conversion strategy.
/// If [BrightnessScaleControl::Automatic] is selected then [Scale] is used to chose which function should be used.
/// [Scale::Linear] makes the [revert_gamma] transfer function be used with [GAMMA_24].
/// If [BrightnessScaleControl::Automatic] is not selected then the enforced revert transfer function is used.
pub(super) fn brightness_from(
    percentage: f32,
    max_brightness: i32,
    strategy: &BrightnessScaleControl,
    scale: &Scale,
) -> i32 {
    match (strategy, scale) {
        (BrightnessScaleControl::Automatic, Scale::Unknown)
        | (BrightnessScaleControl::Linear, _) => revert_linear(percentage, max_brightness),
        (BrightnessScaleControl::Gamma2_2, _) => revert_gamma(percentage, max_brightness, GAMMA_22),
        (BrightnessScaleControl::Gamma2_4, _)
        | (BrightnessScaleControl::Automatic, Scale::Linear) => {
            revert_gamma(percentage, max_brightness, GAMMA_24)
        }
    }
}
