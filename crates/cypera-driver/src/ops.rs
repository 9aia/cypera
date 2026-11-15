//! Engine ops — `cli.<verb>` wire queries dispatch into the `cypera`
//! engine lib through this binary's own `run` verb: each call
//! re-execs `tamias-driver-cypera run <argv>` with the request's
//! params mapped to cypera's flags, so `cypera::run` writes to the
//! child's captured stdout and never touches the serve loop's
//! ndjson stream — and every mutation keeps the engine's own
//! semantics (repo resolution, `cypera.toml` discovery, checks).
//!
//! `ops` answers the ops manifest as JSON — *derived from the
//! engine's own clap tree* ([`cypera::command`]), so a verb can't
//! exist on the CLI and be missing here: declared args become
//! typed params (`arg`/`opt`/`flag`), doc strings ride along
//! unchanged.
//!
//! cypera has no unbounded or host-interactive verbs today — every
//! `gen`/`check` call is bounded and stdin-free, so `surfaces` is
//! empty. The classification stays: a future verb that never
//! returns or needs inherited stdio/tty joins `SURFACES` and rides
//! `run` passthrough / generated-surface subcommands instead.
//!
//! `mutates` is honest: `gen.*` verbs write via `--out` (and `gen
//! cli`'s help-walk mode runs `cargo build`, writing `target/`),
//! so they count as mutating; `check` only reads. Streaming *flags*
//! would join `SKIP_PARAMS` — none exist today.

use clap::{Arg, ArgAction, Command};
use pavo_protocol::wire::{QueryResult, Response};
use serde_json::{json, Map, Value};

/// Unbounded/host-interactive surfaces — verbs that never return
/// or need inherited stdio/tty. They can never be request/response
/// ops. Empty today: every cypera verb is bounded.
const SURFACES: &[&str] = &[];

/// Params excluded from ops — flags that turn a bounded verb into
/// a stream, or params that must never ride argv. None today.
const SKIP_PARAMS: &[&str] = &[];

/// Read-only verbs — everything else writes files (`--out`,
/// `cargo build` artifacts). Mutators belong behind a deliberate
/// trigger, not ambient view polling.
const READ_ONLY: &[&str] = &["check"];

/// `(name, clap spec)` for every leaf verb. Hidden plumbing and
/// clap's `help` are skipped — they stay reachable through `run`
/// passthrough.
fn verbs() -> Vec<(String, Command)> {
    fn walk(prefix: &str, cmd: &Command, out: &mut Vec<(String, Command)>) {
        for sub in cmd.get_subcommands() {
            let name = sub.get_name();
            if name == "help" || sub.is_hide_set() {
                continue;
            }
            let dotted = match prefix {
                "" => name.to_string(),
                _ => format!("{prefix}.{name}"),
            };
            if sub.has_subcommands() {
                walk(&dotted, sub, out);
            } else {
                out.push((dotted, sub.clone()));
            }
        }
    }
    let mut out = Vec::new();
    walk("", &cypera::command(), &mut out);
    out
}

fn op_cmd(name: &str) -> Option<Command> {
    verbs()
        .into_iter()
        .find(|(n, _)| n == name && !SURFACES.contains(&n.as_str()))
        .map(|(_, c)| c)
}

/// Args that can't ride the wire: streaming flags (above) and
/// clap's own help/version actions.
fn skipped(a: &Arg) -> bool {
    SKIP_PARAMS.contains(&a.get_id().as_str())
        || matches!(
            a.get_action(),
            ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
        )
}

/// A clap arg → the manifest's typed param kind — `arg` =
/// positional, `opt` = `--name <v>`, `flag` = `--name` — the
/// contract `pavo-driver-kit`'s surface layer renders args from.
fn kind(a: &Arg) -> &'static str {
    if a.is_positional() {
        "arg"
    } else if matches!(
        a.get_action(),
        ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count
    ) {
        "flag"
    } else {
        "opt"
    }
}

fn param_json(a: &Arg) -> Value {
    // Wire name = the flag name (`--force` → `force`), not the
    // field id — generated surfaces spell params `--<name>`.
    let id = a.get_id().as_str();
    let name = match a.is_positional() {
        true => id,
        false => a.get_long().unwrap_or(id),
    };
    json!({
        "name": name,
        "kind": kind(a),
        "doc": a.get_help().map(|d| d.to_string()).unwrap_or_default(),
    })
}

/// The `ops` query/`ops` argv verb payload — self-describing so
/// hosts and agents discover the engine surface without argv help.
/// cypera has no globals — `--repo`/`--config` are declared
/// per-verb and ride the normal param mapping.
pub fn manifest() -> Value {
    json!({
        "engine": "cypera",
        "invoke": "query `cli.<verb>` with the listed params, or argv `tamias-driver-cypera run <cypera argv>`",
        "globals": [],
        "ops": verbs().iter()
            .filter(|(n, _)| !SURFACES.contains(&n.as_str()))
            .map(|(n, c)| json!({
                "query": format!("cli.{n}"),
                "doc": c.get_about().map(|d| d.to_string()).unwrap_or_default(),
                "params": c.get_arguments()
                    .filter(|a| !skipped(a))
                    .map(param_json)
                    .collect::<Vec<_>>(),
                "mutates": !READ_ONLY.contains(&n.as_str()),
            }))
            .collect::<Vec<_>>(),
        "surfaces": SURFACES,
    })
}

/// Is `query` an engine op (`cli.<verb>`)? Returns the verb name.
pub fn op_of(query: &str) -> Option<String> {
    query
        .strip_prefix("cli.")
        .filter(|n| op_cmd(n).is_some())
        .map(str::to_string)
}

/// A wire scalar → its argv token (numbers/bools stringify).
fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// All values of a param — arrays flatten, scalars wrap.
fn values(v: &Value) -> Vec<String> {
    match v {
        Value::Array(xs) => xs.iter().filter_map(scalar).collect(),
        other => scalar(other).into_iter().collect(),
    }
}

/// `cli.<verb>` params → cypera argv (no argv[0] — `run` prepends
/// it). Declared params map by kind — positionals land last, in
/// schema order; repeatable opts (`--layer`, `--file`, `--only`,
/// `--prefix`) flatten array values into repeated flags.
fn argv(name: &str, cmd: &Command, params: &Map<String, Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    out.extend(name.split('.').map(String::from));
    for a in cmd.get_arguments() {
        if skipped(a) || a.is_positional() {
            continue;
        }
        let id = a.get_id().as_str();
        let long = a.get_long().unwrap_or(id);
        let raw = params
            .get(long)
            .or_else(|| params.get(id))
            .or_else(|| params.get(id.replace('_', "-").as_str()));
        let Some(raw) = raw else { continue };
        match kind(a) {
            "flag" => {
                if raw.as_bool().unwrap_or(false) {
                    out.push(format!("--{long}"));
                }
            }
            _ => {
                for s in values(raw) {
                    out.push(format!("--{long}"));
                    out.push(s);
                }
            }
        }
    }
    // `trailing_var_arg` positionals only parse after a `--`
    // separator — kept generic for any future last-positional.
    if cmd.get_positionals().any(|a| a.is_last_set()) {
        out.push("--".into());
    }
    let mut positionals: Vec<String> = Vec::new();
    for a in cmd.get_positionals() {
        if skipped(a) {
            continue;
        }
        let id = a.get_id().as_str();
        if let Some(raw) = params
            .get(id)
            .or_else(|| params.get(id.replace('_', "-").as_str()))
        {
            positionals.extend(values(raw));
        }
    }
    out.extend(positionals);
    out
}

/// The bare-name entry the embedded surfaces use (`OpSpec.name` is
/// `cli.`-stripped); the wire path goes `op_of` → `run_verb`.
#[cfg(feature = "mcp")]
pub fn run_op(op: &str, params: &Map<String, Value>) -> Response {
    match op_of(&format!("cli.{}", op.strip_prefix("cli.").unwrap_or(op))) {
        Some(name) => run_verb(&name, params),
        None => Response::err(format!("unknown op: {op}")),
    }
}

/// Run one engine op by name: re-exec `run` in a child with
/// captured output, and hand back `{op, argv, code, stdout,
/// stderr}` as the query payload. `ok:false` is reserved for spawn
/// failures — a nonzero `code` is the engine's own exit status
/// (e.g. `check` findings), honest data. The child's stdin is
/// null and it inherits this process's cwd — `--repo`/`--config`
/// params (or upward `cypera.toml` discovery) pick the target repo.
pub fn run_verb(name: &str, params: &Map<String, Value>) -> Response {
    let Some(cmd) = op_cmd(name) else {
        return Response::err(format!("unknown op: {name}"));
    };
    let args = argv(name, &cmd, params);
    let out = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .arg("run")
            .args(&args)
            .output()
    });
    let data = match out {
        Ok(o) => json!({
            "op": name,
            "argv": args,
            "code": o.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&o.stdout),
            "stderr": String::from_utf8_lossy(&o.stderr),
        }),
        Err(e) => return Response::err(format!("run {name}: {e}")),
    };
    Response::ok_typed(&QueryResult {
        query: format!("cli.{name}"),
        data,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn op_of_accepts_cli_prefix_only() {
        assert_eq!(op_of("cli.check").unwrap(), "check");
        assert_eq!(op_of("cli.gen.crate-map").unwrap(), "gen.crate-map");
        assert_eq!(op_of("cli.gen.config").unwrap(), "gen.config");
        assert!(op_of("gen").is_none()); // group, not a leaf
        assert!(op_of("check").is_none()); // view query, not an op
        assert!(op_of("cli.bogus").is_none());
    }

    #[test]
    fn manifest_lists_every_verb_as_cli_query() {
        let m = manifest();
        let ops = m["ops"].as_array().unwrap();
        for (n, _) in verbs() {
            assert!(
                ops.iter().any(|o| o["query"] == json!(format!("cli.{n}"))),
                "{n} missing from ops manifest"
            );
        }
        assert_eq!(m["surfaces"], json!(SURFACES));
        for o in ops {
            for p in o["params"].as_array().unwrap() {
                assert!(["arg", "opt", "flag"].contains(&p["kind"].as_str().unwrap()));
                // streaming/stdin params never ride the wire
                assert!(!SKIP_PARAMS.contains(&p["name"].as_str().unwrap()));
            }
        }
        // mutates honesty — only `check` is read-only
        let read_only: Vec<&str> = ops
            .iter()
            .filter(|o| o["mutates"] == json!(false))
            .filter_map(|o| o["query"].as_str())
            .collect();
        assert_eq!(read_only, vec!["cli.check"]);
    }

    #[test]
    fn argv_splits_dotted_names_and_maps_flags() {
        let cmd = op_cmd("gen.crate-map").unwrap();
        let mut p = Map::new();
        p.insert("repo".into(), json!("/repo"));
        p.insert("layer".into(), json!(["cypera-driver=driver"]));
        assert_eq!(
            argv("gen.crate-map", &cmd, &p),
            vec![
                "gen",
                "crate-map",
                "--repo",
                "/repo",
                "--layer",
                "cypera-driver=driver"
            ]
        );
    }

    #[test]
    fn argv_check_maps_json_flag() {
        let cmd = op_cmd("check").unwrap();
        let mut p = Map::new();
        p.insert("repo".into(), json!("."));
        p.insert("json".into(), json!(true));
        assert_eq!(
            argv("check", &cmd, &p),
            vec!["check", "--repo", ".", "--json"]
        );
    }
}
