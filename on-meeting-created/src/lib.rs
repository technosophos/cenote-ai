use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
fn handle_on_meeting_created(req: Request) -> anyhow::Result<impl IntoResponse> {
    let body = String::from_utf8_lossy(req.body());
    let body: WebhookBody = serde_json::from_str(&body)?;
    println!("Received webhook with page ID {}, title {}", body.data.id, body.meeting_name());

    let notion_id = body.id();

    let mtg_record = MeetingInProgress {
        notion_id: notion_id.to_owned(),
        slack_id: None,
        title_received: body.meeting_name() != "Meeting notes",
    };

    let store = spin_sdk::key_value::Store::open_default()?;
    store.set_json(&notion_id, &mtg_record)?;

    Ok(Response::builder()
        .status(200)
        .body(())
        .build())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct MeetingInProgress {
    notion_id: String,
    slack_id: Option<String>,
    title_received: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct WebhookBody {
    data: WebhookData,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct WebhookData {
    id: String,
    properties: serde_json::Value,
}

impl WebhookBody {
    fn id(&self) -> &str {
        &self.data.id
    }

    fn meeting_name(&self) -> &str {
        self.data
            .properties
            .get("Meeting Name")
            .and_then(|v| v.get("title"))
            .and_then(|v| v.get("plain_text"))
            .and_then(|v| v.as_str())
            .unwrap_or("Meeting notes")
    }
}
