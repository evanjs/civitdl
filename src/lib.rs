#![feature(result_option_inspect)]
#![feature(const_option)]
use reqwest;
use reqwest::{cookie::Jar, Url};
mod model;
use anyhow::anyhow;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use model::model::Model;
use model::model_version::ModelVersion;
use normpath::{self, PathExt};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::{PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use strum;
use strum::{AsRefStr, EnumString};
use tracing::{self};
use tracing::{debug, info, trace};

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[tracing::instrument(skip_all)]
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
    #[strum(serialize = "LORA")]
    Lora,
    //#[strum(serialize = "model")]
    Model,
    //#[strum(serialize = "checkpoint")]
    Checkpoint,
    //#[strum(serialize = "textual inversion")]
    TextualInversion,
    //#[strum(serialize = "hypernetwork")]
    Hypernetwork,
    //#[strum(serialize = "aesthetic gradient")]
    AestheticGradient,
}

#[derive(Clone, Debug)]
pub struct Civit {
    pub client: reqwest::Client,
    pub config: Option<Config>,
}

impl Civit {
    #[tracing::instrument(level = "trace")]
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

    #[tracing::instrument(level = "trace")]
    pub async fn get_download_folder_from_model_version(
        self,
        path: PathBuf,
        model_version: ModelVersion,
    ) -> anyhow::Result<PathBuf> {
        trace!(
            "Attempting to determine download folder for model version: {:?}",
            model_version.id
        );
        let version = self
            .clone()
            .get_model_version_details(model_version.id)
            .await
            .unwrap();
        trace!("Model version: {:#?}", version);
        let model = version.model.unwrap();
        trace!("Model: {:#?}", model);
        trace!("Version Type: {:#?}", model.type_field);
        trace!(
            "Attempting to resolve type for type_field: {:#?}",
            model.type_field
        );
        let resolved_type = ModelType::from_str(model.type_field.as_str()).unwrap();
        let resolved_path = self.get_download_folder_from_model_type(path, resolved_type);
        info!("Resolved path: {:#?}", resolved_path);
        Ok(resolved_path)
    }

    #[tracing::instrument(level = "trace")]
    pub fn get_download_folder_from_model_type(
        &self,
        path: PathBuf,
        model_type: ModelType,
    ) -> PathBuf {
        debug!("Attempting to determine download folder for model type: {model_type:?}");
        let leaf_dir = match model_type {
            ModelType::Model | ModelType::Checkpoint => "models/Stable-diffusion",
            ModelType::Lora => "models/Lora",
            ModelType::TextualInversion => "embeddings",
            ModelType::Hypernetwork => "models/hypernetworks",
            ModelType::AestheticGradient => "models/aesthetic_embeddings",
        };
        trace!("Leaf dir: {:#?}", leaf_dir);
        let final_path = path.join(leaf_dir).normalize().unwrap().into_path_buf();
        trace!("Path buf: {:#?}", final_path);
        final_path
    }

    #[tracing::instrument(level = "trace")]
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

    #[tracing::instrument(level = "trace")]
    pub async fn get_model_version_details(
        self,
        model_version_id: i64,
    ) -> Result<ModelVersion, anyhow::Error> {
        let url = format!("{MAIN_API_URL}/model-versions/{model_version_id}");
        debug!("URL: {:#?}", url);
        match self
            .client
            .get(&url)
            .send()
            .await?
            .json::<ModelVersion>()
            .await
            .inspect_err(|e| debug!("Failed to parse JSON from URL: {url}. Error: {e}"))
        {
            Ok(o) => Ok(o),
            Err(e) => Err(anyhow!(e)),
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn download_latest_resource_for_model(self, model: Model) -> anyhow::Result<String> {
        //
        let first = model
            .clone()
            .model_versions
            .first()
            .unwrap()
            .to_owned()
            .clone();
        println!("Attempting to download {model:?} ...");

        self.clone().download_file(&first).await
    }

    #[tracing::instrument(level = "trace")]
    pub async fn download_file(self, model_version: &ModelVersion) -> anyhow::Result<String> {
        let path = &self
            .config
            .clone()
            .unwrap()
            .stable_diffusion_base_directory
            .clone();
        let url = &model_version.clone().download_url.clone();
        let model_directory = self
            .clone()
            .get_download_folder_from_model_version(path.clone(), model_version.clone())
            .await?;
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
            println!("{}", message);
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
