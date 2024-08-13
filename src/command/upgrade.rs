use camino::{Utf8Path, Utf8PathBuf};
use git_cmd::Repo;
use tracing::debug;

use crate::{
    aws, dir, select,
    terragrunt::{self, PlanOutcome},
};

pub fn upgrade() {
    let repo = repo();
    let git_root = git_root(&repo);
    let tg_accounts = git_root.join("terragrunt").join("accounts");
    let accounts = list_directories_at_path(&tg_accounts);
    let selected_accounts = select::select_accounts(accounts);
    println!("Selected accounts: {:?}", selected_accounts);
    upgrade_accounts(selected_accounts);
}

fn upgrade_accounts(accounts: Vec<Utf8PathBuf>) {
    for account in accounts {
        // logout before login, to avoid issues with multiple profiles
        aws::sso_logout();
        aws::sso_login(account.file_name().unwrap());
        let states = list_directories_at_path(&account);
        let selected_states = select::select_states(states);
        println!("Selected states: {:?}", selected_states);
        for state in selected_states {
            // Update lockfile
            terragrunt::terragrunt_init_upgrade(&state);
            // Verify that there are no changes to apply, to ensure that the state is up-to-date
            assert_eq!(
                terragrunt::are_changes_applied(&state),
                PlanOutcome::NoChanges
            );
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

fn repo() -> Repo {
    let current_dir = dir::current_dir();
    git_cmd::Repo::new(current_dir).unwrap()
}

fn git_root(repo: &Repo) -> camino::Utf8PathBuf {
    let output = repo.git(&["rev-parse", "--show-toplevel"]).unwrap();
    output.into()
}
