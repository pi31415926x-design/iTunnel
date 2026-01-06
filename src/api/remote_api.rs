use log::debug;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct DeviceVerifyRequest {
    device_id: String,
}

#[derive(Deserialize, Debug)]
pub struct DeviceVerifyResponse {
    pub expire: i64,
    pub status: String,
    pub token: String,
}

#[derive(Serialize)]
struct DesktopCfgRequest {
    device_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubscribeRequest {
    pub device_id: String,
    pub subscribe_type: Option<String>, // week / month / 3month
    pub price: Option<f32>,
    pub device_type: Option<String>, // desktop / mobile
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscriptionPlan {
    pub name: String,
    pub duration: String,
    pub price: f32,
    pub features: Vec<String>,
    pub popular: bool,
    pub cta: String,
}

#[derive(Debug, Default)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    pub token: Mutex<Option<String>>,
}

impl ApiClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: "https://iedux.pro/wgx".to_string(),
            token: Mutex::new(None),
        }
    }

    /// 验证设备是否有效
    /// 1) /device_verify, request body {"device_id": machine-uuid}
    /// 成功后会将 token 保存到 self.token
    pub async fn verify_device(&self, device_id: &str) -> Result<bool, String> {
        let url = format!("{}/device_verify", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&DeviceVerifyRequest {
                device_id: device_id.to_string(),
            })
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Server returned error: {}", resp.status()));
        }

        let verify_resp: DeviceVerifyResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        debug!(
            "Verify Response for {}: status={}",
            device_id, verify_resp.status
        );

        // 保存 token
        {
            let mut token_lock = self.token.lock().unwrap();
            *token_lock = Some(verify_resp.token.clone());
        }

        // 解析 JWT 获取 claims
        let claims = super::jwt::parse_jwt(&verify_resp.token)
            .map_err(|e| format!("JWT parse failed: {}", e))?;

        debug!("JWT Claims: {:?}", claims);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(claims.exp > now)
    }

    /// 获取桌面配置
    /// 2) /api/get_desktop_cfg, request body {"device_id": machine-uuid}
    /// 自动注入 Authorization Bearer token
    pub async fn get_desktop_cfg(&self, device_id: &str) -> Result<String, String> {
        let url = format!("{}/api/get_desktop_cfg", self.base_url);

        let token_opt = {
            let token_lock = self.token.lock().unwrap();
            token_lock.clone()
        };

        let mut rb = self.client.post(&url).json(&DesktopCfgRequest {
            device_id: device_id.to_string(),
        });

        if let Some(token) = token_opt {
            rb = rb.bearer_auth(token);
        }

        let resp = rb
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Server returned error: {}", resp.status()));
        }

        resp.text()
            .await
            .map_err(|e| format!("Failed to get body: {}", e))
    }

    /// 转发订阅请求
    /// 3) /api/subscribe_req, request body {"device_id": machine-uuid, "subscribe_type": ..., "price": ..., "device_type": "desktop"}
    /// 自动注入 Authorization Bearer token
    pub async fn subscribe_req(&self, req: &SubscribeRequest) -> Result<String, String> {
        // 在发送请求之前，先进行设备校验以确保 token 有效
        match self.verify_device(&req.device_id).await {
            Ok(true) => debug!("Device verified successfully before subscribe_req"),
            Ok(false) => return Err("Device is not activated or expired".to_string()),
            Err(e) => return Err(format!("Device verification failed: {}", e)),
        }

        let url = format!("{}/subscribe_req", self.base_url);

        let token_opt = {
            let token_lock = self.token.lock().unwrap();
            token_lock.clone()
        };

        debug!("Sending subscribe request: {:?}", req);
        let mut rb = self.client.post(&url).json(req);

        if let Some(token) = token_opt {
            rb = rb.bearer_auth(token);
        }

        let resp = rb
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Server returned error: {}", resp.status()));
        }

        resp.text()
            .await
            .map_err(|e| format!("Failed to get body: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::machine_id::get_machine_id;

    #[tokio::test]
    async fn test_verify_and_cfg() {
        crate::logging::init();
        let client = ApiClient::new();
        let device_id = get_machine_id().unwrap();
        debug!("current device uuid is {}", device_id);
        let result = client.verify_device(&device_id).await;
        println!("Verify Result: {:?}", result);

        if let Ok(true) = result {
            let cfg = client.get_desktop_cfg(&device_id).await;
            println!("Desktop Config: {:?}", cfg);
        }
    }
}
