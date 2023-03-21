#![feature(result_option_inspect)]
#![feature(const_option)]
#![feature(unwrap_infallible)]

use reqwest::{cookie::Jar, Url};
pub mod model;
use anyhow::anyhow;
use futures::{future::join_all, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use model::model_version::ModelVersion;
use model::model_version::ResourceFile;
use model::Model;
use normpath::{self, PathExt};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use strum::{AsRefStr, EnumString};
use tracing::{debug, error, trace, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    api_key: Option<String>,
    stable_diffusion_base_directory: PathBuf,
    stable_diffusion_fallback_directory: PathBuf,
    token: Option<String>,
    model_format: ModelFormat,
    resource_type: ResourceType,
}

#[derive(AsRefStr, Debug, Serialize, Deserialize, Clone, EnumString, PartialEq, Default)]
pub enum ResourceType {
    Model,
    #[strum(serialize = "Pruned Model")]
    #[default]
    PrunedModel,
    #[strum(serialize = "Training Data")]
    TrainingData,
    Archive,
    Config,
    Unknown
}

#[derive(AsRefStr, Debug, Serialize, Deserialize, Clone, EnumString, PartialEq, Default)]
pub enum ModelFormat {
    #[default]
    SafeTensor,
    PickleTensor,
    Other,
    Unknown
}

fn default_stable_diffusion_fallback_directory() -> PathBuf {
    let user_dirs = directories::UserDirs::new().unwrap();
    let downloads_directory = user_dirs.download_dir();
    downloads_directory
        .unwrap()
        .to_path_buf()
        .join("Stable-diffusion")
}

pub fn get_config_directory() -> PathBuf {
    let project_dirs = directories::ProjectDirs::from("io", "evanjs", "civitdl").unwrap();
    let config_dir = project_dirs.config_dir();
    debug!(config_dir =? &config_dir, message = "Checking if config directory exists");
    if !config_dir.exists() {
        debug!(config_dir =? &config_dir, message =? "Attempting to create config directory");

        let created = std::fs::create_dir_all(config_dir).ok();
        if created.is_some() {
            debug!(config_directory =? &config_dir, message="Created config directory");
            config_dir.into()
        } else {
            let exe_dir = std::env::current_exe().unwrap().parent().unwrap().into();
            debug!(exe_dir =? &exe_dir, message =? "Failed to find config directory.\nFalling back to executable directory");
            exe_dir
        }
    } else {
        debug!(config_dir =? &config_dir, message = "Found existing config directory");
        config_dir.into()
    }
}

impl Config {
    #[tracing::instrument(skip_all)]
    pub fn new(
        api_key: Option<String>,
        token: Option<String>,
        stable_diffusion_base_directory: &str,
        stable_diffusion_fallback_directory: &str,
        model_format: &str,
        resource_type: &str,
    ) -> Self {
        Self {
            api_key,
            token,
            stable_diffusion_base_directory: PathBuf::from(stable_diffusion_base_directory),
            stable_diffusion_fallback_directory: PathBuf::from(stable_diffusion_fallback_directory),
            model_format: ModelFormat::from_str(model_format).unwrap_or_default(),
            resource_type: ResourceType::from_str(resource_type).unwrap_or_default(),
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
            model_format: ModelFormat::default(),
            resource_type: ResourceType::default(),
        }
    }
}

#[derive(AsRefStr, Debug, EnumString)]
pub enum ModelType {
    #[strum(serialize = "LORA")]
    Lora,
    Model,
    Checkpoint,
    TextualInversion,
    Hypernetwork,
    AestheticGradient,
    Poses,
    Unknown,
    LoCon,
    Wildcards
}

#[derive(Clone, Debug)]
pub struct Civit {
    pub client: reqwest::Client,
    pub config: Option<Config>,
    pub multi_progress: MultiProgress,
}

impl Civit {
    #[tracing::instrument(level = "trace")]
    pub fn new(maybe_config: Option<Config>) -> Self {
        let jar = Jar::default();
        if let Some(a) = maybe_config.clone() {
            if let Some(t) = a.token {
                let url = "https://civitai.com".parse::<Url>().unwrap();
                let token = format!("__Secure-civitai-token={};", t);
                let cookie = format!(
                    "{} Domain=.civitai.com; Path=/; HttpOnly; Secure; SameSite=Lax",
                    token
                );
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

        let multi_progress = MultiProgress::new();

        Civit {
            client,
            config: maybe_config.or(None),
            multi_progress,
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn get_optimal_file_from_preferred_model_format(
        self,
        model_version: ModelVersion,
    ) -> Result<Option<ResourceFile>, anyhow::Error> {
        debug!(
            "Attempting to get optimal file from model version {}",
            &model_version.id
        );
        if let Some(vs) = model_version.files {
            tracing::info!(config =? &self.config);
            let preferred_model_format = self.config.clone().unwrap().model_format;
            debug!("Preferred model format: {:?}", &preferred_model_format);

            let preferred_resource_type = self.config.clone().unwrap().resource_type;
            debug!("Preferred resource type: {:?}", &preferred_resource_type);
            debug!(files =? vs);

            let primary = vs
                .iter()
                .filter(|v| v.format.is_some())
                .find(|v| {
                    debug!(resource_file =? v, "Parsing model version");
                    let found_model_format =
                        ModelFormat::from_str(v.format.clone().unwrap().as_str())
                            .unwrap_or(ModelFormat::Unknown);
                    let found_resource_type =
                        ResourceType::from_str(&v.type_field).unwrap_or(ResourceType::Unknown);
                    debug!(
                        "Found {:?} model of format {:?}",
                        &found_resource_type, &found_model_format
                    );
                    let okay = preferred_model_format.eq(&found_model_format)
                        && preferred_resource_type.eq(&found_resource_type);
                    debug!(
                        "Need to ensure {:?} is equal to {:?}",
                        &found_model_format,
                        preferred_model_format
                    );
                    debug!(
                        "Need to ensure {:?} is equal to {:?}",
                        &found_resource_type,
                        preferred_resource_type
                    );
                    debug!(
                        "Is {:?} okay? ({:?})({:?}) -- {} ",
                        &v.id, &found_model_format, &found_resource_type, okay
                    );
                    okay
                }).cloned();
            let alt = vs
                .iter()
                .filter(|v| v.format.is_some())
                .find(|v| {
                    let found_model_format =
                        ModelFormat::from_str(v.format.clone().unwrap().as_str())
                            .unwrap_or(ModelFormat::Unknown);
                    let found_resource_type =
                        ResourceType::from_str(&v.type_field).unwrap_or(ResourceType::Unknown);
                    let alt = preferred_model_format.eq(&found_model_format)
                        || preferred_resource_type.eq(&found_resource_type);
                    debug!(
                        "[Alt] Need to ensure {:?} is equal to {:?}",
                        &found_model_format,
                        preferred_model_format
                    );
                    debug!(
                        "[Alt] Need to ensure {:?} is equal to {:?}",
                        &found_resource_type,
                        preferred_resource_type
                    );
                    debug!(
                        "[Alt] Is {:?} okay? ({:?})({:?}) -- {} ",
                        &v.id, &found_model_format, &found_resource_type, alt
                    );
                    alt
                }).cloned();
            debug!(primary =? &primary, alt =? &alt);
            if primary.is_some() {
                return Ok(primary)
            } else {
                if alt.is_some() {
                  debug!(alt_type =? &alt);
                  return Ok(Some(alt.unwrap_or_default()))
                } else {
                    Ok(Some(vs.clone().first().clone().unwrap().clone()))
                }
            }
        } else {
            let first = model_version.files.expect("yo wtf").first().cloned();
            match first {
                Some(s) => Ok(Some(s)),
                None => Err(anyhow!("Failed to get first model thing")),
            }
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
            .or(Err(anyhow!(
                "Failed to resolve model version {}",
                model_version.id
            )));
        match version {
            Ok(v) => {
                trace!("Model version: {:#?}", v);
                let model = v.model.unwrap();
                trace!("Model: {:#?}", model);
                trace!("Version Type: {:#?}", model.type_field);
                trace!(
                    "Attempting to resolve type for type_field: {:#?}",
                    model.type_field
                );
                let resolved_type = ModelType::from_str(model.type_field.as_str()).map_err(|e|error!(error =? e, "Failed to resolve model type")).unwrap();
                let resolved_path = self.get_download_folder_from_model_type(path, resolved_type);
                debug!("Resolved path: {:#?}", resolved_path);
                Ok(resolved_path)
            }
            Err(e) => Err(e),
        }
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
            ModelType::Lora | ModelType::LoCon => "models/Lora",
            ModelType::TextualInversion => "embeddings",
            ModelType::Hypernetwork => "models/hypernetworks",
            ModelType::AestheticGradient => "models/aesthetic_embeddings",
            ModelType::Poses => "models/poses",
            ModelType::Unknown => "downloads",
            ModelType::Wildcards => "downloads/wildcards"
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
    pub async fn download_latest_resource_for_model(
        self,
        model: Model,
        all: bool,
    ) -> anyhow::Result<()> {
        let versions = model.clone().model_versions;
        match all {
            false => {
                let first = versions.first().unwrap().to_owned().clone();
                self.clone().download_file(&first, model.clone()).await
            }
            true => {
                join_all(
                    versions
                        .iter()
                        .map(|v| async { self.clone().download_file(v, model.clone()).await })
                        .collect::<Vec<_>>(),
                )
                .await;
                Ok(())
            }
        }
    }

    pub async fn check_if_file_exists_and_matches_hash(
        self,
        path: PathBuf,
        file: ResourceFile,
    ) -> anyhow::Result<bool> {
        let file_exists = path.exists();
        if !file_exists {
            return Ok(false);
        }

        let size1 = file.size_kb.unwrap();
        let size2 = path.metadata().unwrap().len() as f64 / 1024.0;
        debug!("Checking sizes {} and {}...", &size1, &size2);

        let same = file_exists && size1.eq(&size2);
        debug!("Same: {}", &same);
        Ok(same)
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn download_specific_resource_for_model(
        self,
        model: Model,
        oid: String,
    ) -> anyhow::Result<()> {
        let versions = model.clone().model_versions;
        let target = versions
            .iter()
            .find(|version| version.id.to_string().eq(&oid))
            .ok_or(Err(anyhow!(
                "Failed to find model version {} for model {}",
                oid,
                model.id.to_string()
            )));
        match target {
            Ok(t) => self.clone().download_file(t, model.clone()).await,
            Err(e) => e,
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn download_file(
        self,
        model_version: &ModelVersion,
        model: Model,
    ) -> anyhow::Result<()> {
        let path = &self
            .config
            .clone()
            .unwrap()
            .stable_diffusion_base_directory
            .clone();

        let alt = model_version
            .clone()
            .files
            .unwrap()
            .first()
            .unwrap()
            .clone();
        let target_file = self
            .clone()
            .get_optimal_file_from_preferred_model_format(model_version.clone())
            .await?
            .unwrap_or(alt);
        trace!("Target file: {:?}", &target_file);

        let url = &target_file.download_url.clone();
        trace!("URL: {}", &url);

        let model_directory = self
            .clone()
            .get_download_folder_from_model_version(path.clone(), model_version.clone())
            .await?;
        let result = self
            .client
            .get(url.clone())
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


        let final_path = model_directory.join(&filename);
        debug!("Final path: {}", final_path.to_string_lossy());
        

        let same = self
            .clone()
            .check_if_file_exists_and_matches_hash(final_path.clone(), target_file.clone())
            .await?;
        if same {
            let message = format!(
                "{:?} already exists! Not downloading...",
                final_path.to_string_lossy()
            );
            warn!("{}", message);
            return Err(anyhow!(message));
        }

        let total_size = result
            .content_length()
            .ok_or(anyhow!("Failed to get content length from '{}'", &url))?;

        let check_format = ModelFormat::from_str(&target_file.clone().format.unwrap_or_default()).unwrap_or(ModelFormat::Other);
        let check_type = ResourceType::from_str(&target_file.clone().type_field).unwrap_or(ResourceType::Unknown);
        let pb = self.multi_progress.add(ProgressBar::new(total_size)
            .with_prefix(filename)
            .with_message(format!("Attempting to download version {} for {model:?} (format: {:?}/{:?}) ...", model_version.id, check_type, check_format))
            .with_style(ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{prefix}] [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
                .progress_chars("#>-")))
            .with_finish(indicatif::ProgressFinish::WithMessage(format!(
                        "Downloaded {} ({:?}/{:?}) to {}",
                        url,
                        check_type,
                        check_format,
                        final_path.to_string_lossy()
            ).into()));

        // download chunks
        let mut file = File::create(&final_path).or(Err(anyhow!(
            "Failed to create file '{}'",
            final_path.to_string_lossy()
        )))?;
        let mut downloaded: u64 = 0;
        let mut stream = result.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.or(Err(anyhow!("Failed to read chunk from stream")))?;
            file.write_all(&chunk)
                .or(Err(anyhow!("Error while writing to file")))?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new)
        }

        Ok(())
    }
}

const MAIN_API_URL: &str = "https://civitai.com/api/v1";
