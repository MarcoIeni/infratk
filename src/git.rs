use crate::cmd::Cmd;

pub fn assert_current_branch_is_same_as_pr(pr: &str) {
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
