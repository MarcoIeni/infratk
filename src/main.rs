mod args;
mod aws;
mod clipboard;
mod cmd;
mod cmd_runner;
mod command;
mod config;
mod dir;
mod git;
mod grouped_dirs;
mod log;
mod provider;
mod select;

use args::CliArgs;
use clap::Parser as _;

const LOCKFILE: &str = ".terraform.lock.hcl";

#[tokio::main]
async fn main() {
    log::init(true);
    let args = CliArgs::parse();
    let config = config::parse_config().unwrap();
    match args.command {
        args::Command::Upgrade => command::upgrade::upgrade(&config),
        args::Command::PlanPr(args) => command::plan_pr::plan_pr(args, &config),
        args::Command::UpgradeProvider => {
            command::upgrade_provider::upgrade_provider(&config).await
        }
        args::Command::Config => command::config_cmd::create_default_config(),
        args::Command::LegacyLogin => command::legacy_login::login_to_legacy_aws_account(&config),
        args::Command::Graph(args) => command::graph::print_graph(args).await,
    }
}
