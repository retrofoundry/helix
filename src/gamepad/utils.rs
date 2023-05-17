pub const MAX_N64_AXIS_RANGE: f32 = 80.0;
const OCTAGON_ANGLE_THRESHOLD: f32 = 5.0;
const DEADZONE_PERCENTAGE: f32 = 0.15;

fn normalize_stick_value(value: f32, max_range: f32) -> f32 {
    value * MAX_N64_AXIS_RANGE / max_range
}

fn limit_to_octagon(x: f32, y: f32) -> (f32, f32) {
    // TODO: Implement this
    (x, y)
}

pub fn map_stick_value_to_n64(x: f32, y: f32, max_range: f32) -> Option<(i8, i8)> {
    let adjusted_x = normalize_stick_value(x, max_range);
    let adjusted_y = normalize_stick_value(y, max_range);

    // step 1: create deadzone circle area to discard faulty values
    let magnitude = adjusted_x.powi(2) + adjusted_y.powi(2);
    if magnitude.sqrt() <= (MAX_N64_AXIS_RANGE * DEADZONE_PERCENTAGE) {
        return None;
    }

    // Step 2: Limit values to points within an octagon
    let (limited_x, limited_y) = limit_to_octagon(adjusted_x, adjusted_y);
    return Some((limited_x as i8, limited_y as i8));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_stick_value() {
        assert_eq!(normalize_stick_value(0.0, 1.0), 0.0);
        assert_eq!(normalize_stick_value(0.5, 1.0), 40.0);
        assert_eq!(normalize_stick_value(1.0, 1.0), 80.0);
        assert_eq!(normalize_stick_value(-0.5, 1.0), -40.0);
        assert_eq!(normalize_stick_value(-1.0, 1.0), -80.0);
    }
}
