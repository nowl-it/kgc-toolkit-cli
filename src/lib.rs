//! KGC Toolkit - CLI for King God Castle asset management
//!
//! This library provides tools for:
//! - Downloading XAPK from APKPure
//! - Converting XAPK to Unity project (via AssetRipper)
//! - Comparing Unity project versions
//! - MITM proxy for API traffic capture
//! - Unity prefab parsing
//! - XML config extraction

pub mod cli;
pub mod config;
pub mod core;
pub mod deps;
pub mod proxy;
pub mod tui;
pub mod utils;

pub use core::kgc;
