use assert_cmd::Command;
use predicates::prelude::*;

#[test]
#[ignore = "requires HERDR_LIVE_E2E=1 and a working herdr CLI"]
fn live_herdr_can_see_linked_plugin() {
    if std::env::var("HERDR_LIVE_E2E").ok().as_deref() != Some("1") {
        eprintln!("set HERDR_LIVE_E2E=1 to run the live Herdr plugin smoke test");
        return;
    }

    Command::new("cargo")
        .args(["build", "--release"])
        .assert()
        .success();

    let manifest_dir = std::env::current_dir().unwrap();
    Command::new("herdr")
        .args(["plugin", "link"])
        .arg(&manifest_dir)
        .assert()
        .success();

    Command::new("herdr")
        .args(["plugin", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ramarivera.pretty-which"));

    Command::new("herdr")
        .args(["plugin", "action", "invoke", "ramarivera.pretty-which.open"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
}
