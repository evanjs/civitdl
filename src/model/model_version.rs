use serde_derive::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub nsfw: Option<bool>,
    pub poi: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersion {
    pub id: i64,
    pub model_id: i64,
    pub name: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub trained_words: Vec<String>,
    pub base_model: Option<String>,
    pub early_access_time_frame: Option<i64>,
    pub description: Option<String>,
    pub files: Option<Vec<ResourceFile>>,
    pub images: Option<Vec<Image>>,
    pub model: Option<Model>,
    pub download_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFile {
    pub name: String,
    pub id: i64,
    #[serde(rename = "sizeKB")]
    pub size_kb: Option<f64>,
    #[serde(rename = "type")]
    pub type_field: String,
    pub format: Option<String>,
    pub pickle_scan_result: Option<String>,
    pub pickle_scan_message: Option<String>,
    pub virus_scan_result: Option<String>,
    pub scanned_at: Option<String>,
    pub hashes: Option<Hashes>,
    pub download_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hashes {
    #[serde(rename = "AutoV1")]
    pub auto_v1: Option<String>,
    #[serde(rename = "AutoV2")]
    pub auto_v2: Option<String>,
    #[serde(rename = "SHA256")]
    pub sha256: Option<String>,
    #[serde(rename = "CRC32")]
    pub crc32: Option<String>,
    #[serde(rename = "BLAKE3")]
    pub blake3: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NSFW {
    None,
    Soft,
    Mature,
    X
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub url: String,
    pub nsfw: Option<NSFW>,
    pub width: i64,
    pub height: i64,
    pub hash: Option<String>,
    pub meta: Option<Meta>,
    pub generation_process: Option<String>,
    pub tags: Option<Vec<Option<serde_json::Value>>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(rename = "ENSD")]
    pub ensd: Option<String>,
    #[serde(rename = "Size")]
    pub size: Option<String>,
    pub seed: Option<i64>,
    pub steps: Option<i64>,
    pub prompt: Option<String>,
    pub sampler: Option<String>,
    pub cfg_scale: Option<f64>,
    #[serde(rename = "Clip skip")]
    pub clip_skip: Option<String>,
    pub resources: Option<Vec<Resource>>,
    #[serde(rename = "Model hash")]
    pub model_hash: Option<String>,
    #[serde(rename = "Hires steps")]
    pub hires_steps: Option<String>,
    #[serde(rename = "Hires upscale")]
    pub hires_upscale: Option<String>,
    #[serde(rename = "AddNet Enabled")]
    pub add_net_enabled: Option<String>,
    #[serde(rename = "AddNet Model 1")]
    pub add_net_model_1: Option<String>,
    #[serde(rename = "Hires upscaler")]
    pub hires_upscaler: Option<String>,
    pub negative_prompt: Option<String>,
    #[serde(rename = "AddNet Module 1")]
    pub add_net_module_1: Option<String>,
    #[serde(rename = "AddNet Weight A 1")]
    pub add_net_weight_a_1: Option<String>,
    #[serde(rename = "AddNet Weight B 1")]
    pub add_net_weight_b_1: Option<String>,
    #[serde(rename = "Denoising strength")]
    pub denoising_strength: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub hash: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub resource_type: Option<String>,
    pub weight: Option<f64>,
}
