use camino::Utf8PathBuf;

use crate::{args::PlanPr, run_cmd::Cmd};

pub fn plan_pr(args: PlanPr) {
    let files_changes = get_files_changes(args.pr);
    println!("Files changed in PR: {:?}", files_changes);
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
