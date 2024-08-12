mod args;
mod aws;
mod log;
mod run_cmd;
mod select;
mod terragrunt;
mod upgrade;

use args::CliArgs;
use clap::Parser as _;

fn main() {
    let args = CliArgs::parse();
    log::init(true);
    match args.command {
        args::Command::Upgrade => upgrade::upgrade(),
        args::Command::PlanPr(_plan_pr) => todo!(),
    }
}
