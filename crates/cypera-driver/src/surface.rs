//! Embedded surface dispatch — the driver serving its own engine's
//! ops through `pavo-driver-kit`'s surface layer, no second binary
//! needed. The `mcp` argv verb (feature-gated) plugs this
//! [`Surface`] impl into `surface::mcp::serve_stdio`.
//!
//! `op` reuses the same `cli.*` path wire queries take
//! (`ops::run_verb` re-execs `run` in a child — output stays
//! captured off the serve stream). `surface` dispatches in-process
//! through `cypera::run_from`, which owns exit codes exactly like
//! the `cypera` bin — that's where any future unbounded or
//! host-interactive verb would live, since it needs inherited
//! stdio a captured op child can't provide.

use pavo_driver_kit::surface::{OpOutcome, OpSpec, Surface};
use serde_json::{Map, Value};

use crate::ops;

/// cypera's [`Surface`] — engine dispatch, already linked.
pub struct Engine;

impl Surface for Engine {
    /// Same path as the `cli.<verb>` wire query: re-exec `run` in
    /// a child, captured output → `OpOutcome`.
    fn op(&mut self, op: &OpSpec, params: &Map<String, Value>) -> OpOutcome {
        OpOutcome::from_response(&ops::run_op(&op.name, params))
    }

    /// `cypera <name> <args>` in-process — for surfaces (none
    /// today). Bounded ops never reach here.
    fn surface(&mut self, name: &str, args: &[String]) -> i32 {
        // `name` is the dotted op name (`gen.crate-map`) — split it
        // back into argv tokens for the engine's clap tree.
        let argv = std::iter::once("cypera".to_string())
            .chain(name.split('.').map(String::from))
            .chain(args.iter().cloned());
        match cypera::run_from(argv) {
            Ok(code) => code,
            Err(_) => 1,
        }
    }
}
