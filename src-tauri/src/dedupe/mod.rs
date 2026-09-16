pub mod fingerprint;
pub mod matcher;

pub use fingerprint::normalize_name;
pub use matcher::{find_cross_provider_match, Match};
