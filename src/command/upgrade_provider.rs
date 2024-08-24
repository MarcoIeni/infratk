use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use inquire::{list_option::ListOption, validator::Validation, MultiSelect};
use semver::Version;
use std::fmt;

use crate::{
    aws,
    cmd_runner::CmdRunner,
    config::Config,
    dir::{self, current_dir, current_dir_is_simpleinfra},
    grouped_dirs, provider, LOCKFILE,
};

pub async fn upgrade_provider(config: &Config) {
    assert!(current_dir_is_simpleinfra());
    let lockfiles = get_all_lockfiles();
    let providers = get_all_providers(&lockfiles);
    let outdated_providers = provider::outdated_providers(providers).await.unwrap();
    println!("\nOutdated providers: {outdated_providers}");
    let providers_list = outdated_providers.providers.keys().cloned().collect();
    let selected_providers = select_providers(providers_list);

    update_lockfiles(&outdated_providers, selected_providers, config);
}

fn update_lockfiles(providers: &Providers, selected_providers: Vec<String>, config: &Config) {
    // Filter out the providers that were not selected
    let filtered_providers = providers
        .providers
        .iter()
        .filter(|(k, _)| selected_providers.contains(k))
        .collect::<BTreeMap<_, _>>();

    let all_dirs: Vec<Utf8PathBuf> = filtered_providers
        .values()
        .flat_map(|v| v.versions.values())
        .flat_map(|paths| get_parents(paths.clone()))
        .collect();

    let grouped_dirs = grouped_dirs::GroupedDirs::new(all_dirs);

    if grouped_dirs.contains_legacy_account() {
        let legacy_tg_dirs = grouped_dirs.legacy_terragrunt_dirs();
        upgrade_legacy_dirs(grouped_dirs.terraform_dirs(), legacy_tg_dirs, config);
    }

    let sso_terragrunt_dirs = grouped_dirs.sso_terragrunt_dirs();
    upgrade_terragrunt_with_sso(&sso_terragrunt_dirs);
}

fn get_parents(paths: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    paths
        .iter()
        .map(|p| p.parent().unwrap().to_path_buf())
        .collect()
}

fn upgrade_legacy_dirs<T, U>(terraform_dirs: Vec<T>, terragrunt_dirs: Vec<U>, config: &Config)
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
    let login_env_vars = aws::legacy_login(config.op_legacy_item_id.as_deref());
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

#[derive(Debug, Clone)]
pub struct Providers {
    /// <provider name> -> <provider versions>
    pub providers: BTreeMap<String, ProviderVersions>,
}

impl fmt::Display for Providers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, versions) in &self.providers {
            writeln!(f, "- {name}:{versions}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProviderVersions {
    /// <version> -> <lockfile where the version is present>
    pub versions: BTreeMap<Version, Vec<Utf8PathBuf>>,
}

impl fmt::Display for ProviderVersions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (version, lockfiles) in &self.versions {
            let lockfiles_fmt = lockfiles
                .iter()
                .map(|l| l.strip_prefix(dir::current_dir()).unwrap())
                .collect::<Vec<_>>();
            writeln!(f, "\n  - {version} -> {lockfiles_fmt:?}")?;
        }
        Ok(())
    }
}

/// Get all providers from all lockfiles.
/// The result is a map where the key is the provider name and the value is the
/// list of lockfiles that use that provider.
pub fn get_all_providers(lockfiles: &[Utf8PathBuf]) -> Providers {
    let mut providers = BTreeMap::new();
    for lockfile in lockfiles {
        let content = std::fs::read_to_string(lockfile).expect("could not read lockfile");
        let mut lines = content.lines();
        while let Some(line) = lines.next() {
            if line.starts_with("provider") {
                let provider_name = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .trim_matches('"')
                    .strip_prefix("registry.terraform.io/")
                    .expect("invalid provider name")
                    .to_string();
                if let Some(version_line) = lines.next() {
                    let version = version_line
                        .split_whitespace()
                        .nth(2)
                        .unwrap()
                        .trim_matches('"');
                    let version = Version::parse(version).unwrap();
                    providers
                        .entry(provider_name)
                        .or_insert_with(|| ProviderVersions {
                            versions: BTreeMap::new(),
                        })
                        .versions
                        .entry(version)
                        .or_insert_with(Vec::new)
                        .push(lockfile.clone());
                }
            }
        }
    }
    Providers { providers }
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
