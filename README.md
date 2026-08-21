# SunsetRollercoaster  
台灣開放資料

目標將台灣常見開放資料整合成方便一站式存取的工具

# 資料  
- [x] 台灣電力公司（今日電力、燃料別與區域曲線、備轉容量、各機組發電量）
- [ ] 統一發票
- [ ] 台灣水庫
- [ ] 台灣中油
- [ ] 全國加油站

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
