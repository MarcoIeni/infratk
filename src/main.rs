mod args;
mod aws;
mod cmd;
mod cmd_runner;
mod command;
mod dir;
mod git;
mod log;
mod select;

use args::CliArgs;
use clap::Parser as _;

const LOCKFILE: &str = ".terraform.lock.hcl";

fn main() {
    let args = CliArgs::parse();
    log::init(true);
    match args.command {
        args::Command::Upgrade => command::upgrade::upgrade(),
        args::Command::PlanPr(args) => command::plan_pr::plan_pr(args),
        args::Command::UpgradeProvider => command::upgrade_provider::upgrade_provider(),
    }
}
