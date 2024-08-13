mod args;
mod aws;
mod log;
mod cmd;
mod select;
mod command;
mod dir;
mod cmd_runner;

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
