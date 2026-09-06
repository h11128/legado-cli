# Deferred: engine `site:` search when native search is dead

Status: **ideas only — do not implement** until a specific source is judged worth the cost.  
Default repair action remains **disable** (user preference 2026-07-27).  
Do **not** fake search by filtering homepage/榜单 HTML.

## Use case

Novel host still serves detail/TOC/content, but **on-site search returns 0 hits** (empty jieqi index, etc.).  
Question: can search engines’ `site:hostname keyword` replace the dead search API?

## Precedent in this repo (book-source side)

| Source | Pattern | Notes |
|--------|---------|--------|
| 顶点小说 ddxsmf | `searchUrl = https://cn.bing.com/search?q=site:www.ddxsmf.com%20{{key}}`；`bookList = #b_results > li` | KB: `assets/knowledge_base/book_sources/6875_顶点小说ddxsmf_书源_20260218_103244.md` |

That is **device-side SERP scraping** (阅读 App fetches Bing HTML). It works until Bing changes DOM / shows captcha / throttles.

Google `site:` in the same style is rarer in our KB (no clean hit besides Bing).

## Better from MCP / agent angle (2026-07 research)

Goal for agents: **structured results + stable API**, not brittle HTML selectors on google.com/bing.com.

| Option | MCP / agent fit | `site:` support | Cost / friction | Fit for later |
|--------|-----------------|-----------------|-----------------|---------------|
| **Brave Search API** + official MCP [`@brave/brave-search-mcp-server`](https://github.com/brave/brave-search-mcp-server) | Best: first-party MCP, `brave_web_search`, JSON | Put `site:host key` in query string | Free tier ~2k/mo; Pro for extras | **Preferred experiment** when we revisit |
| **Serper** (`google.serper.dev`) | REST JSON; some agent frameworks wrap it; no stock MCP in this workspace | `q=site:host key` | Credits; free trial common | Strong Google coverage |
| **SerpAPI** | Same idea as Serper | Yes | Pricier | Backup |
| **Bing Web Search API** (Azure) | Official JSON; MCP would be custom/thin wrapper | Yes | Azure key + quota | Aligns with ddxsmf’s Bing choice |
| **Google Custom Search JSON API** | Official; custom MCP easy | Yes | 100 free/day then paid; CX id | OK for tiny volume |
| **Self-host SearXNG** | Community MCP exists in the wild | Depends on enabled engines | Ops cost | Good if we want no third-party key |
| **Tavily / Exa** | Agent-oriented search MCP/APIs | Domain filters vary; not classic `site:` SERP | Paid | Better for RAG than “list book URLs on host X” |
| **Raw `user-fetch` MCP** (this workspace today) | Can GET a Bing/Google URL | N/A — returns page text/HTML | Free but **fragile** (captcha, markup) | Only for one-off human-assisted probes |

### This workspace right now

- Connected MCPs: **legado**, **fetch**, github, etc. — **no dedicated web-search MCP**.
- Agent can only approximate engine search via `user-fetch` on a SERP URL (same class of fragility as ddxsmf).
- Adding Brave Search MCP later would be the cleanest agent path for **triage** (“does `site:b483.com 书名` return `/info/` links?”) before deciding to invest in a source.

## Two different products (do not confuse)

1. **Agent triage / repair-time discovery**  
   Agent + Brave/Serper → see if the host is indexed → decide disable vs invest.  
   Does **not** by itself fix the phone book source.

2. **Runtime book source search** (what 阅读用户点搜索时跑的)  
   Still needs either:  
   - `searchUrl` pointing at an engine SERP (ddxsmf style), or  
   - a **small proxy** we own that calls Brave/Serper and returns Legado-friendly HTML/JSON, or  
   - disable / abandon.

Homepage-filter JS is explicitly **out**.

## Decision gate (when we “have time”)

Only open an implementation task if **all** are true:

1. User marks the host as high value (catalog quality, uniqueness).
2. Agent confirms via API/`site:` that detail URLs are indexed.
3. We pick one path: device SERP scrape **or** API proxy — and accept captcha/quota risk.
4. Verify on device with real keyword; no claim fixed without check.

Until then: **disable** + ledger; leave this doc as the backlog note.

## Related

- Skill trap: jieqi 搜索 0 条 → disable (not homepage filter).
- Discipline §16: same.
- Retrospective §15: b483 case.
