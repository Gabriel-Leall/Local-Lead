use crate::domain::GeoRegion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPolicy {
    pub saturation_threshold: usize,
    pub max_depth: u8,
    pub min_cell_width_meters: f64,
    pub min_cell_height_meters: f64,
    pub min_new_unique_ratio: f64,
}

impl Default for SplitPolicy {
    fn default() -> Self {
        Self {
            saturation_threshold: 18,
            max_depth: 4,
            min_cell_width_meters: 500.0,
            min_cell_height_meters: 500.0,
            min_new_unique_ratio: 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SplitDecision {
    Complete,
    Split,
    StopDepth,
    StopSmall,
    StopLowGain,
}

pub fn should_split(
    policy: &SplitPolicy,
    region: &GeoRegion,
    result_count: usize,
    new_unique: usize,
) -> SplitDecision {
    if region.depth >= policy.max_depth {
        return SplitDecision::StopDepth;
    }
    if region.width_meters() < policy.min_cell_width_meters
        || region.height_meters() < policy.min_cell_height_meters
    {
        return SplitDecision::StopSmall;
    }
    if result_count < policy.saturation_threshold {
        return SplitDecision::Complete;
    }
    if result_count > 0 {
        let ratio = new_unique as f64 / result_count as f64;
        if ratio < policy.min_new_unique_ratio {
            return SplitDecision::StopLowGain;
        }
    }
    SplitDecision::Split
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(depth: u8) -> GeoRegion {
        let mut r = GeoRegion::from_center(25.7, -80.3, 30_000.0);
        r.depth = depth;
        r
    }

    #[test]
    fn small_result_completes() {
        let p = SplitPolicy::default();
        assert_eq!(should_split(&p, &region(0), 5, 5), SplitDecision::Complete);
    }

    #[test]
    fn saturated_splits() {
        let p = SplitPolicy::default();
        assert_eq!(should_split(&p, &region(0), 20, 15), SplitDecision::Split);
    }

    #[test]
    fn respects_max_depth() {
        let p = SplitPolicy::default();
        assert_eq!(should_split(&p, &region(10), 20, 20), SplitDecision::StopDepth);
    }

    #[test]
    fn low_gain_stops() {
        let p = SplitPolicy::default();
        assert_eq!(should_split(&p, &region(0), 20, 1), SplitDecision::StopLowGain);
    }
}
