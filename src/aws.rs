use std::collections::BTreeMap;

use secrecy::SecretString;

use crate::cmd::Cmd;

/// Returns a map of environment variables that you need to use to authenticate with the account.
#[must_use]
pub fn login(account_dir: &str) -> BTreeMap<String, SecretString> {
    match account_dir {
        "legacy" => legacy_login(),
        _ => {
            sso_login(account_dir);
            BTreeMap::new()
        }
    }
}

/// Returns a map of environment variables that can be used to authenticate with the legacy account.
pub fn legacy_login() -> BTreeMap<String, SecretString> {
    let mut env_vars = BTreeMap::new();
    let outcome = Cmd::new("python3", ["./aws-creds.py"]).run();
    assert!(
        outcome.status().success(),
        "failed to login to legacy account"
    );
    for line in outcome.stdout().lines() {
        if line.contains("export") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let key = parts[1].split('=').next().unwrap();
            let value = parts[1].split('=').last().unwrap();
            env_vars.insert(key.to_string(), SecretString::new(value.to_string()));
        }
    }
    env_vars
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
