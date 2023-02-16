use serde_derive::{Serialize, Deserialize};
use serde_json::Value;
use crate::model::model::Resource;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub nsfw: bool,
    pub poi: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersion {
    pub id: i64,
    pub model_id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub trained_words: Vec<Value>,
    pub base_model: String,
    pub early_access_time_frame: i64,
    pub description: Option<String>,
    pub files: Vec<File>,
    pub images: Vec<Image>,
    pub model: Option<Model>,
    pub download_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub name: String,
    pub id: i64,
    #[serde(rename = "sizeKB")]
    pub size_kb: f64,
    #[serde(rename = "type")]
    pub type_field: String,
    pub format: String,
    pub pickle_scan_result: String,
    pub pickle_scan_message: Option<String>,
    pub virus_scan_result: String,
    pub scanned_at: Option<String>,
    pub hashes: Hashes,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub url: String,
    pub nsfw: bool,
    pub width: i64,
    pub height: i64,
    pub hash: String,
    pub meta: Option<Meta>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(rename = "ENSD")]
    pub ensd: Option<String>,
    #[serde(rename = "Size")]
    pub size: Option<String>,
    pub seed: i64,
    pub steps: i64,
    pub prompt: String,
    pub sampler: String,
    pub cfg_scale: f64,
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

