pub mod google_places;
pub mod maps_scraper;

pub use google_places::GooglePlacesProvider;

use crate::domain::DiscoveredPlace;
use crate::error::AppError;

pub struct SearchRequest {
    pub query: String,
    pub city: String,
    pub api_key: String,
}

pub struct RegionSearchRequest {
    pub query: String,
    pub api_key: String,
    pub center_lat: f64,
    pub center_lng: f64,
    pub radius_meters: f64,
}

#[async_trait::async_trait]
pub trait DiscoveryProvider: Send + Sync {
    async fn search(&self, req: &SearchRequest) -> Result<Vec<DiscoveredPlace>, AppError>;
    async fn search_region(&self, req: &RegionSearchRequest) -> Result<Vec<DiscoveredPlace>, AppError>;
}
