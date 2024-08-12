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
    PlanPr(PlanPr),
}

#[derive(clap::Parser, Debug)]
pub struct PlanPr {
    pr_number: u32,
}
