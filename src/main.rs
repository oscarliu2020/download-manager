use std::time::Duration;

use clap::Parser;
use download_manager::{cmd::Cmd, item::Item};
use tokio::{self, task::JoinSet};
use reqwest::{Client, ClientBuilder};
use tracing::{info, warn,debug,error};
use indicatif::{style, MultiProgress, ProgressBar, ProgressStyle};
use tokio_util::task::TaskTracker;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cmd=Cmd::parse();
    // println!("{:?}",cmd);
    let verbosity=if cmd.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::WARN
    };
    let subscriber=tracing_subscriber::FmtSubscriber::builder()
        // .without_time()
        // .with_target(false)
        .with_max_level(verbosity)
        .compact()
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let targets=cmd.get_targets();
    let mut items=vec![];
    for target in targets {
        println!("Detecting: {}",target);
        let res=Item::new(target).await;
        match res {
            Ok(item) => {
                items.push(item);
            }
            Err(e) => {
                error!("{}",e);
            }
        }
    }
    println!("Start downloading...");
    let client=ClientBuilder::new().build().unwrap();
    let tracker=TaskTracker::new();
    let mp=MultiProgress::new();
    let style=ProgressStyle::with_template("{msg} {eta_precise} {spinner} {bar:40.cyan/blue} {decimal_bytes}/{decimal_total_bytes} {decimal_bytes_per_sec}").unwrap().progress_chars("##-");
    let mut handles =vec![];
    for mut item in items{
        let pb=mp.add(ProgressBar::new(item.size).with_style(style.clone()));
        pb.set_message(item.filename.clone());
        let client=client.clone();
        let hdl=tracker.spawn(async move {
            item.download(client, pb).await
        });
        handles.push(hdl);
    }
    tracker.close();
    tracker.wait().await;
    for h in handles {
        match h.await {
            Err(e)=> {
                error!("{}",e);
            },
            _=>()
        }
    }
}
