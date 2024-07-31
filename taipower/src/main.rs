use regex::Regex;
use reqwest;
use serde::{Deserialize};
use serde_json::Value;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::process;

#[derive(Debug, Deserialize)]
struct TaiPowerData {
    #[serde(rename = "aaData")]
    aa_data: Vec<TaiPower>,
}

// AaDataItem {  description: "抽蓄水力(Pumped Hydro)", plant_name: "大觀二#1", capacity: "-", generation: "0.0", percentage: "-", status: " ", notes: "" }

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

fn process_value(value: &Value) -> Value {
    match value {
        Value::String(s) => {
            // 清理 HTML 標籤，並處理空字符串
            if s.trim() == "-" {
                Value::String("".to_string())
            } else {
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

async fn fetch() -> String {
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
            match response.text().await {
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
    // println!("Hello, world!");
    env_logger::init();
    let data = fetch().await;
    // let v: Value = serde_json::from_str(&data).unwrap();
    let mut v: Value = serde_json::from_str(&data).unwrap();
    if let Some(aa_data) = v.get_mut("aaData").and_then(|d| d.as_array_mut()) {
        for item in aa_data.iter_mut() {
            if let Some(array) = item.as_array_mut() {
                for i in 0..array.len() {
                    if let Some(value) = array.get_mut(i) {
                        *value = process_value(value);
                    }
                }
            }
        }
    }
    let taipower: TaiPowerData = serde_json::from_value(v).unwrap();
}
