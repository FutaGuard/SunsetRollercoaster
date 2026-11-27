Read-only REST API for Taipower electricity data, Taiwan and international fuel prices, exchange rates, live fire department dispatch cases, invoice draws, and reservoir records. Responses are JSON; errors are `{"error": "..."}`. Times are RFC 3339 in UTC, and `date`, `start`, `end` parameters are Asia/Taipei dates (`YYYY-MM-DD`).

### Fire dispatch

- `/fire-dispatch/latest`, `/fire-dispatch/latest/{county}`: read from the sources on demand, cached 60 seconds per county; only cases received in the last 6 hours.
- `/fire-dispatch/records`: every stored state (a new record whenever a case's status or unit changes), kept 90 days.
- `/fire-dispatch/cases`: records merged into cases with their status `history`.

Most counties publish no case number, so cases are keyed by `county` + `case_id` where present (Tainan, New Taipei), otherwise by `county` + `received_at` + `location` + `category`.

**Frontend example**: show a county's live cases, then keep a list of cases current by polling `/fire-dispatch/cases` with `since`. `since` is inclusive, so a case seen in the previous poll can come back; replacing it by its key makes that harmless. A `+` in an offset must be sent as `%2B`, which `URLSearchParams` does. Run it as `<script type="module">` (top-level `await`) on a page with `<ul id="cases"></ul>`.

```js
const API = "https://<api-host>";

async function getJson(path, params = {}) {
  const response = await fetch(`${API}${path}?${new URLSearchParams(params)}`);
  if (!response.ok) throw new Error((await response.json()).error);
  return response.json();
}

// Live list of one county (502 when its source is down).
const tainan = await getJson("/fire-dispatch/latest/tainan");
console.log(tainan.fetched_at, tainan.cases);

// Cases of the last 24 hours, then only the ones that changed.
const cases = new Map();
const caseKey = (c) =>
  c.case_id ? `${c.county}#${c.case_id}` : `${c.county}|${c.received_at}|${c.location}|${c.category}`;
let since = null;

async function poll() {
  const params = { limit: 1000, ...(since && { since }) };
  for (const c of await getJson("/fire-dispatch/cases", params)) {
    cases.set(caseKey(c), c);
    if (!since || Date.parse(c.last_seen_at) > Date.parse(since)) since = c.last_seen_at;
  }
  render([...cases.values()].sort((a, b) => b.first_seen_at.localeCompare(a.first_seen_at)));
}

function render(list) {
  document.querySelector("#cases").replaceChildren(
    ...list.map((c) => {
      const row = document.createElement("li");
      row.textContent = `${c.county} ${c.category} ${c.location} ${c.status ?? ""} (${c.history.length} states)`;
      return row;
    }),
  );
}

poll();
setInterval(poll, 30_000); // the crawler stores new states every 30 seconds
```
