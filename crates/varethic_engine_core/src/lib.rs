//! varethic-engine-core: shared types and a machine-readable registry of
//! Chromium component migration status. This is the honest bookkeeping layer:
//! a component is only marked `Migrated` when its Rust replacement passes
//! tests and is integrated.

pub use varethic_url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatus {
    /// Not started; C/C++ implementation still authoritative.
    Legacy,
    /// Rust implementation exists and passes unit tests, not yet integrated.
    InProgress,
    /// Rust implementation integrated and verified against legacy behavior.
    Migrated,
}

#[derive(Debug, Clone)]
pub struct ComponentEntry {
    pub chromium_path: &'static str,
    pub name: &'static str,
    pub rust_crate: Option<&'static str>,
    pub status: MigrationStatus,
    pub notes: &'static str,
}

pub const COMPONENT_REGISTRY: &[ComponentEntry] = &[
    ComponentEntry {
        chromium_path: "//url",
        name: "URL parsing / GURL / omnibox classification",
        rust_crate: Some("varethic-url"),
        status: MigrationStatus::InProgress,
        notes: "Core parse/validate/classify implemented with unit tests; WHATWG conformance and IDN not yet complete.",
    },
    ComponentEntry {
        chromium_path: "//net",
        name: "Network stack",
        rust_crate: Some("varethic-net"),
        status: MigrationStatus::InProgress,
        notes: "HTTP/1.1 GET over HTTP+HTTPS (rustls/webpki-roots), local-server and live-network tested. Redirects/cookies/cache/HTTP2 not yet implemented.",
    },
    ComponentEntry {
        chromium_path: "//content",
        name: "Content / navigation",
        rust_crate: None,
        status: MigrationStatus::Legacy,
        notes: "Not started.",
    },
    ComponentEntry {
        chromium_path: "//services/network",
        name: "Network service (network process)",
        rust_crate: None,
        status: MigrationStatus::Legacy,
        notes: "Not started.",
    },
    ComponentEntry {
        chromium_path: "//mojo",
        name: "Mojo IPC",
        rust_crate: None,
        status: MigrationStatus::Legacy,
        notes: "Not started.",
    },
];

pub fn migration_summary() -> (usize, usize, usize) {
    let mut legacy = 0;
    let mut in_progress = 0;
    let mut migrated = 0;
    for e in COMPONENT_REGISTRY {
        match e.status {
            MigrationStatus::Legacy => legacy += 1,
            MigrationStatus::InProgress => in_progress += 1,
            MigrationStatus::Migrated => migrated += 1,
        }
    }
    (legacy, in_progress, migrated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_nonempty_and_consistent() {
        assert!(!COMPONENT_REGISTRY.is_empty());
        for e in COMPONENT_REGISTRY {
            if e.status == MigrationStatus::Migrated {
                assert!(
                    e.rust_crate.is_some(),
                    "migrated component must name its crate"
                );
            }
        }
    }
}
