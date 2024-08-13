use std::collections::BTreeMap;

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

use crate::{
    args::PlanPr,
    aws,
    run_cmd::Cmd,
    terragrunt::{self, PlanOutcome},
};

pub fn plan_pr(args: PlanPr) {
    let files_changed = get_files_changes(args.pr);
    println!("Files changed in PR: {:?}", files_changed);
    let lock_files = get_lock_files(files_changed);
    println!("Lock files changed in PR: {:?}", lock_files);
    let directories: Vec<&Utf8Path> = lock_files
        .iter()
        .map(|file| file.parent().unwrap())
        .collect();
    plan_directories(directories);
}

fn plan_directories(directories: Vec<&Utf8Path>) {
    // Plan terragrunt first because legacy authentication has precedence over aws sso.
    let terragrunt_dirs: Vec<&Utf8Path> = get_dirs_starting_with(directories.clone(), "terragrunt");
    let grouped_terragrunt_dirs = group_terragrunt_dirs_by_account(terragrunt_dirs);
    plan_all_terragrunt_dirs(&grouped_terragrunt_dirs);
    let terraform_dirs: Vec<&Utf8Path> = get_dirs_starting_with(directories.clone(), "terraform");
    if !grouped_terragrunt_dirs.contains_key("legacy") {
        // we stell need to do legacy login
        aws::sso_logout();
        aws::legacy_login();
    }
    plan_terraform_dirs(terraform_dirs);
}

fn plan_all_terragrunt_dirs<'a>(
    grouped_terragrunt_dirs: &BTreeMap<String, Vec<&'a Utf8Path>>,
) -> Vec<(&'a Utf8Path, PlanOutcome)> {
    let mut outcome = vec![];

    {
        let non_legacy_dirs: BTreeMap<&str, Vec<&Utf8Path>> = grouped_terragrunt_dirs
            .iter()
            .filter(|(account, _)| !account.starts_with("legacy"))
            .map(|(account, dirs)| (account.as_str(), dirs.to_vec()))
            .collect();
        let plan_outcome = plan_terragrunt_dirs(&non_legacy_dirs);
        outcome.extend(plan_outcome)
    }

    {
        let legacy_dirs: BTreeMap<&str, Vec<&Utf8Path>> = grouped_terragrunt_dirs
            .iter()
            .filter(|(account, _)| account.starts_with("legacy"))
            .map(|(account, dirs)| (account.as_str(), dirs.clone()))
            .collect();
        let plan_outcome = plan_terragrunt_dirs(&legacy_dirs);
        outcome.extend(plan_outcome);
    }

    outcome
}

fn plan_terragrunt_dirs<'a>(
    grouped_terragrunt_dirs: &BTreeMap<&str, Vec<&'a Utf8Path>>,
) -> Vec<(&'a Utf8Path, PlanOutcome)> {
    let mut outcome = vec![];
    for (account, dirs) in grouped_terragrunt_dirs {
        // logout before login, to avoid issues with multiple profiles
        aws::sso_logout();
        aws::login(account);
        for d in dirs {
            let plan_outcome = terragrunt::are_changes_applied(d);
            outcome.push((*d, plan_outcome));
        }
    }
    outcome
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

fn plan_terraform_dirs(terraform_dirs: Vec<&Utf8Path>) -> Vec<(&Utf8Path, PlanOutcome)> {
    let mut outcome = vec![];
    for d in terraform_dirs {
        let plan_outcome = terragrunt::are_changes_applied(d);
        outcome.push((d, plan_outcome));
    }
    outcome
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

fn get_files_changes(pr: String) -> Vec<Utf8PathBuf> {
    Cmd::new("gh", ["pr", "diff", &pr, "--name-only"])
        .hide_stdout()
        .run()
        .stdout()
        .lines()
        .map(Utf8PathBuf::from)
        .collect()
}

fn get_lock_files(files: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    files
        .iter()
        .filter(|file| file.file_name() == Some(".terraform.lock.hcl"))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_files_are_filtered() {
        let files = vec![
            Utf8PathBuf::from("main.tf"),
            Utf8PathBuf::from("module1/main.tf"),
            Utf8PathBuf::from("module1/.terraform.lock.hcl"),
            Utf8PathBuf::from("module2/.terraform.lock.hcl"),
        ];
        let lock_files = get_lock_files(files);
        assert_eq!(
            lock_files,
            vec![
                Utf8PathBuf::from("module1/.terraform.lock.hcl"),
                Utf8PathBuf::from("module2/.terraform.lock.hcl"),
            ]
        );
    }
}
