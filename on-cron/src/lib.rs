use cenote_dtos::{MeetingInProgress, WebhookData};
use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
async fn handle_on_cron(_req: Request) -> anyhow::Result<impl IntoResponse> {
    // println!("Handling cron callback");

    let notion_db = match get_notion_db().await {
        Ok(db) => db,
        Err(e) => {
            println!("Failed to get Notion DB: {e:?}");
            return Err(e);
        }
    };

    let store = spin_sdk::key_value::Store::open_default()?;

    let notion_ids = store.get_keys()?;

    for notion_id in &notion_ids {
        if let Err(e) = update_or_gc(&store, notion_id, &notion_db).await {
            println!("Error updating {notion_id}: {e:?}");
        }
    }

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(())
        .build())
}

async fn get_notion_db() -> anyhow::Result<NotionDb> {
    // Get the props from Notion
    /*
    curl -X POST 'https://api.notion.com/v1/databases/190ecc8b718180b7bc41e66368941285/query' \
  -H 'Authorization: Bearer '"$NOTION_API_KEY"'' \
  -H 'Notion-Version: 2022-06-28' \
  -H "Content-Type: application/json" \
--data '...'
     */
    let mut request = spin_sdk::http::RequestBuilder::new(spin_sdk::http::Method::Post, format!("https://api.notion.com/v1/databases/{}/query", spin_sdk::variables::get("notion_db_id")?));
    request.header("Authorization", format!("Bearer {}", spin_sdk::variables::get("notion_api_key")?));
    request.header("Notion-Version", "2022-06-28");
    request.header("Content-Type", "application/json");
    let response: spin_sdk::http::Response = spin_sdk::http::send(request.build()).await?;
    if *(response.status()) == 200 {
        Ok(serde_json::from_slice(response.body())?)
    } else {
        Err(anyhow::anyhow!("response error status {} {}", response.status(), String::from_utf8_lossy(response.body())))
    }
}

#[derive(serde::Deserialize)]
struct NotionDb {
    results: Vec<WebhookData>,
}

async fn update_or_gc(store: &spin_sdk::key_value::Store, notion_id: &str, notion_db: &NotionDb) -> anyhow::Result<()> {
    let mut meeting: MeetingInProgress = match store.get_json(notion_id) {
        Err(e) => {
            gc(store, notion_id);
            return Err(e);
        }
        Ok(None) => return Ok(()),
        Ok(Some(m)) => m,
    };

    let Ok(last_edited) = chrono::DateTime::parse_from_rfc3339(&meeting.last_edited_time) else {
        gc(store, notion_id);
        return Ok(());
    };
    let last_edited = last_edited.to_utc();

    let last_edit_age = chrono::Utc::now().signed_duration_since(&last_edited);
    if last_edit_age.num_hours() > 2 {
        gc(store, notion_id);
        return Ok(());
    }

    let Some(db_page) = notion_db.results.iter().find(|p| p.id == notion_id) else {
        gc(store, notion_id);
        return Ok(());
    };

    // if db_page.last_edited_time == meeting.last_edited_time {
    //     return Ok(());
    // }

    let unchanged = db_page.ai_summary() == meeting.last_slacked_summary.as_deref() &&
        Some(db_page.meeting_name()) == meeting.last_slacked_meeting_name.as_deref();

    if unchanged {
        // Nothing to update
        return Ok(());
    }

    // IF WE ARE HERE THEN THERE IS NEW STUFF!
    update_slack(&mut meeting, &db_page).await?;

    meeting.last_edited_time = db_page.last_edited_time().to_owned();

    _ = store.set_json(notion_id, &meeting);

    Ok(())
}

async fn update_slack(meeting: &mut MeetingInProgress, db_page: &WebhookData) -> anyhow::Result<()> {
    println!("UPDATING SLACK with meeting name {} and summary {:?}", db_page.meeting_name(), db_page.ai_summary());
    let slack_client = cenote_dtos::slack::SlackClient::from_variable()?;
    let text = format!("{}\n\n{}", db_page.meeting_name(), db_page.ai_summary().unwrap_or_default());
    match meeting.slack_id.as_ref() {
        Some(ts) => { slack_client.update("C08BV2B875J".to_owned(), ts.to_owned(), text).await?; }
        None => {
            let slack_id = slack_client.post_message("C08BV2B875J".to_owned(), text, None).await;
            println!("Slack TS {slack_id:?}");
            meeting.slack_id = slack_id.ok();
        }
    }
    println!("Update succeeded!");
    meeting.last_slacked_meeting_name = Some(db_page.meeting_name().to_owned());
    meeting.last_slacked_summary = db_page.ai_summary().map(|s| s.to_owned());
    Ok(())
}

fn gc(store: &spin_sdk::key_value::Store, notion_id: &str) {
    let _ = store.delete(notion_id);
}
