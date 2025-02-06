use anyhow::bail;
use serde::{Deserialize, Serialize};
use spin_sdk::http::{Method, Request, Response};

pub struct SlackClient {
    token: String,
}

impl SlackClient {
    pub fn from_variable() -> anyhow::Result<Self> {
        let token = spin_sdk::variables::get("slack_token")?;
        Ok(Self { token })
    }

    /// Post a message to a channel, optionally threaded under the given root message.
    ///
    /// Returns the created message's timestamp.
    pub async fn post_message(
        &self,
        channel: String,
        text: String,
        thread_ts: Option<String>,
    ) -> anyhow::Result<String> {
        let req = RequestBody {
            channel,
            text,
            thread_ts,
            ..Default::default()
        };
        let resp = self.post("chat.postMessage", req).await?;
        Ok(resp.ts)
    }

    /// Update a message.
    pub async fn update(
        &self,
        channel: String,
        message_ts: String,
        text: String,
    ) -> anyhow::Result<()> {
        let req = RequestBody {
            channel,
            text,
            ts: Some(message_ts),
            ..Default::default()
        };
        self.post("update", req).await?;
        Ok(())
    }

    async fn post(&self, path_suffix: &str, body: RequestBody) -> anyhow::Result<ResponseBody> {
        let body = serde_json::to_vec(&body)?;
        let req = Request::builder()
            .method(Method::Post)
            .header("authorization", format!("Bearer {}", self.token))
            .header("content-type", "application/json")
            .uri(format!("https://slack.com/api/{path_suffix}"))
            .body(body)
            .build();
        let resp: Response = spin_sdk::http::send(req).await?;
        let resp_body: ResponseBody = serde_json::from_slice(resp.body())?;
        if !resp_body.ok {
            bail!("API error: {}", resp_body.error)
        }
        Ok(resp_body)
    }
}

// Lazily combine fields for postMessage and update
#[derive(Default, Serialize)]
struct RequestBody {
    channel: String,
    text: String,
    // Message to update
    #[serde(skip_serializing_if = "Option::is_none")]
    ts: Option<String>,
    // Root message to thread under
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ResponseBody {
    ok: bool,
    error: String,
    channel: String,
    ts: String,
}
