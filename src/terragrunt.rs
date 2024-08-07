use camino::Utf8Path;

use crate::run_cmd::Cmd;

/// Check if Terragrunt plan is clean.
/// Useful to check wheter there are some unapplied changes in the repo.
pub fn are_changes_applied(state: &Utf8Path) -> bool {
    // The `-detailed-exitcode` returns the following exit codes:
    // 0 - Succeeded, diff is empty (no changes)
    // 1 - Errored
    // 2 - Succeeded, there is a diff
    let output = Cmd::new("terragrunt", ["plan", "-detailed-exitcode", "-input=false"])
        .with_current_dir(state)
        .run();
    output.status().code().unwrap() == 0
}

pub fn terragrunt_init_upgrade(state: &Utf8Path) {
    let output = Cmd::new("terragrunt", ["init", "--upgrade", "-input=false"])
        .with_current_dir(state)
        .run();
    assert!(output.status().success());
}
