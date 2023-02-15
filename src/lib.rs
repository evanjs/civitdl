#![feature(result_option_inspect)]

use reqwest;
mod model;
use anyhow::anyhow;
use model::model::Model;
use tracing;
use tracing::debug;

#[derive(Clone, Debug)]
pub struct Civit {
    pub client: reqwest::Client,
}

impl Civit {
    pub fn new() -> Self {
        Civit {
            client: reqwest::Client::new(),
        }
    }

    pub fn get_download_folder() {
        todo!()
    }

    pub fn get_model_type_download_folder() {
        todo!();
    }

    #[tracing::instrument]
    pub async fn get_model_details(
        self,
        model_id: String,
    ) -> Result<Model, anyhow::Error> {
        let url = format!("{MAIN_API_URL}/models/{model_id}");
        match self.client
            .get(&url)
            .send()
            .await?
            .json::<Model>()
            .await
            .inspect_err(|e| debug!("Failed to parse JSON from URL: {url}. Error: {e}"))
        {
            Ok(o) => Ok(o),
            Err(e) => Err(anyhow!(e)),
        }
    }
}

const MAIN_API_URL: &'static str = &"https://civitai.com/api/v1";
