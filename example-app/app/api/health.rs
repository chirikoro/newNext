// Health check API endpoint (/api/health)
// Returns JSON: { "status": "ok" }

use serde_json::json;

pub async fn handler() -> serde_json::Value {
    json!({
        "status": "ok",
        "framework": "hayabusa",
        "version": "0.1.0"
    })
}
