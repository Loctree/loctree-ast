//! Deterministic `loct anchors` catalog emission.
//!
//! The catalog builder itself lives in [`crate::anchors`] (core domain
//! logic, consumable without CLI handler imports — see the
//! `pack_does_not_import_cli_handlers` contract); this handler owns the CLI
//! argument handling and stdout emission only.

use std::path::Path;

use super::super::super::command::AnchorsOptions;
use super::super::{DispatchResult, GlobalOptions, load_or_create_snapshot};

pub use crate::anchors::build_anchor_catalog;

pub fn handle_anchors_command(opts: &AnchorsOptions, global: &GlobalOptions) -> DispatchResult {
    let root = opts.root.as_deref().unwrap_or_else(|| Path::new("."));
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let snapshot = match load_or_create_snapshot(&root, global) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("[loct][error] failed to load snapshot: {error}");
            return DispatchResult::Exit(1);
        }
    };

    let catalog = match build_anchor_catalog(&snapshot, &root) {
        Ok(catalog) => catalog,
        Err(error) => {
            eprintln!("[loct][error] failed to emit anchor catalog: {error}");
            return DispatchResult::Exit(1);
        }
    };

    match serde_json::to_string_pretty(&catalog) {
        Ok(json) => {
            println!("{json}");
            DispatchResult::Exit(0)
        }
        Err(error) => {
            eprintln!("[loct][error] failed to serialize anchor catalog: {error}");
            DispatchResult::Exit(1)
        }
    }
}
