use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use inquire::{list_option::ListOption, validator::Validation, MultiSelect};

use crate::{
    aws,
    cmd_runner::CmdRunner,
    dir::{current_dir, current_dir_is_simpleinfra},
    grouped_dirs, LOCKFILE,
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
    // Filter out the providers that were not selected
    let providers = providers
        .iter()
        .filter(|(k, _)| selected_providers.contains(k));
    let all_dirs: Vec<&Utf8Path> = providers
        .flat_map(|(_, v)| v.iter())
        .map(|l| l.parent().unwrap())
        .collect();
    let grouped_dirs = grouped_dirs::GroupedDirs::new(all_dirs);

    if grouped_dirs.contains_legacy_account() {
        let legacy_tg_dirs = grouped_dirs.legacy_terragrunt_dirs();
        upgrade_legacy_dirs(grouped_dirs.terraform_dirs(), legacy_tg_dirs);
    }

    let sso_terragrunt_dirs = grouped_dirs.sso_terragrunt_dirs();
    upgrade_terragrunt_with_sso(&sso_terragrunt_dirs);
}

fn upgrade_legacy_dirs<T, U>(terraform_dirs: Vec<T>, terragrunt_dirs: Vec<U>)
where
    T: AsRef<Utf8Path>,
    U: AsRef<Utf8Path>,
{
    let terraform_dirs = terraform_dirs
        .iter()
        .map(|d| d.as_ref())
        .collect::<Vec<_>>();
    let terragrunt_dirs = terragrunt_dirs
        .iter()
        .map(|d| d.as_ref())
        .collect::<Vec<_>>();
    // logout before login, to avoid issues with multiple profiles
    aws::sso_logout();
    let login_env_vars = aws::legacy_login();
    let cmd_runner = CmdRunner::new(login_env_vars);

    for d in terraform_dirs {
        cmd_runner.terraform_init_upgrade(d);
    }
    for d in terragrunt_dirs {
        cmd_runner.terragrunt_init_upgrade(d);
    }
}

fn upgrade_terragrunt_with_sso<T>(terragrunt_sso_dirs: &BTreeMap<&str, Vec<T>>)
where
    T: AsRef<Utf8Path>,
{
    let terragrunt_sso_dirs = terragrunt_sso_dirs
        .iter()
        .map(|(k, v)| (*k, v.iter().map(|d| d.as_ref()).collect::<Vec<_>>()))
        .collect::<BTreeMap<_, _>>();
    for (account, dirs) in terragrunt_sso_dirs {
        aws::sso_logout();
        aws::sso_login(account);
        for d in dirs {
            CmdRunner::new(BTreeMap::new()).terragrunt_init_upgrade(d);
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
