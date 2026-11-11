use super::geometry;
#[path = "desktop_layout_cache_blocking.rs"]
pub mod cache_blocking;
#[path = "desktop_layout_cache_normalization.rs"]
mod normalization;
#[path = "desktop_layout_cache_parent_summary.rs"]
mod parent_summary;
#[path = "desktop_layout_cache_products.rs"]
mod products;
#[path = "desktop_layout_cache_context.rs"]
pub mod context;
pub use context::{parent_summary_with_context, statistics_with_context, GeometryContext};
