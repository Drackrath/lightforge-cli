//! Lightforge core.
//!
//! UI-agnostic domain logic: product models, the on-disk path layout, the
//! `.build.info` engine, prefix provisioning, Proton-GE version management,
//! TACT/CASC downloading, Battle.net, and addon management.
//!
//! Networking lives behind `async fn` in the modules that need it; everything
//! else stays synchronous and directly testable. `core` never depends on clap.

#![cfg_attr(test, allow(clippy::cast_possible_truncation))]

pub mod addons;
pub mod advice;
pub mod backup;
pub mod battlenet;
pub mod buildinfo;
pub mod clientlocale;
pub mod companion;
pub mod config;
pub(crate) mod cvars;
pub mod debug;
pub mod discover;
pub mod doctor;
pub mod error;
pub mod families;
pub(crate) mod fsutil;
#[cfg(fuzzing)]
pub mod fuzzing;
pub mod library;
pub mod logging;
pub mod migrate;
pub mod net;
pub mod paths;
pub mod portal;
pub mod prefix;
pub mod prefix_move;
pub mod preflight;
pub mod product;
pub mod proton;
pub mod runtime;
pub mod setup;
mod staging;
pub mod state;
pub mod steam;
pub mod storage;
pub mod tact;
pub mod text;
pub mod transfers;
pub mod tweaks;

pub use error::{CoreError, Result};
pub use paths::Layout;
pub use product::{Product, ProductCode, ProductProfile};
pub use state::ActionState;
