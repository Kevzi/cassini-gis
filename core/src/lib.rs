//! Moteur de Cassini.
//!
//! Toute la logique métier vit ici. L'interface, la CLI, l'agent local et le
//! serveur MCP ne font qu'appeler ce moteur (CLAUDE.md §2.5).

pub mod agent_inventory;
pub mod angle;
pub mod attribute_guard;
pub mod basemap;
pub mod commands;
pub mod contract;
pub mod cost;
pub mod dem;
pub mod error;
pub mod exec;
pub mod external;
pub mod focus;
pub mod geodesic;
pub mod geodesy;
pub mod geojson;
pub mod horn;
pub mod journal;
pub mod plan;
pub mod plan_completion;
pub mod point;
pub mod raster_path;
pub mod registry;
pub mod serve;
pub mod style;
pub mod wmm;
pub mod work_crs;

/// Version du moteur, alignée sur celle du workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
