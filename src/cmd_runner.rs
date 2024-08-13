use std::collections::BTreeMap;

use camino::Utf8Path;
use secrecy::SecretString;

use crate::cmd::Cmd;

#[derive(Debug, PartialEq)]
pub enum PlanOutcome {
    NoChanges,
    Changes(String),
}

pub struct CmdRunner {
    env_vars: BTreeMap<String, SecretString>,
}

impl CmdRunner {
    pub fn new(env_vars: BTreeMap<String, SecretString>) -> Self {
        Self { env_vars }
    }

    pub fn terragrunt_plan(&self, state: &Utf8Path) -> PlanOutcome {
        self.tg_or_tf_plan(state, "terragrunt")
    }

    pub fn terraform_plan(&self, module: &Utf8Path) -> PlanOutcome {
        self.tg_or_tf_plan(module, "terraform")
    }

    /// Check if Terragrunt or Terraform plan is clean.
    /// Useful to check wheter there are some unapplied changes in the repo.
    fn tg_or_tf_plan(&self, directory: &Utf8Path, command: &str) -> PlanOutcome {
        // The `-detailed-exitcode` returns the following exit codes:
        // 0 - Succeeded, diff is empty (no changes)
        // 1 - Errored
        // 2 - Succeeded, there is a diff
        let output = Cmd::new(command, ["plan", "-detailed-exitcode", "-input=false"])
            .with_env_vars(self.env_vars.clone())
            .with_current_dir(directory)
            .run();
        let is_diff_empty = output.status().code().unwrap() == 0;
        if is_diff_empty {
            PlanOutcome::NoChanges
        } else {
            PlanOutcome::Changes(output.stdout().to_string())
        }

    }

    pub fn terragrunt_init_upgrade(&self, state: &Utf8Path) {
        let output = Cmd::new("terragrunt", ["init", "--upgrade", "-input=false"])
            .with_env_vars(self.env_vars.clone())
            .with_current_dir(state)
            .run();
        assert!(output.status().success());
    }
}
