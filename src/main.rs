use std::process::exit;

use civitdl::Civit;
use clap;
use clap::{arg, ArgAction, Parser};
use env_logger;
use futures::future::join_all;
use tokio;
use tracing::{debug, error, info};
mod model;
use civitdl::Config;
use dotenvy;
use envy;
use env_logger::Env;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, long_help = "The IDs of the models to download", action=ArgAction::Set, num_args=1..)]
    ids: Vec<String>,

    #[arg(
        short,
        long,
        long_help = "Whether to download all available versions/resources of the specified models"
    )]
    all: bool,

    #[arg(short, long, long_help = "The ID of the model version to download")]
    override_id: Option<String>,
}


#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    dotenvy::dotenv().unwrap();

    let args = Args::parse();
    let mut ids = args.ids;

    if ids.len() == 0 {
        error!("No model ids provided! Exiting ...");
        exit(1)
    } else {
        info!("Parsed IDs: {ids:?}");
    }

    let mut config: Option<Config> = None;

    match envy::from_env::<Config>() {
        Ok(parsed_config) => {
            debug!("Parsed config: {:#?}", &parsed_config);
            config = Some(parsed_config);
        }
        Err(e) => {
            error!(error =? e);
        }
    }

    let all = args.all;

    let civit = Civit::new(config);
    let mut res = Vec::new();
    let override_id = args.override_id;


    if let Some(oid) = override_id {
        let id = ids.first().unwrap();
        let civit_client = civit.clone();
        let model_id = id.clone();

        let model = civit_client
            .clone()
            .get_model_details(id.clone())
            .await
            .expect(format!("Failed to get model details for {model_id}").as_str());

        civit_client
            .download_specific_resource_for_model(model, oid)
            .await.unwrap();
    } else {
        let results = join_all(
            ids.iter_mut()
                .map(|id| async {
                    let civit_client = civit.clone();
                    let model_id = id.clone();

                    let model = civit_client
                        .get_model_details(id.clone())
                        .await
                        .expect(format!("Failed to get model details for {model_id}").as_str());
                    model
                })
                .collect::<Vec<_>>(),
        )
        .await;

        res.extend(results);

        join_all(
            res.iter()
                .map(|model| async {
                    let m = model.clone();
                    let civit_client = civit.clone();
                    civit_client
                        .download_latest_resource_for_model(m, all)
                        .await
                })
                .collect::<Vec<_>>(),
        )
        .await;
    }
}
