use std::process::exit;

use civitdl::Civit;
use clap;
use clap::{arg, Parser};
use env_logger;
use futures::future::join_all;
use tokio;
use tracing::error;

mod model;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    ids: Vec<String>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let args = Args::parse();
    let mut ids = args.ids;

    if ids.len() == 0 {
        error!("No model ids provided! Exiting ...");
        exit(1)
    } else {
        println!("Parsed IDs: {ids:?}");
    }

    env_logger::init();
    let civit = Civit::new();
    let mut res = Vec::new();
    let results = join_all(ids.iter_mut().map(|id| async {
        let civit_client = civit.clone();
        let model_id = id.clone();
        println!("Attempting to download model {model_id} ...");
        let model = civit_client
            .get_model_details(id.clone())
            .await
            .expect(format!("Failed to get model details for {model_id}").as_str());
        model
        // TODO: get model details
        // TODO: download model resources
    }).collect::<Vec<_>>()).await;

    res.extend(results);
    
    println!("Final results: {res:#?}");
}
