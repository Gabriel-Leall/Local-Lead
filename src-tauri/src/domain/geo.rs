use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoRegion {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
    pub depth: u8,
}

impl GeoRegion {
    pub fn from_center(center_lat: f64, center_lng: f64, radius_meters: f64) -> Self {
        let lat_delta = radius_meters / 111_320.0;
        let lng_delta = radius_meters / (111_320.0 * (center_lat.to_radians().cos().max(0.2)));
        Self {
            north: center_lat + lat_delta,
            south: center_lat - lat_delta,
            east: center_lng + lng_delta,
            west: center_lng - lng_delta,
            depth: 0,
        }
    }

    pub fn center(&self) -> (f64, f64) {
        ((self.north + self.south) / 2.0, (self.east + self.west) / 2.0)
    }

    pub fn width_meters(&self) -> f64 {
        let (_, lng_c) = self.center();
        (self.east - self.west).abs() * 111_320.0 * (lng_c.to_radians().cos().max(0.2))
    }

    pub fn height_meters(&self) -> f64 {
        (self.north - self.south).abs() * 111_320.0
    }

    pub fn radius_meters(&self) -> f64 {
        (self.width_meters().max(self.height_meters())) / 2.0
    }

    pub fn split_into_four(&self) -> [Self; 4] {
        let (clat, clng) = self.center();
        let d = self.depth + 1;
        [
            Self { north: self.north, south: clat, east: clng, west: self.west, depth: d },
            Self { north: self.north, south: clat, east: self.east, west: clng, depth: d },
            Self { north: clat, south: self.south, east: clng, west: self.west, depth: d },
            Self { north: clat, south: self.south, east: self.east, west: clng, depth: d },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_contains_center() {
        let r = GeoRegion::from_center(25.7617, -80.1918, 30_000.0);
        assert!(r.north > 25.7617 && r.south < 25.7617);
        assert!(r.east > -80.1918 && r.west < -80.1918);
        assert_eq!(r.depth, 0);
    }

    #[test]
    fn split_covers_parent() {
        let r = GeoRegion::from_center(0.0, 0.0, 10_000.0);
        let kids = r.split_into_four();
        assert!(kids.iter().all(|k| k.depth == 1));
        let (clat, clng) = r.center();
        assert!((kids[0].south - clat).abs() < 1e-9);
        assert!((kids[0].east - clng).abs() < 1e-9);
    }

    #[test]
    fn size_shrinks_with_depth() {
        let r = GeoRegion::from_center(0.0, 0.0, 10_000.0);
        let kids = r.split_into_four();
        assert!(kids[0].width_meters() < r.width_meters());
    }
}
