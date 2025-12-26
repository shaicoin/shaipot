use colored::*;
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(short, long)]
    pub threads: Option<usize>,
    #[clap(short, long)]
    pub address: Option<String>,
    #[clap(short, long)]
    pub pool: Option<String>,
    #[clap(long)]
    pub vdftime1: Option<String>,
    #[clap(long)]
    pub vdftime2: Option<String>,

    #[clap(skip)]
    pub vdftime1_parsed: u64,
    #[clap(skip)]
    pub vdftime2_parsed: u64,

}

impl Args {
    pub fn parse_and_validate() -> Args {
        let mut args = Args::parse();

        if args.address.is_none() || args.pool.is_none() {
            Args::show_demo_usage();
            std::process::exit(0);
        }

        // 解析vdftime1，默认值为1000毫秒
        args.vdftime1_parsed = if let Some(vdftime1_str) = args.vdftime1.clone() {
            match vdftime1_str.parse::<u64>() {
                Ok(vdf1) => vdf1,
                Err(_) => 1000, // 解析失败时使用默认值
            }
        } else {
            1000 // 未提供时使用默认值
        };

        // 解析vdftime2，默认值为10毫秒
        args.vdftime2_parsed = if let Some(vdftime2_str) = args.vdftime2.clone() {
            match vdftime2_str.parse::<u64>() {
                Ok(vdf2) => vdf2,
                Err(_) => 10, // 解析失败时使用默认值
            }
        } else {
            10 // 未提供时使用默认值
        };

        args
    }

    pub fn show_demo_usage() {
        println!();
        println!("{}", "Run the miner with required arguments:".bold().bright_yellow());
        println!("{}", "--address <shaicoin_address> --pool <POOL_URL>".bold().bright_red());
        println!("{}", "OPTIONAL: --threads <AMT>".bold().bright_red());
        println!("{}", "OPTIONAL: --vdftime1 <MILLISECONDS> (default: 1000)".bold().bright_red());
        println!("{}", "OPTIONAL: --vdftime2 <MILLISECONDS> (default: 10)".bold().bright_red());
        println!();
        println!("Example mining with 4 threads:");
        println!("./shaipot --address sh1qeexkz69dz6j4q0zt0pkn36650yevwc8eksqeuu --pool wss://pool.shaicoin.org --threads 4");
        println!("Example with custom vdftime1 and vdftime2:");
        println!("./shaipot --address sh1qeexkz69dz6j4q0zt0pkn36650yevwc8eksqeuu --pool wss://pool.shaicoin.org --vdftime1 2000 --vdftime2 20");
    }
}

#[derive(Serialize, Deserialize)]
pub struct SubmitMessage {
    pub r#type: String,
    pub miner_id: String,
    pub nonce: String,
    pub job_id: String,
    pub path: String,
}

#[derive(Deserialize, Debug)]
pub struct ServerMessage {
    pub r#type: String,
    pub job_id: Option<String>,
    pub data: Option<String>,
    pub target: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub job_id: String,
    pub data: String,
    pub target: String,
}