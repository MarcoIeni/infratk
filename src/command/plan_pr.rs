use core::panic;
use std::collections::BTreeMap;

use arboard::Clipboard;
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use regex::Regex;

use crate::{
    args::PlanPr,
    aws,
    cmd::Cmd,
    cmd_runner::{CmdRunner, PlanOutcome},
    dir::current_dir_is_simpleinfra,
    git::assert_current_branch_is_same_as_pr,
    LOCKFILE,
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
    let output_str = format_output(output);
    println!("{output_str}");
    if args.clipboard {
        // Strip ANSI escape sequences
        let re = Regex::new(r"\x1b\[([\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e])").unwrap();
        let output_str = re.replace_all(&output_str, "").to_string();

        let mut clipboard = Clipboard::new().unwrap();
        clipboard.set_text(output_str).unwrap();
    }
}

/// Print two lists of directories, one for each outcome
fn format_output(output: Vec<(&Utf8Path, PlanOutcome)>) -> String {
    let mut output_str = String::from("## 📃📃 Plan summary 📃📃\n");
    let (no_changes, changes): (Vec<_>, Vec<_>) = output
        .into_iter()
        .partition(|(_, o)| matches!(o, PlanOutcome::NoChanges));
    if !no_changes.is_empty() {
        output_str.push_str("\nNo changes detected (apply not needed):\n");
    }
    for (dir, _) in no_changes {
        output_str.push_str(&format!("✅ {}\n", dir));
    }

    if !changes.is_empty() {
        output_str.push_str("\nChanges detected (apply needed):\n");
    }
    for (dir, _) in &changes {
        output_str.push_str(&format!("❌ {}\n", dir));
    }

    if !changes.is_empty() {
        output_str.push_str("\n## 📃📃 Plan output 📃📃\n");
    }
    for (dir, output) in &changes {
        output_str.push_str(&format!("👉 {}:\n", dir));
        if let PlanOutcome::Changes(output) = output {
            output_str.push_str(&format!("\n```\n{}\n```\n", output));
        } else {
            panic!("Expected changes, got no changes");
        }
    }

    output_str
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
            .cloned()
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
        .map(|(account, dirs)| (account.as_str(), dirs.clone()))
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
        .filter(|file| file.file_name() == Some(LOCKFILE))
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
