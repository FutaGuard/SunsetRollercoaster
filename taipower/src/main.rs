use regex::Regex;
use reqwest;
use std::collections::HashMap;
use serde::{Deserialize};
use serde_json::Value;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::process;

#[derive(Debug, Deserialize)]
struct TaiPowerData {
    #[serde(rename = "aaData")]
    aa_data: Vec<TaiPower>,
}

#[derive(Debug, Deserialize)]
struct TaiPower {
    category: String,
    label: String,
    plant_name: String,
    capacity: Option<String>,
    generation: Option<String>,
    percentage: Option<String>,
    status: Option<String>,
    notes: Option<String>,
}

fn remove_html_tags(input: &str) -> String {
    let re = Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(input, "").to_string()
}

fn process_value(value: &Value, leadzero: bool) -> Value {
    match value {
        Value::String(s) => {
            // 清理 HTML 標籤，並處理空字符串
            if ["-", " "].contains(&s.as_str()) {
                Value::String("".to_string())
            } else if leadzero {
                // 如果需要處理前導零，則對字符串進行處理
                // Value::String()
                Value::String("0".to_string())
            }
            else {
                Value::String(remove_html_tags(s))
            }
        }
        Value::Number(_) => {
            // 如果是數字，保持不變
            value.clone()
        }
        _ => {
            // 對於其他情況，直接返回原始值
            value.clone()
        }
    }
}

async fn fetch() -> HashMap<String, Value> {
    const URL: &str = "https://www.taipower.com.tw/d006/loadGraph/loadGraph/data/genary.json";
    let client = reqwest::Client::new();
    let ua = "*";
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(ua));

    match client.get(URL)
        .headers(headers)
        .send()
        .await {
        Ok(response) => {
            match response.json::<HashMap<String, Value>>().await {
                Ok(data) => {
                    data
                }
                Err(err) => {
                    println!("TaiPoser Parse JSON Error: {}", err);
                    process::exit(1);
                }
            }
        }
        Err(err) => {
            println!("TaiPower Requests Error: {}", err);
            process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut data = fetch().await;
    data.get_mut("aaData")
        .and_then(|v| v.as_array_mut())
        .map(|f| {
            for item in f.iter_mut() {
                item.as_array_mut().map(|a| {
                    for i in 0..a.len() {
                        let mut leadzero = false;
                        if [3, 4].contains(&i) {
                            leadzero = true;
                        }
                        a[i] = process_value(&a[i], leadzero);
                    }
                });
            }
        });
    
    let taipower: TaiPowerData = serde_json::from_value(Value::Object(data.into_iter().collect())).unwrap();
    
}
