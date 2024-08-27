#[derive(clap::Parser, Debug)]
#[command(about, version, author)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Upgrade terragrunt states or Terraform modules.
    Upgrade(UpgradeArgs),
    /// Given a PR, run terragrunt/terraform plan on every module that changed.
    PlanPr(PlanPr),
    /// Select a provider and upgrade all lockfiles.
    UpgradeProvider,
    /// Create default configuration and print its path.
    /// If you are using 1Password, you can get an `ITEM_ID` by running
    /// `op item list`.
    Config,
    /// Login to the AWS legacy account.
    /// TODO: Doesn't work yet because it doesn't export the env vars.
    #[command(visible_alias = "ll")]
    LegacyLogin,
    /// Get the graph of the terraform modules to see how they depend on each other.
    Graph(GraphArgs),
}

#[derive(clap::Parser, Debug)]
pub struct UpgradeArgs {
    /// If true, copy the output to the clipboard.
    #[arg(long)]
    pub clipboard: bool,
}

#[derive(clap::Parser, Debug)]
pub struct PlanPr {
    /// PR Number OR URL OR Branch.
    pub pr: String,
    /// If true, copy the output to the clipboard.
    #[arg(long)]
    pub clipboard: bool,
}

#[derive(clap::Parser, Debug)]
pub struct GraphArgs {
    /// If true, copy the graphviz output to the clipboard.
    #[arg(long)]
    pub clipboard: bool,
    /// Check for outdated providers and show them in the graph.
    #[arg(long)]
    pub outdated: bool,
}
