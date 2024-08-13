#[derive(clap::Parser, Debug)]
#[command(about, version, author)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Upgrade terragrunt states or Terraform modules.
    Upgrade,
    /// Given a PR, run terragrunt/terraform plan on every module that changed.
    PlanPr(PlanPr),
}

#[derive(clap::Parser, Debug)]
pub struct PlanPr {
    /// PR Number OR URL OR Branch.
    pub pr: String,
}