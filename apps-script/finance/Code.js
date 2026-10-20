// 透過試算表的 GOOGLEFINANCE 取得即時匯率，並以 Web App (GET) 回傳 JSON。
// GOOGLEFINANCE 的歷史資料無法從 Apps Script 讀取，歷史由 crawler 定時抓取累積。

const SHEET_NAME = 'rates';
const QUOTE = 'TWD';
// 與臺灣銀行牌告匯率的 19 種幣別一致。
const BASES = [
  'USD', 'HKD', 'GBP', 'AUD', 'CAD', 'SGD', 'CHF', 'JPY', 'ZAR', 'SEK',
  'NZD', 'THB', 'PHP', 'IDR', 'EUR', 'KRW', 'VND', 'MYR', 'CNY',
];
const MAX_ATTEMPTS = 5;
const LOCK_TIMEOUT_MS = 30000;

function doGet() {
  const lock = LockService.getScriptLock();
  lock.waitLock(LOCK_TIMEOUT_MS);
  try {
    return json_(fetchRates_());
  } catch (err) {
    return json_({ error: String((err && err.message) || err) });
  } finally {
    lock.releaseLock();
  }
}

// 在編輯器手動執行一次以完成 OAuth 授權，也可用來檢查目前匯率。
function checkRates() {
  Logger.log(JSON.stringify(fetchRates_(), null, 2));
}

function fetchRates_() {
  const sheet = getSheet_();
  const range = sheet.getRange(1, 1, BASES.length, 1);
  const formulas = BASES.map((base) => [`=GOOGLEFINANCE("CURRENCY:${base}${QUOTE}")`]);

  let values = [];
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    // 先清空再寫入公式，強制重新計算，避免讀到舊值。
    range.clearContent();
    SpreadsheetApp.flush();
    range.setFormulas(formulas);
    SpreadsheetApp.flush();
    values = range.getValues().map(([value]) => value);
    if (values.every(isRate_)) break;
    Utilities.sleep(1000 * attempt);
  }

  return {
    quote: QUOTE,
    fetched_at: new Date().toISOString(),
    rates: BASES.map((base, i) => ({
      base,
      quote: QUOTE,
      rate: isRate_(values[i]) ? values[i] : null,
    })),
  };
}

function isRate_(value) {
  return typeof value === 'number' && Number.isFinite(value) && value > 0;
}

function getSheet_() {
  const spreadsheet = SpreadsheetApp.getActiveSpreadsheet();
  return spreadsheet.getSheetByName(SHEET_NAME) || spreadsheet.insertSheet(SHEET_NAME);
}

function json_(body) {
  return ContentService.createTextOutput(JSON.stringify(body)).setMimeType(
    ContentService.MimeType.JSON,
  );
}
