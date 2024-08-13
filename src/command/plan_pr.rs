use core::panic;
use std::collections::BTreeMap;

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

use crate::{
    args::PlanPr,
    aws,
    cmd::Cmd,
    cmd_runner::{CmdRunner, PlanOutcome},
    dir::current_dir_is_simpleinfra,
};

pub fn plan_pr(args: PlanPr) {
    assert!(current_dir_is_simpleinfra());
    assert_current_branch_is_same_as_pr(&args.pr);
    let files_changed = get_files_changes(args.pr);
    println!("Files changed in PR: {:?}", files_changed);
    let lock_files = get_lock_files(files_changed);
    println!("Lock files changed in PR: {:?}", lock_files);
    let directories: Vec<&Utf8Path> = lock_files
        .iter()
        .map(|file| file.parent().unwrap())
        .collect();
    let output = plan_directories(directories);
    print_output(output);
}

fn assert_current_branch_is_same_as_pr(pr: &str) {
    let current_branch = get_current_branch();
    let pr_branch = get_pr_branch(pr);
    assert_eq!(
        current_branch, pr_branch,
        "You are not in the same branch as the PR locally"
    );
}

fn get_current_branch() -> String {
    Cmd::new("git", ["rev-parse", "--abbrev-ref", "HEAD"])
        .hide_stdout()
        .run()
        .stdout()
        .trim()
        .to_string()
}

fn get_pr_branch(pr: &str) -> String {
    let output = Cmd::new(
        "gh",
        [
            "pr",
            "view",
            pr,
            "--json",
            "headRefName",
            "-q",
            ".headRefName",
        ],
    )
    .hide_stdout()
    .run();
    output.stdout().trim().to_string()
}

/// Print two lists of directories, one for each outcome
fn print_output(output: Vec<(&Utf8Path, PlanOutcome)>) {
    let (no_changes, changes): (Vec<_>, Vec<_>) = output
        .into_iter()
        .partition(|(_, o)| matches!(o, PlanOutcome::NoChanges));
    println!("📃📃 Plan summary 📃📃");
    println!("No changes detected (apply not needed):");
    for (dir, _) in no_changes {
        println!("✅ {}", dir);
    }
    println!("Changes detected (apply needed):");
    for (dir, _) in &changes {
        println!("❌ {}", dir);
    }

    println!("\n📃📃 Plan output 📃📃");
    for (dir, output) in &changes {
        println!("👉 {}:", dir);
        if let PlanOutcome::Changes(output) = output {
            println!("{}", output);
        } else {
            panic!("Expected changes, got no changes");
        }
    }
}

fn plan_directories(directories: Vec<&Utf8Path>) -> Vec<(&Utf8Path, PlanOutcome)> {
    // Plan terragrunt first because legacy authentication has precedence over aws sso.
    let terragrunt_dirs: Vec<&Utf8Path> = get_dirs_starting_with(directories.clone(), "terragrunt");
    let terraform_dirs: Vec<&Utf8Path> = get_dirs_starting_with(directories.clone(), "terraform");
    let grouped_terragrunt_dirs = group_terragrunt_dirs_by_account(terragrunt_dirs);

    let mut output = vec![];
    let should_work_on_legacy =
        grouped_terragrunt_dirs.contains_key("legacy") || !terraform_dirs.is_empty();
    if should_work_on_legacy {
        let legacy_tg_dirs: Vec<&Utf8Path> = grouped_terragrunt_dirs
            .get("legacy")
            .map(|dirs| dirs.to_vec())
            .unwrap_or_default();
        let o = plan_legacy_dirs(terraform_dirs, legacy_tg_dirs);
        output.extend(o);
    }

    let o = plan_sso_terragrunt_dirs(&grouped_terragrunt_dirs);
    output.extend(o);
    output
}

fn plan_legacy_dirs<'a>(
    terraform_dirs: Vec<&'a Utf8Path>,
    terragrunt_dirs: Vec<&'a Utf8Path>,
) -> Vec<(&'a Utf8Path, PlanOutcome)> {
    // logout before login, to avoid issues with multiple profiles
    aws::sso_logout();
    let login_env_vars = aws::legacy_login();
    let cmd_runner = CmdRunner::new(login_env_vars);

    let mut output = vec![];
    for d in terraform_dirs {
        // Skip the terraform/releases directory since it fails
        if d == "terraform/releases" {
            continue;
        }
        let o = cmd_runner.terraform_plan(d);
        output.push((d, o));
    }
    for d in terragrunt_dirs {
        let o = cmd_runner.terragrunt_plan(d);
        output.push((d, o));
    }
    output
}

fn plan_sso_terragrunt_dirs<'a>(
    grouped_terragrunt_dirs: &BTreeMap<String, Vec<&'a Utf8Path>>,
) -> Vec<(&'a Utf8Path, PlanOutcome)> {
    let non_legacy_dirs: BTreeMap<&str, Vec<&Utf8Path>> = grouped_terragrunt_dirs
        .iter()
        .filter(|(account, _)| !account.starts_with("legacy"))
        .map(|(account, dirs)| (account.as_str(), dirs.to_vec()))
        .collect();
    run_terragrunt_plan_with_sso(&non_legacy_dirs)
}

fn run_terragrunt_plan_with_sso<'a>(
    grouped_terragrunt_dirs: &BTreeMap<&str, Vec<&'a Utf8Path>>,
) -> Vec<(&'a Utf8Path, PlanOutcome)> {
    let mut outcome = vec![];
    for (account, dirs) in grouped_terragrunt_dirs {
        aws::sso_logout();
        aws::sso_login(account);
        for d in dirs {
            let plan_outcome = CmdRunner::new(BTreeMap::new()).terragrunt_plan(d);
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
