use crate::run_cmd::Cmd;

// TODO: handle legacy login?

pub fn login(account_dir: &str) {
    match account_dir {
        "legacy" => legacy_login(),
        _ => sso_login(account_dir),
    }
}

pub fn legacy_login() {
    let outcome = Cmd::new("eval", ["$(~/proj/simpleinfra/aws-creds.py)"]).run();
    assert!(
        outcome.status().success(),
        "failed to login to legacy account"
    );
}

pub fn sso_login(account_dir: &str) {
    assert_ne!(
        account_dir, "legacy",
        "can't login to legacy account with sso"
    );
    let account = match account_dir {
        "root" => "rust-root",
        account_dir => account_dir,
    };
    let output = Cmd::new("aws", ["sso", "login", "--profile", account]).run();
    assert!(output.status().success());
}

pub fn sso_logout() {
    let output = Cmd::new("aws", ["sso", "logout"]).run();
    assert!(output.status().success());
}
