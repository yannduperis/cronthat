mod cronthat;

use crate::cronthat::CronThat;
use anyhow::Context;
use clap::Parser;

fn main() {
    let cli = CronThat::parse();
    let res = cli.execute().context("Something went wrong");
    match res {
        Ok(_) => {}
        Err(err) => {
            println!("{:?}", err);
            std::process::exit(1);
        }
    }
}
