mod args;
mod aws;
mod log;
mod run_cmd;
mod select;
mod terragrunt;
mod command;
mod dir;

use args::CliArgs;
use clap::Parser as _;

fn main() {
    let args = CliArgs::parse();
    log::init(true);
    match args.command {
        args::Command::Upgrade => command::upgrade::upgrade(),
        args::Command::PlanPr(args) => command::plan_pr::plan_pr(args),
    }
}
