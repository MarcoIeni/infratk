use std::collections::BTreeMap;

use camino::{Utf8Component, Utf8PathBuf};
use inquire::{list_option::ListOption, validator::Validation, MultiSelect};

use crate::{
    cmd_runner::CmdRunner,
    dir::{current_dir, current_dir_is_simpleinfra},
    LOCKFILE,
};

pub fn upgrade_provider() {
    assert!(current_dir_is_simpleinfra());
    let lockfiles = get_all_lockfiles();
    let providers = get_all_providers(&lockfiles);
    let providers_list = providers.keys().cloned().collect();
    let selected_providers = select_providers(providers_list);

    update_lockfiles(&providers, selected_providers);
}

fn update_lockfiles(
    providers: &BTreeMap<String, Vec<Utf8PathBuf>>,
    selected_providers: Vec<String>,
) {
    for p in selected_providers {
        if let Some(lockfiles) = providers.get(&p) {
            for l in lockfiles {
                let directory = l.parent().unwrap();
                let is_terraform = directory
                    .components()
                    .any(|c| c == Utf8Component::Normal("terraform"));
                let cmd_runner = CmdRunner::new(BTreeMap::new());
                // TODO: login to AWS
                if is_terraform {
                    cmd_runner.terraform_init_upgrade(directory)
                } else {
                    cmd_runner.terragrunt_init_upgrade(directory)

                }
            }
        }
    }
}

pub fn select_providers(providers: Vec<String>) -> Vec<String> {
    let selected = MultiSelect::new("Select one or more providers:", providers)
        .with_validator(|selected: &[ListOption<&String>]| {
            if selected.is_empty() {
                Ok(Validation::Invalid("Select one item!".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .unwrap_or_else(|e| panic!("failed to select providers: {e:?}"));

    selected.into_iter().collect()
}

/// Get all providers from all lockfiles.
/// The result is a map where the key is the provider name and the value is the
/// list of lockfiles that use that provider.
pub fn get_all_providers(lockfiles: &[Utf8PathBuf]) -> BTreeMap<String, Vec<Utf8PathBuf>> {
    let mut providers = BTreeMap::new();
    for lockfile in lockfiles {
        let content = std::fs::read_to_string(lockfile).expect("could not read lockfile");
        let lines = content.lines();
        for line in lines {
            if line.starts_with("provider") {
                let provider = line.split_whitespace().nth(1).unwrap().trim_matches('"');
                providers
                    .entry(provider.to_string())
                    .or_insert_with(Vec::new)
                    .push(lockfile.clone());
            }
        }
    }
    providers
}

fn get_all_lockfiles() -> Vec<Utf8PathBuf> {
    let mut lockfiles = vec![];
    let current_dir = current_dir();
    let walker = ignore::WalkBuilder::new(current_dir)
        // Read hidden files
        .hidden(false)
        .build();
    for entry in walker {
        let entry = entry.expect("invalid entry");
        let file_type = entry.file_type().expect("unknown file type");
        if !file_type.is_dir() && entry.file_name() == LOCKFILE {
            let path = entry.path().to_path_buf();
            let utf8path = Utf8PathBuf::from_path_buf(path).unwrap();
            lockfiles.push(utf8path);
        }
    }
    lockfiles
}
