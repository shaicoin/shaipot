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

use utils::*;
use std::ptr;
use models::*;
use hasher::*;
use colored::*;
use std::thread;
use ascii_art::*;
use std::sync::Arc;
use std::time::Duration;
use vdf_solution::HCGraphUtil;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use std::sync::{atomic::{AtomicUsize, AtomicPtr, Ordering}, mpsc};

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

    let miner_id = args.address.unwrap();
    let request = args.pool.unwrap().clone().into_client_request().unwrap();
    let (ws_stream, _) = connect_async(request)
        .await
        .expect("Failed to connect to WebSocket server");

    let (write, mut read) = ws_stream.split();
    let (server_sender, server_receiver) = mpsc::channel::<String>();

    let current_job: Arc<AtomicPtr<Option<Job>>> = Arc::new(AtomicPtr::new(ptr::null_mut()));

    // Spawn write task to send solutions to the server
    tokio::spawn(async move {
        let mut write = write;
        while let Ok(msg) = server_receiver.recv() {
            write.send(Message::Text(msg)).await.unwrap();
        }
    });

    // Spawn worker threads for processing jobs
    let hash_count = Arc::new(AtomicUsize::new(0));
    for _ in 0..num_workers {
        let current_job_loop = Arc::clone(&current_job);
        let hash_count = Arc::clone(&hash_count);
        let server_sender_clone = server_sender.clone();
        let miner_id = miner_id.clone();

        thread::spawn(move || {
            let mut hc_util = HCGraphUtil::new();
            loop {
                let job_option = unsafe {
                    let job_ptr = current_job_loop.load(Ordering::SeqCst);
                    if job_ptr.is_null() {
                        None
                    } else {
                        (*job_ptr).clone()
                    }
                };

                if let Some(job) = job_option {
                    loop {
                        let nonce = generate_nonce();

                        if let Some((hash, path_hex)) = compute_hash_no_vdf(&("".to_owned() + &job.data + &nonce), &mut hc_util) {
                            hash_count.fetch_add(1, Ordering::Relaxed);

                            if meets_target(&hash, &job.target) {
                                let submit_msg = SubmitMessage {
                                    r#type: String::from("submit"),
                                    miner_id: miner_id.to_string(),
                                    nonce: nonce,
                                    job_id: job.job_id.clone(),
                                    path: path_hex,
                                };

                                let msg = serde_json::to_string(&submit_msg).unwrap();
                                let _ = server_sender_clone.send(msg);

                                // dealloc the job
                                let none_ptr = Box::into_raw(Box::new(None));
                                let old_ptr = current_job_loop.swap(none_ptr, Ordering::SeqCst);
                                if !old_ptr.is_null() {
                                    unsafe {
                                        let _ = Box::from_raw(old_ptr);
                                    }
                                }
                                break;
                            }

                            // Check if there is a new job
                            let new_job_option = unsafe {
                                let job_ptr = current_job_loop.load(Ordering::SeqCst);
                                if job_ptr.is_null() {
                                    None
                                } else {
                                    (*job_ptr).clone()
                                }
                            };

                            if new_job_option.is_none() || new_job_option.unwrap().job_id != job.job_id {
                                break;
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
            thread::sleep(Duration::from_secs(5));
            let count = hash_count.load(Ordering::Relaxed);
            println!("{}: {} hashes/second", "Hash rate".cyan(), (count - last_count) / 5);
            last_count = count;
        }
    });

    // Run the read task
    let current_job_clone = Arc::clone(&current_job);
    loop {
        match read.next().await {
            Some(Ok(msg)) => {
                let server_message: ServerMessage =
                    serde_json::from_str(msg.to_text().unwrap()).unwrap();
                match server_message.r#type.as_str() {
                    "job" => {
                        if let (Some(job_id), Some(data), Some(target)) = (
                            server_message.job_id.clone(),
                            server_message.data.clone(),
                            server_message.target.clone(),
                        ) {
                            let new_job = Box::new(Some(Job {
                                job_id: job_id.clone(),
                                data: data.clone(),
                                target: target.clone(),
                            }));
                            let job_ptr = Box::into_raw(new_job);

                            let old_ptr = current_job_clone.swap(job_ptr, Ordering::SeqCst);
                            if !old_ptr.is_null() {
                                unsafe {
                                    let _ = Box::from_raw(old_ptr); // Free the old job
                                }
                            }

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
                        println!(
                            "{}",
                            format!("Share accepted")
                                .bold()
                                .green()
                        );
                        display_share_accepted();
                    }
                    "rejected" => {
                        println!("{}", "Share rejected.".red());
                    }
                    _ => {}
                }
            }
            Some(Err(e)) => {
                println!("Error receiving message: {:?}", e);
                break;
            }
            None => {
                println!("WebSocket connection closed.");
                break;
            }
        }
    }    
}
