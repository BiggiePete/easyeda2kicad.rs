use crate::error::{Error, Result};
use serde::Deserialize;
use serde_json::Value;

const API_ENDPOINT: &str = "https://easyeda.com/api/products/{lcsc_id}/components?version=6.4.19.5";
const ENDPOINT_3D_MODEL: &str = "https://modules.easyeda.com/3dmodel/{uuid}";
const ENDPOINT_3D_MODEL_STEP: &str = "https://modules.easyeda.com/qAxj6KHrDKw4blvCG8QJPs7Y/{uuid}";
const USER_AGENT: &str = "easyeda2kicad_rs/0.1.0";

#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub code: Option<i32>,
    pub message: Option<String>,
}

pub struct EasyedaApi {
    client: reqwest::Client,
}

impl EasyedaApi {
    pub fn new() -> Self {
        EasyedaApi {
            client: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .unwrap(),
        }
    }

    pub async fn get_cad_data_of_component(&self, lcsc_id: &str) -> Result<Value> {
        let url = API_ENDPOINT.replace("{lcsc_id}", lcsc_id);
        let res = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<ApiResponse>()
            .await?;

        if res.success {
            res.result.ok_or_else(|| {
                Error::MissingData("API response missing 'result' field".to_string())
            })
        } else {
            let _msg = res
                .message
                .unwrap_or_else(|| "Unknown API error".to_string());
            Err(Error::MissingData("API Error".to_string()))
        }
    }

    pub async fn get_raw_3d_model_obj(&self, uuid: &str) -> Result<String> {
        let url = ENDPOINT_3D_MODEL.replace("{uuid}", uuid);
        let res = self.client.get(&url).send().await?;
        if res.status().is_success() {
            Ok(res.text().await?)
        } else {
            Err(Error::MissingData(format!(
                "No raw 3D model (OBJ) found for uuid: {}",
                uuid
            )))
        }
    }

    pub async fn get_step_3d_model(&self, uuid: &str) -> Result<bytes::Bytes> {
        let url = ENDPOINT_3D_MODEL_STEP.replace("{uuid}", uuid);
        let res = self.client.get(&url).send().await?;
        if res.status().is_success() {
            Ok(res.bytes().await?)
        } else {
            Err(Error::MissingData(format!(
                "No STEP 3D model found for uuid: {}",
                uuid
            )))
        }
    }
}
