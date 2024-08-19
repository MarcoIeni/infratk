use std::collections::BTreeMap;

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

/// Directoried grouped by type and account
pub struct GroupedDirs {
    /// Directories under the terraform directory
    terraform: Vec<Utf8PathBuf>,
    /// Directories under the terragrunt directory, grouped by account
    terragrunt: BTreeMap<String, Vec<Utf8PathBuf>>,
}

impl GroupedDirs {
    pub fn new<T>(directories: Vec<T>) -> Self
    where
        T: AsRef<Utf8Path>,
    {
        let directories: Vec<&Utf8Path> = directories.iter().map(|d| d.as_ref()).collect();
        let terragrunt_dirs: Vec<&Utf8Path> =
            get_dirs_starting_with(directories.clone(), "terragrunt");
        let terraform_dirs: Vec<&Utf8Path> =
            get_dirs_starting_with(directories.clone(), "terraform");
        let grouped_terragrunt_dirs = group_terragrunt_dirs_by_account(terragrunt_dirs);
        Self {
            terraform: terraform_dirs
                .into_iter()
                .map(|d| d.to_path_buf())
                .collect(),
            terragrunt: grouped_terragrunt_dirs
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(|d| d.to_path_buf()).collect()))
                .collect(),
        }
    }

    pub fn contains_legacy_account(&self) -> bool {
        self.terragrunt.contains_key("legacy") || !self.terraform.is_empty()
    }

    pub fn terraform_dirs(&self) -> Vec<&Utf8Path> {
        self.terraform
            .iter()
            .map(|d| d.as_path())
            .collect::<Vec<_>>()
    }

    pub fn legacy_terragrunt_dirs(&self) -> Vec<Utf8PathBuf> {
        self.terragrunt.get("legacy").cloned().unwrap_or_default()
    }

    /// Returns a map of account names to directories.
    /// Legacy account is excluded.
    pub fn sso_terragrunt_dirs(&self) -> BTreeMap<&str, Vec<Utf8PathBuf>> {
        self.terragrunt
            .iter()
            .filter(|(account, _)| !account.starts_with("legacy"))
            .map(|(account, dirs)| (account.as_str(), dirs.clone()))
            .collect()
    }
}

fn get_dirs_starting_with<'a>(directories: Vec<&'a Utf8Path>, name: &str) -> Vec<&'a Utf8Path> {
    directories
        .into_iter()
        .filter(|&d| is_root_dir(d, name))
        .collect()
}

fn is_root_dir(dir: &Utf8Path, name: &str) -> bool {
    dir.components().next() == Some(Utf8Component::Normal(name))
}

fn group_terragrunt_dirs_by_account(
    terragrunt_dirs: Vec<&Utf8Path>,
) -> BTreeMap<String, Vec<&Utf8Path>> {
    let mut dirs = BTreeMap::new();
    for d in terragrunt_dirs {
        let mut components = d.components();
        let terragrunt_dir = components.next().unwrap();
        assert_eq!(terragrunt_dir, Utf8Component::Normal("terragrunt"));
        let accounts_dir = components.next().unwrap();
        assert_eq!(accounts_dir, Utf8Component::Normal("accounts"));
        let account = components.next().unwrap();
        // Add the directory to the account's list of directories.
        // If the account does not exist, create a new list with the directory.
        // If the account exists, append the directory to the list.
        dirs.entry(account.to_string())
            .or_insert_with(Vec::new)
            .push(d);
    }
    dirs
}
