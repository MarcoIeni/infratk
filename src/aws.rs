use crate::run_cmd::Cmd;

// TODO: handle legacy login?

pub fn sso_login(account_dir: &str) {
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
