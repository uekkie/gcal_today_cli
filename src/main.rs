extern crate google_calendar3 as calendar3;
use calendar3::{
    CalendarHub,
    yup_oauth2,
    hyper_rustls, hyper_util};
use tokio;
use chrono::{Local, Utc, TimeZone, DateTime, FixedOffset};

// 予定を格納する構造体
#[derive(Debug)]
struct Event {
    summary: String,
    start_time: Option<DateTime<FixedOffset>>,
    end_time: Option<DateTime<FixedOffset>>,
}

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
                    // 予定を格納するベクトル
                    let mut events_vec: Vec<Event> = Vec::new();
                    
                    // 予定をベクトルに格納
                    for event in items {
                        let summary = event.summary.unwrap_or("無題".to_string());
                        let mut start_time = None;
                        let mut end_time = None;
                        
                        if let Some(start) = event.start {
                            if let Some(start_dt) = start.date_time {
                                // UTC の DateTime を JST（日本標準時）に変換
                                let jst_time = start_dt.with_timezone(&FixedOffset::east(9 * 3600));
                                start_time = Some(jst_time);
                            }
                        }
                        
                        if let Some(end) = event.end {
                            if let Some(end_dt) = end.date_time {
                                // UTC の DateTime を JST（日本標準時）に変換
                                let jst_time = end_dt.with_timezone(&FixedOffset::east(9 * 3600));
                                end_time = Some(jst_time);
                            }
                        }
                        
                        events_vec.push(Event {
                            summary,
                            start_time,
                            end_time,
                        });
                    }
                    
                    // 予定を開始時刻でソート
                    events_vec.sort_by(|a, b| {
                        match (a.start_time, b.start_time) {
                            (Some(a_time), Some(b_time)) => a_time.cmp(&b_time),
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => std::cmp::Ordering::Equal,
                        }
                    });
                    
                    // 現在時刻を取得
                    let now_jst = Local::now().with_timezone(&FixedOffset::east(9 * 3600));
                    let now_time_str = now_jst.format("%H:%M").to_string();
                    
                    // 予定を表示
                    for event in &events_vec {
                        if let Some(start_time) = event.start_time {
                            let custom_format = start_time.format("%H:%M").to_string();
                            println!("{} {}", custom_format, event.summary);
                            
                            // 現在時刻がこの予定の開始時刻と次の予定の開始時刻の間にある場合
                            // 「====今====」を表示
                            if let Some(next_event) = events_vec.iter().find(|e| {
                                if let Some(next_start) = e.start_time {
                                    next_start > start_time
                                } else {
                                    false
                                }
                            }) {
                                if let Some(next_start) = next_event.start_time {
                                    if now_jst >= start_time && now_jst < next_start {
                                        println!("{} ====今====", now_time_str);
                                    }
                                }
                            } else {
                                // 最後の予定の場合
                                if now_jst >= start_time {
                                    println!("{} ====今====", now_time_str);
                                }
                            }
                        }
                    }
                    
                    // 最初の予定より前の場合
                    if let Some(first_event) = events_vec.first() {
                        if let Some(first_start) = first_event.start_time {
                            if now_jst < first_start {
                                println!("{} ====今====", now_time_str);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("カレンダーの予定を取得できませんでした: {:?}", e);
        }
    }
}
