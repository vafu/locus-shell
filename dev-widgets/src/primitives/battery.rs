
pub(super) fn battery_fraction(percent: &f64) -> f64 {
    (percent / 100.0).clamp(0.0, 1.0)
}

pub(super) fn battery_label(percent: f64) -> String {
    format!("{percent:.0}%")
}
