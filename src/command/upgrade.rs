use camino::{Utf8Path, Utf8PathBuf};
use tracing::debug;

use crate::{
    aws,
    cmd_runner::{CmdRunner, PlanOutcome},
    config::Config,
    git, select,
};

pub fn upgrade(config: &Config) {
    let repo = git::repo();
    let git_root = git::git_root(&repo);
    let tg_accounts = git_root.join("terragrunt").join("accounts");
    let accounts = list_directories_at_path(&tg_accounts);
    let selected_accounts = select::select_accounts(accounts);
    println!("Selected accounts: {:?}", selected_accounts);
    upgrade_accounts(selected_accounts, config);
}

fn upgrade_accounts(accounts: Vec<Utf8PathBuf>, config: &Config) {
    for account in accounts {
        // logout before login, to avoid issues with multiple profiles
        aws::sso_logout();
        let env_vars = aws::login(account.file_name().unwrap(), config);
        let cmd_runner = CmdRunner::new(env_vars);
        let states = list_directories_at_path(&account);
        let selected_states = select::select_states(states);
        println!("Selected states: {:?}", selected_states);
        for state in selected_states {
            // Update lockfile
            cmd_runner.terragrunt_init_upgrade(&state);
            // Verify that there are no changes to apply, to ensure that the state is up-to-date
            assert_eq!(cmd_runner.terragrunt_plan(&state), PlanOutcome::NoChanges);
        }
    }
}

fn list_directories_at_path(path: &Utf8Path) -> Vec<Utf8PathBuf> {
    debug!("Listing directories at path: {:?}", path);
    let mut children_dirs = vec![];
    let dir = path.read_dir().unwrap();
    for entry in dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let utf8_path = Utf8PathBuf::from_path_buf(path).unwrap();
            children_dirs.push(utf8_path);
        }
    }
    children_dirs
}
