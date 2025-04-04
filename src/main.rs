extern crate google_calendar3 as calendar3;
use calendar3::api::Channel;
use calendar3::{Result, Error};
use calendar3::{
    CalendarHub,
    FieldMask,
    yup_oauth2,
    hyper_rustls, hyper_util};
use std::path::Path;
use tokio;
use chrono::{Datelike, Local, Utc, TimeZone};

#[tokio::main]
async fn main() {
    let secret_path = "client_secret.json";
    let token_path = "token.json";

    let secret = yup_oauth2::read_application_secret(secret_path)
        .await
        .expect("client_secret.json を読み込めませんでした");

    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(token_path)
    .build()
    .await
    .expect("認証フローの初期化に失敗しました");

    let client = hyper_util::client::legacy::Client::builder(
    hyper_util::rt::TokioExecutor::new()
    )
    .build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_or_http()
            .enable_http1()
            .build()
    );
    let hub = CalendarHub::new(client,auth);

    let now = Local::now();
    let today_start = now.date().and_hms(0, 0, 0); // NaiveDateTime
    let today_end = now.date().and_hms(23, 59, 59); // NaiveDateTime

    let today_start_utc = Utc.from_local_datetime(&today_start.naive_utc()).unwrap();
    let today_end_utc = Utc.from_local_datetime(&today_end.naive_utc()).unwrap();

    let events = hub.events().list("primary") // "primary"は主カレンダー
        .time_min(today_start_utc)
        .time_max(today_end_utc)
        .single_events(true) // 繰り返しイベントも展開
        .order_by("startTime")
        .doit()
        .await;

    match events {
        Ok((_, event_list)) => {
            if let Some(items) = event_list.items {
                if items.is_empty() {
                    println!("今日の予定はありません。");
                } else {
                    for event in items {
                        println!("{}", event.summary.unwrap_or("無題".to_string()));
                        if let Some(start) = event.start {
                            
                            if let Some(start_time) = start.date_time {
                                // UTC の DateTime を JST（日本標準時）に変換
                                let jst_time = start_time.with_timezone(&chrono::FixedOffset::east(9 * 3600));
                                let custom_format = jst_time.format("%Y/%m/%d %H:%M").to_string();
                                println!("{}",custom_format);
                            }
                        }
                        println!(); // 改行
                    }
                }
            }
        }
        Err(e) => {
            println!("カレンダーの予定を取得できませんでした: {:?}", e);
        }
    }
}
