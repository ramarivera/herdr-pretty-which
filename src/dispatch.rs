//! Dispatch supported Herdr actions through its CLI.
//! Unsupported client-only actions remain reference-only.

use crate::launcher::items::Dispatch;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Map a Herdr keybinding action name to its dispatch path, or `None` if the
/// action has no implemented CLI equivalent.
///
/// Action names come from `crate::model::SPECS` — they're the
/// canonical Herdr action ids (e.g. `new_workspace`, `focus_pane_left`).
pub fn dispatch_for_action(action: &str) -> Option<Dispatch> {
    use Dispatch::*;
    let d = match action {
        // --- Workspaces ---
        "workspace_picker" | "switch_workspace" => WorkspacePicker,
        "switch_tab" => TabPicker,
        "focus_agent" => AgentPicker,
        "edit_scrollback" => EditScrollback,
        "cycle_pane_next" => CyclePaneNext,
        "cycle_pane_previous" => CyclePanePrevious,
        "rename_workspace" => Target {
            kind: "workspace".into(),
            verb: "rename".into(),
        },
        "rename_tab" => Target {
            kind: "tab".into(),
            verb: "rename".into(),
        },
        "rename_pane" => Target {
            kind: "pane".into(),
            verb: "rename".into(),
        },
        "close_workspace" => Target {
            kind: "workspace".into(),
            verb: "close".into(),
        },
        "close_tab" => Target {
            kind: "tab".into(),
            verb: "close".into(),
        },
        "close_pane" => Target {
            kind: "pane".into(),
            verb: "close".into(),
        },
        "reload_config" => Cli(vec_into(&["herdr", "server", "reload-config"])),
        "new_workspace" => Cli(vec_into(&["herdr", "workspace", "create", "--focus"])),
        "new_worktree" => Cli(vec_into(&["herdr", "worktree", "create", "--focus"])),
        "previous_workspace" => PrevWorkspace,
        "next_workspace" => NextWorkspace,

        // --- Tabs ---
        "new_tab" => Cli(vec_into(&["herdr", "tab", "create", "--focus"])),
        "previous_tab" => PrevTab,
        "next_tab" => NextTab,

        // --- Panes / agents ---
        "split_vertical" => Cli(vec_into(&[
            "herdr",
            "pane",
            "split",
            "--direction",
            "right",
            "--focus",
        ])),
        "split_horizontal" => Cli(vec_into(&[
            "herdr",
            "pane",
            "split",
            "--direction",
            "down",
            "--focus",
        ])),
        "zoom" | "fullscreen" => Cli(vec_into(&[
            "herdr",
            "pane",
            "zoom",
            "--current",
            "--toggle",
        ])),
        "focus_pane_left" => Cli(vec_into(&["herdr", "pane", "focus", "--direction", "left"])),
        "focus_pane_down" => Cli(vec_into(&["herdr", "pane", "focus", "--direction", "down"])),
        "focus_pane_up" => Cli(vec_into(&["herdr", "pane", "focus", "--direction", "up"])),
        "focus_pane_right" => Cli(vec_into(&[
            "herdr",
            "pane",
            "focus",
            "--direction",
            "right",
        ])),
        "previous_agent" => PrevAgent,
        "next_agent" => NextAgent,

        _ => return None,
    };
    Some(d)
}

/// Query Herdr for the cwd of the current caller pane. This is the directory
/// the palette should use as the working directory for shell commands and for
/// newly created panes/tabs/workspaces/plugin panes.
///
/// Herdr's global focused pane can move while a plugin is opening; prefer the
/// process/caller-aware `pane current` result, then fall back to scanning the
/// focused pane list for older or degraded Herdr contexts.
pub fn focused_pane_cwd() -> Option<PathBuf> {
    context_field("focused_pane_cwd")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HERDR_ACTIVE_PANE_CWD")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
        })
        .or_else(current_pane_cwd)
        .or_else(focused_pane_cwd_from_list)
}

fn current_pane_cwd() -> Option<PathBuf> {
    let output = Command::new(herdr_bin().ok()?)
        .args(["pane", "current"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    extract_current_pane_cwd(&text)
}

fn extract_current_pane_cwd(text: &str) -> Option<PathBuf> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    v.get("result")
        .and_then(|r| r.get("pane"))
        .or_else(|| v.get("pane"))
        .and_then(cwd_from_pane_entry)
}

fn focused_pane_cwd_from_list() -> Option<PathBuf> {
    let output = Command::new(herdr_bin().ok()?)
        .args(["pane", "list"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let arr = list_array_for_kind(&v, "pane")?;
    arr.iter()
        .find(|entry| entry.get("focused").and_then(|f| f.as_bool()) == Some(true))
        .and_then(cwd_from_pane_entry)
}

fn cwd_from_pane_entry(entry: &serde_json::Value) -> Option<PathBuf> {
    ["foreground_cwd", "cwd"].iter().find_map(|key| {
        entry
            .get(*key)
            .and_then(|c| c.as_str())
            .filter(|c| !c.is_empty())
            .map(PathBuf::from)
    })
}

/// For surface-creating `herdr` subcommands, inject an explicit `--cwd` flag
/// so the new pane/workspace/tab starts in the focused pane's directory
/// rather than inheriting the palette process's cwd.
fn inject_cwd_for_creation(argv: &mut Vec<String>, cwd: &Path) {
    if argv.iter().any(|s| s == "--cwd") {
        return;
    }
    let program = argv.first().map(|s| s.as_str());
    let kind = argv.get(1).map(|s| s.as_str());
    let sub = argv.get(2).map(|s| s.as_str());
    let third = argv.get(3).map(|s| s.as_str());
    let needs_cwd = program == Some("herdr")
        && (matches!(
            (kind, sub),
            (Some("pane"), Some("split"))
                | (Some("workspace"), Some("create"))
                | (Some("tab"), Some("create"))
                | (Some("worktree"), Some("create"))
        ) || matches!(
            (kind, sub, third),
            (Some("plugin"), Some("pane"), Some("open"))
        ));
    if needs_cwd {
        argv.push("--cwd".to_string());
        argv.push(cwd.to_string_lossy().to_string());
    }
}

/// Resolve the `herdr` binary path. `HERDR_BIN_PATH` wins, else PATH lookup.
pub fn herdr_bin() -> Result<String> {
    if let Ok(p) = std::env::var("HERDR_BIN_PATH") {
        if !p.is_empty() {
            return Ok(p);
        }
    }
    which("herdr").context("could not find `herdr` on PATH (set HERDR_BIN_PATH?)")
}

fn which(cmd: &str) -> Result<String, std::io::Error> {
    // Lightweight PATH lookup; avoids pulling in the `which` crate for one call.
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("{cmd} not found on PATH"),
    ))
}

/// Execute a [`Dispatch`] using `cwd` as the working directory for the child
/// process. New pane/workspace/tab creation commands also receive an explicit
/// `--cwd` flag so they inherit the focused pane's directory. The palette
/// closes immediately after so Herdr retains focus. Errors are surfaced but
/// do not panic.
pub fn run(dispatch: &Dispatch, cwd: &Path) -> Result<()> {
    match dispatch {
        Dispatch::ShellCommand(command) => {
            use std::process::Stdio;
            let mut child = Command::new("/bin/sh")
                .args(["-lc", command])
                .current_dir(cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Dispatch::WorkspacePicker
        | Dispatch::TabPicker
        | Dispatch::AgentPicker
        | Dispatch::Target { .. } => {
            anyhow::bail!("picker requires launcher UI")
        }
        Dispatch::Cli(argv) => {
            if argv.get(1).map(String::as_str) == Some("plugin")
                && argv.get(2).map(String::as_str) == Some("action")
                && argv.get(3).map(String::as_str) == Some("invoke")
            {
                if let Ok(context) = std::env::var("HERDR_PLUGIN_CONTEXT_JSON") {
                    let context: serde_json::Value =
                        serde_json::from_str(&context).context("invalid plugin context")?;
                    let action = argv.get(4).context("missing plugin action id")?;
                    let plugin = argv
                        .windows(2)
                        .find(|pair| pair[0] == "--plugin")
                        .map(|pair| pair[1].as_str());
                    return socket_request(
                        "plugin.action.invoke",
                        serde_json::json!({"action_id": action, "plugin_id": plugin, "context": context}),
                    );
                }
            }
            let mut owned: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
            inject_cwd_for_creation(&mut owned, cwd);
            inject_caller_target(&mut owned)?;
            let strs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            run_argv(&strs, cwd)?;
        }
        Dispatch::FocusWorkspace(id) => {
            run_argv(&["herdr", "workspace", "focus", id], cwd)?;
        }
        Dispatch::FocusTab(id) => {
            run_argv(&["herdr", "tab", "focus", id], cwd)?;
        }
        Dispatch::FocusAgent(target) => {
            run_argv(&["herdr", "agent", "focus", target], cwd)?;
        }
        Dispatch::EditScrollback => socket_request(
            "pane.edit_scrollback",
            serde_json::json!({"pane_id": caller_target("pane")?}),
        )?,
        Dispatch::CyclePaneNext => cycle_pane(Neighbor::Next)?,
        Dispatch::CyclePanePrevious => cycle_pane(Neighbor::Prev)?,
        Dispatch::NextWorkspace => {
            focus_neighbor("workspace", Neighbor::Next, cwd)?;
        }
        Dispatch::PrevWorkspace => {
            focus_neighbor("workspace", Neighbor::Prev, cwd)?;
        }
        Dispatch::NextTab => {
            focus_neighbor("tab", Neighbor::Next, cwd)?;
        }
        Dispatch::PrevTab => {
            focus_neighbor("tab", Neighbor::Prev, cwd)?;
        }
        Dispatch::NextAgent => {
            focus_neighbor("agent", Neighbor::Next, cwd)?;
        }
        Dispatch::PrevAgent => {
            focus_neighbor("agent", Neighbor::Prev, cwd)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Neighbor {
    Next,
    Prev,
}

fn cycle_pane(direction: Neighbor) -> Result<()> {
    let workspace = caller_target("workspace")?;
    let tab = caller_target("tab")?;
    let pane = caller_target("pane")?;
    let out = Command::new(herdr_bin()?)
        .args(["pane", "list", "--workspace", &workspace])
        .output()?;
    if !out.status.success() {
        anyhow::bail!(
            "herdr pane list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let next = next_pane_id(&json, &tab, &pane, direction)?;
    let Some(next) = next else {
        return Ok(());
    };
    socket_request("pane.focus", serde_json::json!({"pane_id": next}))
}

fn next_pane_id<'a>(
    json: &'a serde_json::Value,
    tab: &str,
    pane: &str,
    direction: Neighbor,
) -> Result<Option<&'a str>> {
    let panes: Vec<&str> = list_array_for_kind(json, "pane")
        .context("pane list had no array")?
        .iter()
        .filter(|p| p.get("tab_id").and_then(|v| v.as_str()) == Some(tab))
        .filter_map(|p| p.get("pane_id").and_then(|v| v.as_str()))
        .collect();
    let current = panes
        .iter()
        .position(|id| *id == pane)
        .context("caller pane is not in its tab")?;
    if panes.len() < 2 {
        return Ok(None);
    }
    let next = match direction {
        Neighbor::Next => (current + 1) % panes.len(),
        Neighbor::Prev => (current + panes.len() - 1) % panes.len(),
    };
    Ok(Some(panes[next]))
}

/// Resolve the live ordered list of `<kind>` ids, find the current one, and
/// focus its neighbor. `<kind>` ∈ {workspace, tab, agent}. For agents, "current"
/// is the focused terminal; for workspaces/tabs it's the focused entity.
fn focus_neighbor(kind: &str, neighbor: Neighbor, cwd: &Path) -> Result<()> {
    let mut command = Command::new(herdr_bin()?);
    command.args([kind, "list"]);
    if kind == "tab" {
        command.args(["--workspace", &caller_target("workspace")?]);
    }
    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "herdr {kind} list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let ids: Vec<String> = extract_entries(&text, kind)?
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let current = if kind == "agent" && std::env::var_os("HERDR_PLUGIN_CONTEXT_JSON").is_some() {
        let pane = caller_target("pane")?;
        agent_terminal_for_pane(&text, &pane).ok()
    } else if kind != "agent" {
        Some(caller_target(kind)?)
    } else {
        extract_focused_id(&text, kind).ok()
    };
    let pos = current
        .as_ref()
        .and_then(|id| ids.iter().position(|candidate| candidate == id));
    let target = match (pos, neighbor) {
        (Some(pos), Neighbor::Next) => (pos + 1) % ids.len(),
        (Some(pos), Neighbor::Prev) => (pos + ids.len() - 1) % ids.len(),
        (None, Neighbor::Next) if kind == "agent" => 0,
        (None, Neighbor::Prev) if kind == "agent" => ids.len() - 1,
        _ => anyhow::bail!("caller isn't present in the current list"),
    };
    let id = &ids[target];
    run_argv(&["herdr", kind, "focus", id], cwd)
}

fn agent_terminal_for_pane(text: &str, pane_id: &str) -> Result<String> {
    let json: serde_json::Value = serde_json::from_str(text)?;
    list_array_for_kind(&json, "agent")
        .context("agent list had no array")?
        .iter()
        .find(|entry| entry.get("pane_id").and_then(|v| v.as_str()) == Some(pane_id))
        .and_then(|entry| entry.get("terminal_id"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .context("caller pane has no listed agent")
}

#[cfg(unix)]
fn socket_request(method: &str, params: serde_json::Value) -> Result<()> {
    let path = std::env::var_os("HERDR_SOCKET_PATH")
        .context("Herdr socket path is missing; cannot preserve plugin caller context")?;
    socket_request_at(Path::new(&path), method, params)
}
#[cfg(unix)]
fn socket_request_at(path: &Path, method: &str, params: serde_json::Value) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;
    let mut socket = UnixStream::connect(path).context("cannot connect to Herdr socket")?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    let request = serde_json::json!({"id":"pretty-which", "method":method, "params":params});
    serde_json::to_writer(&mut socket, &request)?;
    socket.write_all(b"\n")?;
    let mut response = String::new();
    BufReader::new(socket).read_line(&mut response)?;
    let response: serde_json::Value =
        serde_json::from_str(&response).context("invalid Herdr response")?;
    if let Some(error) = response.get("error") {
        anyhow::bail!(
            "Herdr {method} failed: {}",
            error
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
        );
    }
    if response.get("result").is_none() {
        anyhow::bail!("Herdr {method} response has no result");
    }
    Ok(())
}
#[cfg(not(unix))]
fn socket_request(_method: &str, _params: serde_json::Value) -> Result<()> {
    anyhow::bail!("preserving plugin caller context requires the Herdr local socket")
}

fn context_field_from(context: &str, field: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(context)
        .ok()?
        .get(field)?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
fn context_field(field: &str) -> Option<String> {
    std::env::var("HERDR_PLUGIN_CONTEXT_JSON")
        .ok()
        .and_then(|context| context_field_from(&context, field))
}

fn inject_caller_target(argv: &mut Vec<String>) -> Result<()> {
    let active = std::env::var_os("HERDR_PLUGIN_CONTEXT_JSON").is_some()
        || std::env::var_os("HERDR_ACTIVE_PANE_ID").is_some();
    inject_target_with(argv, active, caller_target)
}
fn inject_target_with(
    argv: &mut Vec<String>,
    active: bool,
    resolve: impl Fn(&str) -> Result<String>,
) -> Result<()> {
    if argv.first().map(String::as_str) != Some("herdr") {
        return Ok(());
    }
    let command = (
        argv.get(1).map(String::as_str),
        argv.get(2).map(String::as_str),
    );
    let (kind, flag) = match command {
        (Some("pane"), Some("split" | "focus" | "zoom")) => ("pane", "--pane"),
        (Some("tab"), Some("create")) | (Some("worktree"), Some("create")) => {
            ("workspace", "--workspace")
        }
        _ => return Ok(()),
    };
    if argv.iter().any(|s| s == flag) {
        return Ok(());
    }
    if active {
        let id = resolve(kind)?;
        if kind == "pane" {
            argv.retain(|arg| arg != "--current");
        }
        argv.push(flag.into());
        argv.push(id);
    }
    Ok(())
}

/// Resolve caller target using the context supplied by Herdr when opening a plugin.
pub fn caller_target(kind: &str) -> Result<String> {
    let field = match kind {
        "workspace" => "workspace_id",
        "tab" => "tab_id",
        "pane" => "focused_pane_id",
        _ => anyhow::bail!("unsupported target {kind}"),
    };
    if std::env::var_os("HERDR_PLUGIN_CONTEXT_JSON").is_some() {
        return context_field(field)
            .with_context(|| format!("plugin context has no caller {kind} id"));
    }
    let key = match kind {
        "workspace" => "HERDR_ACTIVE_WORKSPACE_ID",
        "tab" => "HERDR_ACTIVE_TAB_ID",
        "pane" => "HERDR_ACTIVE_PANE_ID",
        _ => anyhow::bail!("unsupported target {kind}"),
    };
    if let Ok(id) = std::env::var(key) {
        if !id.is_empty() {
            return Ok(id);
        }
    }
    if kind != "pane" {
        return current_id(kind);
    }
    if let Ok(id) = std::env::var("HERDR_PANE_ID") {
        if !id.is_empty() {
            return Ok(id);
        }
    }
    let out = Command::new(herdr_bin()?)
        .args(["pane", "current"])
        .output()?;
    if !out.status.success() {
        anyhow::bail!(
            "cannot find current pane: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    json.pointer("/result/pane/pane_id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .context("current pane response has no pane id")
}

pub fn run_target(kind: &str, verb: &str, id: &str, label: Option<&str>, cwd: &Path) -> Result<()> {
    let mut args = vec!["herdr", kind, verb, id];
    if let Some(label) = label {
        args.push(label);
    }
    run_argv(&args, cwd)
}

/// `herdr <kind> list` → ordered vector of (id, label). JSON is the default
/// output for all herdr list commands (no `--json` flag exists). Public so the
/// source collector can build jump targets from the same resolver the
/// prev/next dispatch path uses.
pub fn list_entries(kind: &str) -> Result<Vec<(String, String)>> {
    list_entries_from(&herdr_bin()?, kind)
}
fn list_entries_from(binary: &str, kind: &str) -> Result<Vec<(String, String)>> {
    let out = Command::new(binary).args([kind, "list"]).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "herdr {kind} list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    extract_entries(&text, kind).context("could not parse entries from list output")
}

/// Resolve the focused entity without guessing when the response lacks one.
fn current_id(kind: &str) -> Result<String> {
    let out = Command::new(herdr_bin()?).args([kind, "list"]).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "herdr {kind} list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    extract_focused_id(&text, kind).context("could not determine focused id from list output")
}

/// Extract ordered (id, label) entries from a herdr JSON list envelope.
/// Id field is kind-specific: `workspace_id`, `tab_id`, or `terminal_id`.
/// Label is `label` (workspaces/tabs); for agents we synthesize
/// `<agent> · <cwd basename>` since agents have no `label` field.
fn extract_entries(text: &str, kind: &str) -> Result<Vec<(String, String)>> {
    let id_field = id_field_for_kind(kind);
    let v: serde_json::Value = serde_json::from_str(text).context("list output was not JSON")?;
    let arr = list_array_for_kind(&v, kind).context("list output had no array")?;
    let mut out = Vec::with_capacity(arr.len());
    for entry in arr {
        let id = entry
            .get(id_field)
            .and_then(|i| i.as_str())
            .map(str::to_string);
        let label = match kind {
            "agent" => {
                let agent = entry
                    .get("agent")
                    .and_then(|s| s.as_str())
                    .unwrap_or("agent");
                let cwd = entry
                    .get("cwd")
                    .and_then(|s| s.as_str())
                    .map(|c| {
                        std::path::Path::new(c)
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_else(|| c.to_string())
                    })
                    .unwrap_or_default();
                format!("{agent} · {cwd}")
            }
            _ => entry
                .get("label")
                .and_then(|s| s.as_str())
                .map(str::to_string)
                .unwrap_or_default(),
        };
        if let Some(id) = id {
            out.push((id, label));
        }
    }
    Ok(out)
}

fn extract_focused_id(text: &str, kind: &str) -> Result<String> {
    let id_field = id_field_for_kind(kind);
    let v: serde_json::Value = serde_json::from_str(text).context("list output was not JSON")?;
    let arr = list_array_for_kind(&v, kind).context("list output had no array")?;
    arr.iter()
        .find(|entry| entry.get("focused").and_then(|focused| focused.as_bool()) == Some(true))
        .and_then(|entry| entry.get(id_field).and_then(|id| id.as_str()))
        .map(str::to_string)
        .context("list output had no id")
}

fn list_array_for_kind<'a>(
    v: &'a serde_json::Value,
    kind: &str,
) -> Option<&'a Vec<serde_json::Value>> {
    let plural = match kind {
        "workspace" => "workspaces",
        "tab" => "tabs",
        "agent" => "agents",
        "pane" => "panes",
        other => other,
    };
    v.get("result")
        .and_then(|r| r.get(plural))
        .and_then(|w| w.as_array())
        .or_else(|| v.as_array())
}

fn id_field_for_kind(kind: &str) -> &'static str {
    match kind {
        "workspace" => "workspace_id",
        "tab" => "tab_id",
        "agent" => "terminal_id",
        _ => "id",
    }
}

/// Run an argv, resolving `argv[0] == "herdr"` to the real binary path. String
/// slices are promoted to owned for the child. The child runs with `cwd` as
/// its working directory.
fn run_argv(argv: &[&str], cwd: &Path) -> Result<()> {
    let mut owned: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    if owned.first().is_some_and(|first| first == "herdr") {
        owned[0] = herdr_bin()?;
    }
    let (cmd, args) = owned.split_first().context("empty argv")?;
    let output = Command::new(cmd).args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "{} failed: {}",
            cmd,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn vec_into(slice: &[&str]) -> Vec<String> {
    slice.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests;
