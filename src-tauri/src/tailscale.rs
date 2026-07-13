use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TailscaleStatus {
    pub connected: bool,
    pub dns_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CliStatus {
    #[serde(rename = "BackendState")]
    backend_state: Option<String>,
    #[serde(rename = "Self")]
    self_node: Option<CliSelf>,
}

#[derive(Debug, Deserialize)]
struct CliSelf {
    #[serde(rename = "DNSName")]
    dns_name: Option<String>,
}

pub async fn status() -> TailscaleStatus {
    let output = match Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            return TailscaleStatus {
                connected: false,
                dns_name: None,
                error: Some(format!("Tailscale is not available: {error}")),
            }
        }
    };

    if !output.status.success() {
        return TailscaleStatus {
            connected: false,
            dns_name: None,
            error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        };
    }

    match serde_json::from_slice::<CliStatus>(&output.stdout) {
        Ok(value) => {
            let dns_name = value
                .self_node
                .and_then(|node| node.dns_name)
                .map(|name| name.trim_end_matches('.').to_string());
            TailscaleStatus {
                connected: value.backend_state.as_deref() == Some("Running") && dns_name.is_some(),
                dns_name,
                error: None,
            }
        }
        Err(error) => TailscaleStatus {
            connected: false,
            dns_name: None,
            error: Some(format!("Could not read Tailscale status: {error}")),
        },
    }
}
