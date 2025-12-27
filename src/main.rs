//           ,____           \'/
//       .-'` .   |        -= * =-
//     .'  '    ./           /.\
//    /  '    .'        
//   ;  '    /           
//  :  '  _ ;            
// ;  :  /(\ \
// |  .       '.
// |  ' /     --'
// |  .   '.__\
// ;  :       /
//  ;  .     |            ,
//   ;  .    \           /|
//    \  .    '.       .'/
//     '.  '  . `'---'`.'
//       `'-..._____.-`
//
// Care about the emission. It’s freedom in code.
// Just a pulse in the network, a chance to be heard.
//
mod vdf_solution;
mod ascii_art;
mod models;
mod hasher;
mod utils;
mod api;
mod graph_bridge;

use utils::*;
use models::*;
use hasher::*;
use rand::Rng;
use colored::*;
use std::thread;
use ascii_art::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex};
use crate::api::MinerState;
use vdf_solution::HCGraphUtil;
use futures_util::{StreamExt, SinkExt};
use std::sync::{atomic::{AtomicUsize, Ordering}, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

#[tokio::main]
async fn main() {
    let args = Args::parse_and_validate();
    std::panic::set_hook(Box::new(|_info| {}));

    let max_workers = num_cpus::get();
    assert!(max_workers > 0);

    let num_workers = match args.threads {
        Some(t) => {
            if t >= max_workers {
                println!("{}", "Requested number of threads exceeds available cores. Using maximum allowed".bold().red());
                max_workers
            } else {
                t
            }
        }
        None => max_workers,
    };

    println!("{}", "STARTING MINER".bold().green());
    println!("{} {}", "USING WORKERS: ".bold().cyan(), format!("{}", num_workers).bold().cyan());
    print_startup_art();

    // Handle Ctrl+C signal
    tokio::spawn(handle_exit_signals());

    let bailout_timer = Some(args.vdftime1_parsed);
    let miner_id = args.address.unwrap();

    let (server_sender, server_receiver) = mpsc::channel::<String>();

    let current_job: Arc<Mutex<Option<Job>>> = Arc::new(Mutex::new(None));

    let miner_state = Arc::new(MinerState {
        hash_count: Arc::new(AtomicUsize::new(0)),
        accepted_shares: Arc::new(AtomicUsize::new(0)),
        rejected_shares: Arc::new(AtomicUsize::new(0)),
        hashrate_samples: Arc::new(Mutex::new(Vec::new())),
        version: String::from("2.0.0"),
    });

    // Spawn worker threads for processing jobs
    let hash_count = Arc::new(AtomicUsize::new(0));
    for _ in 0..num_workers {
        let current_job_loop = Arc::clone(&current_job);
        let hash_count = Arc::clone(&hash_count);
        let server_sender_clone = server_sender.clone();
        let miner_id = miner_id.clone();
        let api_hash_count = Arc::clone(&miner_state.hash_count);

        thread::spawn(move || {
            let mut hc_util = HCGraphUtil::new(bailout_timer);
            loop {
                let job_option = {
                    let job_guard = current_job_loop.blocking_lock();
                    job_guard.clone()
                };

                if let Some(job) = job_option {
                    loop {
                        let nonce = generate_nonce();

                        let data_with_nonce = format!("{}{}", job.data, nonce);
                        
                        match compute_hash_no_vdf(
                            &data_with_nonce, 
                            &mut hc_util, 
                            args.vdftime1_parsed, 
                            args.vdftime2_parsed,
                            &hash_count,
                            &api_hash_count,
                            &job,
                            &nonce,
                            &miner_id,
                            &server_sender_clone
                        ) {
                            Some(true) => {
                                // 找到有效解并已提交，清除当前作业
                                let mut job_guard = current_job_loop.blocking_lock();
                                *job_guard = None;
                                break;
                            },
                            Some(false) => {
                                // 找到解但不满足难度要求，继续挖矿
                                // 检查是否有新作业
                                let new_job_option = {
                                    let job_guard = current_job_loop.blocking_lock();
                                    job_guard.clone()
                                };

                                if new_job_option.is_none() || new_job_option.unwrap().job_id != job.job_id {
                                    break;
                                }
                            },
                            None => {
                                // 没有找到解，继续尝试
                            }
                        }
                    }
                }
            }
        });
    }

    // Spawn hash rate monitoring task
    tokio::spawn(async move {
        let mut last_count = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let count = hash_count.load(Ordering::Relaxed);
            println!("{}: {:.2} hashes/second", "Hash rate".cyan(), (count - last_count) as f64 / 5.0);
            last_count = count;
        }
    });

    // Spawn api hash rate monitoring task
    let api_hashrate_clone = miner_state.clone();
    tokio::spawn(async move {
        let mut last_count = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await; // Measure every second
            let current_count = api_hashrate_clone.hash_count.load(Ordering::Relaxed);
            let hashes_per_second = current_count - last_count;
            let mut samples = api_hashrate_clone.hashrate_samples.lock().await;
            samples.push(hashes_per_second as u64);
            if samples.len() > 10 {
                samples.remove(0);
            }
            last_count = current_count;
        }
    });

    let api_state = miner_state.clone();
    tokio::spawn(api::start_http_server(api_state));

    let current_job_clone = Arc::clone(&current_job);
    let request_clone = args.pool.unwrap().clone();

    let server_receiver = Arc::new(Mutex::new(server_receiver));

    loop {
        let request = request_clone.clone().into_client_request().unwrap();
        let (ws_stream, _) = match connect_async(request).await {
            Ok((ws_stream, response)) => {
                (ws_stream, response)
            }
            Err(_e) => {
                let delay_secs = rand::thread_rng().gen_range(5..30);
                println!("{}", format!("Failed to connect will retry in {} seconds...", delay_secs).red());
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                continue;
            }
        };

        let (write, mut read) = ws_stream.split();

        // Spawn write task to send solutions to the server
        let server_receiver_clone = Arc::clone(&server_receiver);
        tokio::spawn(async move {
            let mut write = write;
            while let Ok(msg) = {
                let receiver = server_receiver_clone.lock().await;
                receiver.recv()
            } {
                write.send(Message::Text(msg)).await.unwrap();
            }
        });
        
        loop {
            match read.next().await {
                Some(Ok(msg)) => {
                    match msg {
                        Message::Text(text_msg) => {
                            let server_message: ServerMessage =
                                serde_json::from_str(&text_msg).unwrap();
                            match server_message.r#type.as_str() {
                                "job" => {
                                    if let (Some(job_id), Some(data), Some(target)) = (
                                        server_message.job_id.clone(),
                                        server_message.data.clone(),
                                        server_message.target.clone(),
                                    ) {
                                        let new_job = Job {
                                            job_id: job_id.clone(),
                                            data: data.clone(),
                                            target: target.clone(),
                                        };
        
                                        let mut job_guard = current_job_clone.lock().await;
                                        *job_guard = Some(new_job);
        
                                        println!(
                                            "{} {}",
                                            "Received new job:".bold().blue(),
                                            format!(
                                                "ID = {}, Data = {}, Target = {}",
                                                job_id, data, target
                                            )
                                            .bold()
                                            .yellow()
                                        );
                                    }
                                }
                                "accepted" => {
                                    miner_state.accepted_shares.fetch_add(1, Ordering::Relaxed);
                                    println!(
                                        "{}",
                                        format!("Share accepted")
                                            .bold()
                                            .green()
                                    );
                                    display_share_accepted();
                                }
                                "rejected" => {
                                    miner_state.rejected_shares.fetch_add(1, Ordering::Relaxed);
                                    println!("{}", "Share rejected.".red());
                                }
                                _ => {}
                            }
                        }
                        Message::Close(_) => {
                            println!("{}", "You are now a frog.".green());
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                }
                Some(Err(_e)) => {
                    println!("{}", "WebSocket connection closed. Will sleep then try to reconnect.".red());
                    break;
                }
                None => {
                    println!("{}", "WebSocket connection closed. Will sleep then try to reconnect.".red());
                    break;
                }
            }
        }

        let mut job_guard = current_job_clone.lock().await;
        *job_guard = None;

        let delay_secs = rand::thread_rng().gen_range(11..42);
        println!("{}", format!("Reconnecting in {} seconds...", delay_secs).yellow());
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        println!("{}", "Attempting to reconnect...".red());
    }
}