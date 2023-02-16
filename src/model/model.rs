use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;
use std::fmt;

use crate::model::model_version::ModelVersion;

#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: i64,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub poi: bool,
    pub nsfw: bool,
    pub allow_no_credit: bool,
    pub allow_commercial_use: String,
    pub allow_derivatives: bool,
    pub allow_different_license: bool,
    pub creator: Creator,
    pub tags: Vec<String>,
    pub model_versions: Vec<ModelVersion>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub username: String,
    pub image: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub hash: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_field: String,
    pub weight: Option<Value>,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Model {}: {} ({})", self.id, self.name, self.type_field)
    }
}
