//! Parser tests using markup trimmed from each county's live case list.

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sunset_api::fire_dispatch::{FireDispatchError, parse_grid, parse_new_taipei, parse_taipei};
use sunset_api::models::FireDispatchCase;

/// 2026-11-26 11:20 in Asia/Taipei.
fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 26, 3, 20, 0).unwrap()
}

/// Taipei local time as UTC.
fn taipei(h: u32, m: u32, s: u32) -> Option<DateTime<Utc>> {
    Some(Utc.with_ymd_and_hms(2026, 11, 26, h - 8, m, s).unwrap())
}

fn case(received_at: Option<DateTime<Utc>>, category: &str, location: &str) -> FireDispatchCase {
    FireDispatchCase {
        received_at,
        category: category.to_string(),
        location: location.to_string(),
        unit: None,
        status: None,
        case_id: None,
        latitude: None,
        longitude: None,
    }
}

#[test]
fn dts_table_maps_columns_by_header_and_drops_stale_rows() {
    // Tainan: commented-out cells, a case number column, and leftover old rows.
    let html = r#"<table id="dataTable">
      <tr><th>序號</th><!-- <th></th> --><th>編號</th><th>受理時間</th><th>案類</th>
          <th>發生地點</th><th>派遣分隊</th><th>執行狀況</th></tr>
      <tr><td>1</td><!-- <td></td> --><td>111247</td><td>2026/11/26 11:12:47</td><td>緊急救護</td>
          <td>台南市北區</td><td>和緯分隊</td><td>已到達</td></tr>
      <tr><td>2</td><td>090001</td><td>2026/11/25 09:00:01</td><td>火災</td>
          <td>台南市東區</td><td>東門分隊</td><td>已返隊</td></tr>
      <tr><td colspan="7">查無資料</td></tr>
    </table>"#;

    let cases = parse_grid(html, now()).unwrap();

    assert_eq!(
        cases,
        vec![FireDispatchCase {
            unit: Some("和緯分隊".into()),
            status: Some("已到達".into()),
            case_id: Some("111247".into()),
            ..case(taipei(11, 12, 47), "緊急救護", "台南市北區")
        }]
    );
}

#[test]
fn div_grid_skips_inline_field_titles() {
    // Taoyuan: `<div class="tr">` rows whose cells repeat the header in `<span class="title">`.
    let html = r#"<div class="table">
      <div class="tr"><div class="th">發生時間</div><div class="th">案類</div><div class="th">案別</div>
        <div class="th">發生地點</div><div class="th">派遣分隊</div><div class="th text-nowrap">案件狀態</div></div>
      <div class="tr">
        <div class="td text-nowrap"><span class="title">發生時間</span><date>2026-11-26 10:41</date></div>
        <div class="td"><span class="title">案類</span>火災</div>
        <div class="td"><span class="title">案別</span>誤(謊)報</div>
        <div class="td"><span class="title">發生地點</span>桃園市龍潭區中原路二段</div>
        <div class="td text-left"><span class="title">派遣分隊</span>高平分隊</div>
        <div class="td state"><span class="text-nowrap state">處理中</span></div>
      </div>
    </div>"#;

    let cases = parse_grid(html, now()).unwrap();

    assert_eq!(
        cases,
        vec![FireDispatchCase {
            unit: Some("高平分隊".into()),
            status: Some("處理中".into()),
            ..case(taipei(10, 41, 0), "火災-誤(謊)報", "桃園市龍潭區中原路二段")
        }]
    );
}

#[test]
fn list_grid_skips_screen_reader_labels_and_map_widgets() {
    // Changhua: sr-only labels and a map link; Taichung: a map button and modal inside the cell.
    let changhua = r#"<ul class="list rwd-table" role="grid">
      <li class="list_head"><span class="w10">序號</span><span class="w20">受理時間</span><span>案類</span>
        <span>案別</span><span>發生地點</span><span>派遣分隊</span><span>執行狀況</span></li>
      <li class="list_date">
        <span data-th="序號："><span class="sr-only">序號</span>1</span>
        <span data-th="受理時間："><span class="sr-only">受理時間</span>2026/11/26 11:16:18</span>
        <span data-th="案類："><span class="sr-only">案類</span>緊急救護</span>
        <span data-th="案別："><span class="sr-only">案別</span>車禍</span>
        <span data-th="發生地點："><span class="sr-only">發生地點</span><a class="map_btn" href="https://www.google.com.tw/maps/place/埤頭鄉彰水路一段">埤頭鄉彰水路一段 <img src="map_icon.png" alt=""></a></span>
        <span data-th="派遣分隊："><span class="sr-only">派遣分隊</span>埤頭分隊</span>
        <span data-th="執行狀況："><span class="sr-only">執行狀況</span>已出勤</span>
      </li>
    </ul>"#;
    let taichung = r#"<ul class="list rwd-table">
      <li class="list_head"><span class="w15 text_le">受理時間</span><span>案類</span><span>案別</span>
        <span>發生地點</span><span>派遣分隊</span><span>執行狀況</span></li>
      <li>
        <span class="w15 list_word" data-th="受理時間：">2026/11/26 11:10:32</span>
        <span data-th="案類：">緊急救護</span>
        <span data-th="案別">車禍</span>
        <span data-th="發生地點：">東區旱溪東路
          <button type="button" class="map_icon"><span class="sr-only">google 地圖</span></button>
          <div class="modal fade map_box"><div class="modal-title">台中市東區旱溪東路</div></div>
        </span>
        <span data-th="派遣分隊：">東英分隊</span>
        <span data-th="執行狀況：">已出動</span>
      </li>
    </ul>"#;

    assert_eq!(
        parse_grid(changhua, now()).unwrap(),
        vec![FireDispatchCase {
            unit: Some("埤頭分隊".into()),
            status: Some("已出勤".into()),
            ..case(taipei(11, 16, 18), "緊急救護-車禍", "埤頭鄉彰水路一段")
        }]
    );
    assert_eq!(
        parse_grid(taichung, now()).unwrap(),
        vec![FireDispatchCase {
            unit: Some("東英分隊".into()),
            status: Some("已出動".into()),
            ..case(taipei(11, 10, 32), "緊急救護-車禍", "東區旱溪東路")
        }]
    );
}

#[test]
fn roc_year_dates_and_blank_cells() {
    // Chiayi County: ROC year, combined category column, blank status.
    let html = r#"<table class="disaster_form"><tbody>
      <tr><th scope="col">受理時間</th><th scope="col">案類-細項</th><th scope="col">案發地點</th>
          <th scope="col">派遣分隊</th><th scope="col">案件狀態</th></tr>
      <tr><td><p>115-11-26 11:03:46</p></td><td><p>緊急救護-急病</p></td>
          <td align="left"><p>嘉義縣中埔鄉中興西村</p></td><td><p>水上分隊</p></td><td><p></p></td></tr>
    </tbody></table>"#;

    assert_eq!(
        parse_grid(html, now()).unwrap(),
        vec![FireDispatchCase {
            unit: Some("水上分隊".into()),
            ..case(taipei(11, 3, 46), "緊急救護-急病", "嘉義縣中埔鄉中興西村")
        }]
    );
}

#[test]
fn header_without_rows_is_no_cases_but_missing_header_is_an_error() {
    let empty = r#"<table class="disaster_form"><tbody><tr><th>受理時間</th><th>案類-細項</th>
      <th>案發地點</th><th>派遣分隊</th><th>案件狀態</th></tr></tbody></table>"#;
    assert_eq!(parse_grid(empty, now()).unwrap(), vec![]);

    let maintenance = "<html><body><h1>系統維護中</h1></body></html>";
    assert!(matches!(
        parse_grid(maintenance, now()),
        Err(FireDispatchError::Invalid(_))
    ));
}

#[test]
fn taipei_json_joins_kind_and_code() {
    let body = json!({"total": 2, "rows": [
        {"inTime": "2026/11/26 11:14:15", "csKindName": "救護", "csCodeName": "", "caseStatus": "已出勤", "csPlaceFuzzy": "臺北市大同區哈密街"},
        {"inTime": "2026/11/26 10:02:00", "csKindName": "火災", "csCodeName": "住宅", "caseStatus": "到達現場", "csPlaceFuzzy": "臺北市中正區延平南路"},
        {"inTime": "2026/11/25 10:02:00", "csKindName": "火災", "csCodeName": "", "caseStatus": "已返隊", "csPlaceFuzzy": "臺北市士林區"}
    ]});

    assert_eq!(
        parse_taipei(&body.to_string(), now()).unwrap(),
        vec![
            FireDispatchCase {
                status: Some("已出勤".into()),
                ..case(taipei(11, 14, 15), "救護", "臺北市大同區哈密街")
            },
            FireDispatchCase {
                status: Some("到達現場".into()),
                ..case(taipei(10, 2, 0), "火災-住宅", "臺北市中正區延平南路")
            },
        ]
    );
}

#[test]
fn new_taipei_unwraps_geojson_string_and_lists_vehicles() {
    let data = json!({"type": "FeatureCollection", "features": [
        {"type": "Feature", "properties": {
            "featureId": "261126138325", "fireType": "一般火警", "type": "FireV2",
            "endPointInfo": "新北市永和區永和路一段", "lat": 25.00685, "lng": 121.51301,
            "caseList": [{"startPointInfo": "永和11"}, {"startPointInfo": "永和91"}, {"startPointInfo": "永和11"}]
        }},
        {"type": "Feature", "properties": {
            "featureId": "261126138323", "type": "AmbulanceBack", "endPointInfo": "北市台大醫院"
        }}
    ]});
    let body = json!({"status": 200, "message": "successful", "data": data.to_string()});

    assert_eq!(
        parse_new_taipei(&body.to_string()).unwrap(),
        vec![
            FireDispatchCase {
                unit: Some("永和11、永和91".into()),
                case_id: Some("261126138325".into()),
                latitude: Some(25.00685),
                longitude: Some(121.51301),
                ..case(None, "一般火警", "新北市永和區永和路一段")
            },
            FireDispatchCase {
                case_id: Some("261126138323".into()),
                ..case(None, "救護案件", "北市台大醫院")
            },
        ]
    );

    let failure = json!({"status": 500, "message": "error", "data": null});
    assert!(matches!(
        parse_new_taipei(&failure.to_string()),
        Err(FireDispatchError::Invalid(_))
    ));
}
