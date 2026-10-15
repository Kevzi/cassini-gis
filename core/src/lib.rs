//! Moteur de Cassini.
//!
//! Toute la logique métier vit ici. L'interface, la CLI, l'agent local et le
//! serveur MCP ne font qu'appeler ce moteur (CLAUDE.md §2.5).

pub mod basemap;
pub mod commands;
pub mod registry;

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
