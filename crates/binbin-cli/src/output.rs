use anyhow::{Context, Result};
use serde_json::Value;
pub(super) fn print(value: &Value, json_output: bool) {
    if json_output {
        println!("{value}")
    } else {
        println!("{value:#}")
    }
}
pub(super) fn origin(s: &str) -> Result<String> {
    let url = url::Url::parse(s).context("Invalid server URL")?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "Expected an HTTP(S) URL without credentials"
    );
    Ok(url.origin().ascii_serialization())
}
