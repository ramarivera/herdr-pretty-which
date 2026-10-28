use super::*;

#[test]
fn pane_cycle_stays_in_caller_tab() {
    let json = serde_json::json!({"result":{"panes":[{"pane_id":"p1","tab_id":"t1"},{"pane_id":"p3","tab_id":"t2"},{"pane_id":"p2","tab_id":"t1"}]}});
    assert_eq!(
        next_pane_id(&json, "t1", "p1", Neighbor::Next).unwrap(),
        Some("p2")
    );
    assert_eq!(
        next_pane_id(&json, "t1", "p1", Neighbor::Prev).unwrap(),
        Some("p2")
    );
    assert!(next_pane_id(&json, "t1", "p3", Neighbor::Next).is_err());
}

#[test]
fn plugin_context_targets_original_pane_and_workspace() {
    let context = r#"{"workspace_id":"w1","tab_id":"w1:t1","focused_pane_id":"w1:p3","focused_pane_cwd":"/caller"}"#;
    let resolve = |kind: &str| {
        let field = match kind {
            "pane" => "focused_pane_id",
            "workspace" => "workspace_id",
            _ => "tab_id",
        };
        context_field_from(context, field).context("missing id")
    };
    let mut zoom = vec_into(&["herdr", "pane", "zoom", "--current", "--toggle"]);
    inject_target_with(&mut zoom, true, resolve).unwrap();
    assert_eq!(
        zoom,
        vec_into(&["herdr", "pane", "zoom", "--toggle", "--pane", "w1:p3"])
    );
    let mut tab = vec_into(&["herdr", "tab", "create", "--focus"]);
    inject_target_with(&mut tab, true, resolve).unwrap();
    assert_eq!(
        tab,
        vec_into(&["herdr", "tab", "create", "--focus", "--workspace", "w1"])
    );
    assert_eq!(
        context_field_from(context, "focused_pane_cwd").as_deref(),
        Some("/caller")
    );
    let mut split = vec_into(&["herdr", "pane", "split"]);
    assert!(inject_target_with(&mut split, true, |_| anyhow::bail!("missing pane")).is_err());
}

#[test]
fn agent_cycles_resolve_underlying_pane_instead_of_overlay() {
    let text = r#"{"result":{"agents":[{"pane_id":"w1:p1","terminal_id":"t1"},{"pane_id":"w1:p2","terminal_id":"t2"}]}}"#;
    assert_eq!(agent_terminal_for_pane(text, "w1:p2").unwrap(), "t2");
    assert!(agent_terminal_for_pane(text, "missing").is_err());
}

#[test]
fn workspace_picker_and_reload_config_have_dispatches() {
    assert!(matches!(
        dispatch_for_action("workspace_picker"),
        Some(Dispatch::WorkspacePicker)
    ));
    assert!(matches!(
        dispatch_for_action("reload_config"),
        Some(Dispatch::Cli(_))
    ));
}

#[test]
fn lists_parse_real_envelopes() {
    let json = r#"{"result":{"workspaces":[{"workspace_id":"w1","label":"one","focused":true}]}}"#;
    assert_eq!(
        extract_entries(json, "workspace").unwrap(),
        vec![("w1".into(), "one".into())]
    );
    assert_eq!(extract_focused_id(json, "workspace").unwrap(), "w1");
}

#[cfg(unix)]
#[test]
fn plugin_socket_preserves_caller_context_and_reports_errors() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        let request: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], "plugin.action.invoke");
        assert_eq!(request["params"]["context"]["focused_pane_id"], "w1:p1");
        stream
            .write_all(b"{\"id\":\"pretty-which\",\"error\":{\"message\":\"denied\"}}\n")
            .unwrap();
    });
    let result = socket_request_at(
        &path,
        "plugin.action.invoke",
        serde_json::json!({"context":{"focused_pane_id":"w1:p1"}}),
    );
    assert!(result.unwrap_err().to_string().contains("denied"));
    server.join().unwrap();
}

#[cfg(unix)]
#[test]
fn fake_herdr_list_reports_source_failures_and_parses_rows() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("herdr");
    std::fs::write(&path, "#!/bin/sh\nif [ \"$1\" = workspace ]; then echo '{\"result\":{\"workspaces\":[{\"workspace_id\":\"w1\",\"label\":\"one\"}]}}'; else echo unavailable >&2; exit 2; fi\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let binary = path.to_str().unwrap();
    assert_eq!(
        list_entries_from(binary, "workspace").unwrap(),
        vec![("w1".into(), "one".into())]
    );
    assert!(list_entries_from(binary, "tab")
        .unwrap_err()
        .to_string()
        .contains("unavailable"));
}

#[cfg(unix)]
#[test]
fn failed_process_reports_error() {
    let result = run(
        &Dispatch::Cli(vec![
            "/bin/sh".into(),
            "-c".into(),
            "echo denied >&2; exit 9".into(),
        ]),
        Path::new("/"),
    );
    assert!(result.unwrap_err().to_string().contains("denied"));
}
