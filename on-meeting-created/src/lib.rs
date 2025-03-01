use cenote_dtos::{slack, MeetingInProgress, WebhookBody, WebhookData};
use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
async fn handle_on_meeting_created(req: Request) -> anyhow::Result<impl IntoResponse> {
    let body = String::from_utf8_lossy(req.body());
    // println!("PAGE CREATED: Received webhook with body {body}");
    let body: WebhookBody = serde_json::from_str(&body)?;
    let body: &WebhookData = body.as_ref();
    // println!("Found page ID {}, title {}", body.id(), body.meeting_name());

    let notion_id = body.id();

    println!("CREATING A SLACK MESSAGE with meeting name {}", body.meeting_name());
    let slack_client = slack::SlackClient::from_variable()?;
    let slack_id = slack_client.post_message("C08BV2B875J".to_owned(), body.meeting_name().to_owned(), None).await;
    println!("MESSAGE CREATED: ts = {slack_id:?}");

    let mtg_record = MeetingInProgress {
        notion_id: notion_id.to_owned(),
        slack_id: slack_id.ok(),
        last_edited_time: body.last_edited_time().to_owned(),
        url: body.url().to_owned(),
        last_slacked_meeting_name: Some(body.meeting_name().to_owned()),
        last_slacked_summary: body.ai_summary().map(|s| s.to_owned()),
    };

    let store = spin_sdk::key_value::Store::open_default()?;
    store.set_json(&notion_id, &mtg_record)?;

    Ok(Response::builder()
        .status(200)
        .body(())
        .build())
}
