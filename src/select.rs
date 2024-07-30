use camino::Utf8PathBuf;
use inquire::{list_option::ListOption, validator::Validation, MultiSelect};

pub fn select_accounts(accounts: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    select_paths(accounts, "account")
}

pub fn select_states(states: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    select_paths(states, "state")
}

fn select_paths(paths: Vec<Utf8PathBuf>, resource_name: &str) -> Vec<Utf8PathBuf> {
    let selected = MultiSelect::new(&format!("Select one or more {resource_name}s:"), paths)
        .with_validator(|selected: &[ListOption<&Utf8PathBuf>]| {
            if selected.is_empty() {
                Ok(Validation::Invalid("Select one item!".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .unwrap_or_else(|e| panic!("failed to select {resource_name}: {e:?}"));

    selected
}
