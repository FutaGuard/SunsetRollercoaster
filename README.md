# SunsetRollercoaster  
台灣開放資料

目標將台灣常見開放資料整合成方便一站式存取的工具

# 資料  
- [x] 台灣電力公司（今日電力、燃料別與區域曲線、備轉容量、各機組發電量）
- [x] 匯率（Google Finance，20 種幣別對新台幣）
- [x] 消防局即時案件（18 縣市，依縣市分組）
- [ ] 統一發票
- [ ] 台灣水庫
- [ ] 台灣中油
- [ ] 全國加油站

## 油價同步

國內油價與國際原油週均價可能在不同時間公布。爬蟲會先寫入已取得的國內油價；同期原油尚未公布、連線失敗或回應無法解析時，缺少的原油價格以 `null` 回傳，後續每日同步再補齊。暫時缺值不會清除已儲存的原油價格。

油價爬蟲與其他 Python 測試可使用 `uv run --frozen python -m unittest discover -s tests -v` 驗證。

## 台電 API

台電功率資料統一換算為 MW，可用 `date`、`start`、`end` 查詢歷史區間；
日期參數一律使用台北日期的 `YYYY-MM-DD` 格式，例如
`?start=2026-08-14&end=2026-08-21`，不需附帶時間或時區。
各類別也提供 `/latest`：

- `/taipower/power-snapshots`
- `/taipower/fuel-mix`
- `/taipower/area-loads`、`/taipower/area-snapshots`
- `/taipower/operating-reserves`
- `/taipower/generators`

完整參數與回應 schema 請見 `/swagger-ui`。

## 匯率

匯率來自 Google 試算表的 `GOOGLEFINANCE`，由 `apps-script/finance/` 的 Apps Script
Web App 以 JSON 提供。幣別與臺灣銀行牌告匯率一致，`rate` 為 1 單位 `base`
可兌換的新台幣。

- `/finance/exchange-rates/latest`：最近一次成功抓取後的 180 秒內直接回快取。
  之後的第一個請求觸發背景更新；只要距離來源的 `observed_at` 未滿 10 分鐘，
  就立即回傳上一份匯率及原始時間。每個 API 程序同時只抓一次，更新失敗不會延長舊資料期限。
  冷啟動或資料已達 10 分鐘時，請求最多等 5 秒；來源失敗、回傳過期資料或仍未完成時回 502。
  請求結束不會取消背景更新；未設定來源回 503。
- `/finance/exchange-rates`：crawler 每小時寫入資料庫的歷史紀錄，可用 `base`
  （例如 `USD`）、`date`、`start`、`end` 篩選。GOOGLEFINANCE 的歷史資料無法從
  Apps Script 讀取，歷史紀錄完全靠定時同步累積。

Web App 網址只設定在 `env_config.yml` 的 `finance.exchange_rate_url`，crawler
與 sunset-api 共用，不放進 repo；`apps-script/finance/.clasp.json`（含 scriptId）
同樣被 git 忽略，需自行保留或以 `clasp clone <scriptId>` 重建。

修改 Apps Script 後，在 `apps-script/finance/` 執行 `clasp push`，再用
`clasp redeploy <deployment-id>` 更新既有部署，網址才會維持不變。新增 OAuth
權限時，需先在 Apps Script 編輯器執行一次 `checkRates` 重新授權。

## 消防局即時案件

sunset-api 直接讀取 18 個縣市消防局公開的即時案件列表（不經資料庫），統一成同一種格式：

- `/fire-dispatch/counties`：縣市代碼、名稱與來源網址。
- `/fire-dispatch/latest`：全部縣市，依縣市分組。各縣市同時抓取；某縣市來源失敗時，
  該縣市帶 `error` 且 `cases` 為空，不影響其他縣市。
- `/fire-dispatch/latest/{county}`：單一縣市，例如 `/fire-dispatch/latest/tainan`。
  未知代碼回 404，來源失敗或頁面格式無法辨識回 502。

每筆案件為 `received_at`、`category`（`案類-案別`）、`location`、`unit`、`status`、
`case_id`、`latitude`、`longitude`，來源沒有的欄位為 `null`。只保留 6 小時內受理的案件
（列表常殘留舊案或範例資料）；新北來源沒有受理時間，`received_at` 為 `null`，
但有車輛與座標。每個縣市的結果快取 60 秒，同時進來的請求共用一次抓取。

### 歷史紀錄

Python crawler 每 30 秒讀一次 sunset-api 的 `/fire-dispatch/latest`（位址設定於
`env_config.yml` 的 `sunset_api.url`，compose 網路內為 `http://sunset-api:<server.port>`），
寫入 `fire_dispatch_record` 資料表。解析、代理都沿用 sunset-api，crawler 不重複實作。

多數縣市不公開案件編號，因此不推斷「同一案件」：同一案件每換一次狀態或派遣分隊就新增一筆，
狀態沒變只更新 `last_seen_at`，`first_seen_at` 為第一次看到該狀態的時間。紀錄保留 90 天：
crawler 每次同步時刪除 `first_seen_at` 超過 90 天的紀錄。

- `/fire-dispatch/records`：依 `county`、`date`、`start`、`end`（以台北日期篩選 `first_seen_at`）
  查詢，`first_seen_at` 由新到舊，支援 `limit`、`offset`。
- `since`（RFC 3339，例如 `2026-11-27T12:00:00Z`）：`records` 與 `cases` 都可用，只回傳
  `last_seen_at >= since` 的紀錄或案件，改為由舊到新。含等於 `since` 的資料，增量輪詢時
  傳入已收到的最大 `last_seen_at`，紀錄再用 `id` 去重、案件則以新回傳的整筆取代舊的；
  時區偏移的 `+` 需寫成 `%2B`。
- `/fire-dispatch/cases`：把紀錄合併成案件。有案件編號的縣市（台南、新北）依編號，
  其餘依「縣市＋受理時間＋地點＋類別」視為同一件（同時派遣多個分隊會併成一件）。
  目前欄位取最後看到的狀態，`history` 依序列出每個狀態；可用 `county`、`date`、`start`、
  `end`（篩案件的 `first_seen_at`）與 `since`。都沒給時只列 24 小時內出現的案件。

南投消防局憑證鏈到 TWCA CYBER Root CA，不在內建根憑證中；
`sunset-api/certs/twca-cyber-root-ca.pem` 只加入這個 API 的抓取用連線，
SHA-256 指紋與 TWCA 2025 年 WebTrust 稽核報告列出的一致。

高雄、新竹市、嘉義市的消防局擋掉部署主機所在的機房網段（TCP 連線逾時，從家用網路連得上）。
在 `env_config.yml` 設定 `fire_dispatch`（見 `env_config.example.yml`）後，
`proxy_counties` 列出的縣市改走與 Apps Script `PROXY_URL` 相同的網址前綴代理
（`<proxy_url>/<來源網址>`，Basic Auth 用 `proxy_basic_auth`），其他縣市照常直連。
代理網址與帳密只放在 `env_config.yml`，API 錯誤訊息與 log 不會帶出代理網址。

## 資料庫遷移

資料庫名稱為 `sunset`。從舊版升級時，先將 `env_config.yml` 的
`database.name` 改為 `sunset`，再啟動 crawler。crawler 會在排程啟動前自動執行：

```console
alembic upgrade head
```

Alembic 會透過 `postgres` maintenance database 將舊的 `taiwanreservoir`
改名為 `sunset`，原有 schema 與資料不會重建。改名時會短暫中斷舊資料庫的
現有連線。執行帳號必須是資料庫 owner，並且具有 `CREATEDB`
權限；若有其他 session，還必須有權終止它們。遷移完成後可以撤回
`CREATEDB` 權限。

也可以手動執行：

```console
uv run alembic upgrade head
```

若 `taiwanreservoir` 與 `sunset` 同時存在，遷移會停止，不會自動覆蓋或刪除任何一邊。
