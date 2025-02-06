#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MeetingInProgress {
    pub notion_id: String,
    pub slack_id: Option<String>,
    pub last_edited_time: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_slacked_meeting_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_slacked_summary: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WebhookBody {
    pub data: WebhookData,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WebhookData {
    pub id: String,
    pub last_edited_time: String,
    pub properties: serde_json::Value,
    pub url: String,
}

impl AsRef<WebhookData> for WebhookBody {
    fn as_ref(&self) -> &WebhookData {
        &self.data
    }
}

impl WebhookData {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn meeting_name(&self) -> &str {
        self.properties
            .get("Meeting Name")
            .and_then(|v| v.get("title"))
            .and_then(|v| v.as_array())
            .and_then(|v| v[0].get("plain_text"))
            .and_then(|v| v.as_str())
            .unwrap_or("Meeting notes")
    }

    pub fn last_edited_time(&self) -> &str {
        &self.last_edited_time
    }

    pub fn ai_summary(&self) -> Option<&str> {
        self.properties
            .get("AI summary")
            .and_then(|v| v.get("rich_text"))
            .and_then(|v| v.as_array())
            .and_then(|v| v[0].get("plain_text"))
            .and_then(|v| v.as_str())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}
