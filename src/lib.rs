#![feature(result_option_inspect)]
#![feature(const_option)]
use reqwest;
use reqwest::{cookie::Jar, Url};
mod model;
use anyhow::anyhow;
use directories;
use futures::StreamExt;
use model::model::Model;
use normpath::{self, PathExt};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use strum;
use strum::{AsRefStr, EnumString};
use tracing::{self, instrument};
use tracing::{debug, info, trace};

use indicatif::{ProgressBar, ProgressStyle};

#[tracing::instrument]
pub fn get_download_folder_from_model_type(path: PathBuf, model_type: ModelType) -> PathBuf {
    info!("Attempting to determine download folder for model type: {model_type:?}");
    let leaf_dir = match model_type {
        ModelType::Model | ModelType::Checkpoint => "models/Stable-diffusion",
        ModelType::Lora => "models/Lora",
        ModelType::TextualInversion => "embeddings",
        ModelType::Hypernetwork => "models/hypernetwork",
        ModelType::AestheticGradient => "models/aesthetic_embeddings",
    };
    path.join(leaf_dir).normalize().unwrap().into_path_buf()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    api_key: Option<String>,
    stable_diffusion_base_directory: PathBuf,
    stable_diffusion_fallback_directory: PathBuf,
    token: Option<String>,
}

fn default_stable_diffusion_fallback_directory() -> PathBuf {
    let user_dirs = directories::UserDirs::new().unwrap();
    let downloads_directory = user_dirs.download_dir();
    downloads_directory
        .unwrap()
        .to_path_buf()
        .join("Stable-diffusion".to_string())
        .to_path_buf()
}

impl Config {
    #[tracing::instrument]
    pub fn new(
        api_key: Option<String>,
        token: Option<String>,
        stable_diffusion_base_directory: &str,
        stable_diffusion_fallback_directory: &str,
    ) -> Self {
        Self {
            api_key,
            token,
            stable_diffusion_base_directory: PathBuf::from(stable_diffusion_base_directory),
            stable_diffusion_fallback_directory: PathBuf::from(stable_diffusion_fallback_directory),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_key: None,
            token: None,
            stable_diffusion_fallback_directory: default_stable_diffusion_fallback_directory(),
            stable_diffusion_base_directory: default_stable_diffusion_fallback_directory(),
        }
    }
}

#[derive(AsRefStr, Debug, EnumString)]
pub enum ModelType {
    Lora,
    Model,
    Checkpoint,
    #[strum(serialize = "Textual Inversion")]
    TextualInversion,
    Hypernetwork,
    #[strum(serialize = "Aesthetic Gradient")]
    AestheticGradient,
}

#[derive(Clone, Debug)]
pub struct Civit {
    pub client: reqwest::Client,
    pub config: Option<Config>,
}

impl Civit {
    #[tracing::instrument(level = "debug")]
    pub fn new(maybe_config: Option<Config>) -> Self {
        let jar = Jar::default();
        if let Some(a) = maybe_config.clone() {
            if let Some(t) = a.token {
                let url = "https://civitai.com".parse::<Url>().unwrap();
                let token = format!("__Secure-civitai-token={};", t).to_string();
                let cookie = format!(
                    "{} Domain=.civitai.com; Path=/; HttpOnly; Secure; SameSite=Lax",
                    token.clone()
                )
                .to_string();
                jar.add_cookie_str(cookie.as_str(), &url);
                trace!("Added cookie {} to jar", cookie);
            }
        }

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .cookie_provider(Arc::new(jar))
            .build()
            .unwrap();
        debug!("Constructed client: {:#?}", client);

        Civit {
            client,
            config: maybe_config.or(None),
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn get_model_details(self, model_id: String) -> Result<Model, anyhow::Error> {
        let url = format!("{MAIN_API_URL}/models/{model_id}");
        match self
            .client
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

    #[tracing::instrument(level = "debug")]
    pub async fn download_latest_resource_for_model(self, model: Model) -> anyhow::Result<String> {
        let first = model.model_versions.first().unwrap();
        let files = first.files.first().unwrap();
        let f = ModelType::from_str(files.type_field.as_str()).unwrap();
        println!("Attempting to download {model:?} ...");
        self.download_file(&first.download_url, f).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn download_file(self, url: &str, model_type: ModelType) -> anyhow::Result<String> {
        trace!("Client: {:#?}", self.client);
        let path = self.config.clone().unwrap().stable_diffusion_base_directory;
        let model_directory = get_download_folder_from_model_type(path.clone(), model_type);
        let result = self
            .client
            .get(url)
            .send()
            .await
            .or(Err(anyhow!("Failed to GET from '{}'", &url)))?;

        let headers = result.headers();
        trace!("Headers: {:#?}", &headers);

        let content_disposition = result
            .headers()
            .iter()
            .find(|(x, _)| x.as_str().eq("content-disposition"))
            .ok_or(anyhow!("Failed to get content disposition from '{}'", &url))?
            .1
            .to_str()
            .ok()
            .unwrap();
        let filename = content_disposition
            .split("filename=")
            .last()
            .unwrap()
            .replace('"', "");
        trace!("Content Disposition for {:?}: {:?}", &url, &filename);

        let final_path = model_directory.join(filename);
        debug!("Final path: {}", final_path.to_string_lossy());

        if final_path.exists() {
            let message = format!(
                "{:?} already exists! Not downloading...",
                final_path.to_string_lossy()
            );
            info!(message);
            return Err(anyhow!(message));
        }

        let total_size = result
            .content_length()
            .ok_or(anyhow!("Failed to get content length from '{}'", &url))?;

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
            .progress_chars("#>-"));

        // download chunks
        let mut file = File::create(&final_path).or(Err(anyhow!(
            "Failed to create file '{}'",
            final_path.to_string_lossy()
        )))?;
        let mut downloaded: u64 = 0;
        let mut stream = result.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.or(Err(anyhow!("")))?;
            file.write_all(&chunk)
                .or(Err(anyhow!("Error while writing to file")))?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new)
        }

        pb.finish_with_message(format!(
            "Downloaded {} to {}",
            url,
            final_path.to_string_lossy()
        ));
        Ok("".to_string())
    }
}

const MAIN_API_URL: &'static str = &"https://civitai.com/api/v1";
