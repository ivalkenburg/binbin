use anyhow::{Context, Result, bail};
use reqwest::{Client, Response};
use serde_json::Value;
pub(super) struct Api {
    pub(super) client: Client,
    pub(super) server: String,
    pub(super) token: Option<String>,
}
impl Api {
    pub(super) async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut r = self
            .client
            .request(method, format!("{}{path}", self.server));
        if let Some(t) = &self.token {
            r = r.bearer_auth(t)
        }
        if let Some(b) = body {
            r = r.json(&b)
        }
        let response = r.send().await?;
        decode(response).await
    }
    pub(super) async fn get(&self, path: &str) -> Result<Value> {
        self.call(reqwest::Method::GET, path, None).await
    }
    pub(super) async fn post(&self, path: &str, body: Value) -> Result<Value> {
        self.call(reqwest::Method::POST, path, Some(body)).await
    }
}
pub(super) async fn decode(r: Response) -> Result<Value> {
    let status = r.status();
    let value: Value = r
        .json()
        .await
        .with_context(|| format!("{status}: Server did not return JSON"))?;
    if !status.is_success() {
        bail!(
            "{}: {}",
            status,
            value["error"].as_str().unwrap_or("Request failed")
        )
    }
    Ok(value)
}
