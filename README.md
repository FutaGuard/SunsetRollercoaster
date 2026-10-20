# SunsetRollercoaster  
台灣開放資料

目標將台灣常見開放資料整合成方便一站式存取的工具

# 資料  
- [x] 台灣電力公司（今日電力、燃料別與區域曲線、備轉容量、各機組發電量）
- [x] 匯率（Google Finance，19 種幣別對新台幣）
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

- `/finance/exchange-rates/latest`：sunset-api 直接向 Apps Script 取得即時匯率，
  於記憶體快取 180 秒；快取期間內不會再呼叫 Google。來源失敗回 502、未設定回 503。
- `/finance/exchange-rates`：crawler 每小時寫入資料庫的歷史紀錄，可用 `base`
  （例如 `USD`）、`date`、`start`、`end` 篩選。GOOGLEFINANCE 的歷史資料無法從
  Apps Script 讀取，歷史紀錄完全靠定時同步累積。

Web App 網址只設定在 `env_config.yml` 的 `finance.exchange_rate_url`，crawler
與 sunset-api 共用，不放進 repo；`apps-script/finance/.clasp.json`（含 scriptId）
同樣被 git 忽略，需自行保留或以 `clasp clone <scriptId>` 重建。

修改 Apps Script 後，在 `apps-script/finance/` 執行 `clasp push`，再用
`clasp redeploy <deployment-id>` 更新既有部署，網址才會維持不變。新增 OAuth
權限時，需先在 Apps Script 編輯器執行一次 `checkRates` 重新授權。

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
