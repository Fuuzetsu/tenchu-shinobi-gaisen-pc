use std::time::Duration;
use tenchu_assets::Result;

pub fn get(url: &str, timeout: Duration) -> Result<ureq::Body> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .user_agent(concat!("tenchu-pc/", env!("CARGO_PKG_VERSION")))
        .max_redirects(5)
        .timeout_global(Some(timeout))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(15)))
        .timeout_per_read(Some(Duration::from_secs(15)))
        .build()
        .into();
    agent
        .get(url)
        .call()
        .map(|response| response.into_body())
        .map_err(|e| e.to_string())
}
