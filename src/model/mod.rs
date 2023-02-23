pub(crate) mod model_version;

use serde_derive::Deserialize;
use serde_derive::Serialize;

use std::fmt;

use crate::model::model_version::ModelVersion;

#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_field: String,
    pub poi: Option<bool>,
    pub nsfw: Option<bool>,
    pub allow_no_credit: Option<bool>,
    pub allow_commercial_use: Option<String>,
    pub allow_derivatives: Option<bool>,
    pub allow_different_license: Option<bool>,
    pub creator: Option<Creator>,
    pub tags: Option<Vec<serde_json::Value>>,
    pub model_versions: Vec<ModelVersion>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub username: Option<String>,
    pub image: Option<String>,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Model {}: {} ({})", self.id, self.name, self.type_field)
    }
}
