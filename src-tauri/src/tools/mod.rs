//! Tool inspection subsystem: definitions, version checking, latest-version
//! lookup, package enumeration, history, and settings.

pub mod checker;
pub mod history;
pub mod latest;
pub mod packages;
pub mod registry;
pub mod settings;
pub mod types;

pub mod projects;
pub mod sizes;
pub mod source_files;
pub mod versions;

pub use checker::{check_all, check_one, is_outdated};
pub use history::History;
pub use latest::{fetch_latest, LatestCache};
pub use packages::list_packages;
pub use registry::find;
pub use settings::{active_definitions, NotifyMode, Settings};
pub use sizes::measure_sizes;
pub use types::*;
