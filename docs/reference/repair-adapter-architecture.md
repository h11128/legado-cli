# LegadoSkill platform architecture

Scope: **not only repair** — create, optimize, merge, pattern-extract, gate/check, migrate/hunt, video/file, and infra.  
Device MCP verify remains source of truth for “works on phone.”

Related: `docs/repair-pipeline-design.md` (layer branching), `assets/真实书源模板库.txt` (static samples).
Spec anchors in this doc: **§3 types**, **§8 contracts**, **§9 database**, **§10 algorithms**, **§12 acceptance**.

---

## 0. Operational rule (thorough parity in progress)

**Do not claim §12 functional green** unless `docs/parity/THOROUGH_ACCEPTANCE.md` hard gates pass (including **search-layer live E2E** + `search-parity` suite). Soft CLI/shim inventory alone is **not** enough.

| Prefer now | Still OK (orchestration) | Blocked claim |
|------------|--------------------------|---------------|
| `source-cli diagnose` → `repair` (live probe for search) | `mcp_discover`, `parity_*`, harvest/wave | “§12 functional complete” without thorough gate |
| Gap tracker: `docs/parity/SEARCH_LAYER_GAPS.md` | Python `REPAIR_USE_PYTHON=1` emergency | §12.6 perf cutover |

See `docs/parity/ACCEPTANCE_LOG.md` for current honest status.

---

## 1. Templates: we already have them — gap is executability

| Asset | Role today | Target role |
|-------|------------|-------------|
| `assets/真实书源模板库.txt` | Human/LLM copy-paste JSON | Seed `SiteFamily` defaults |
| `assets/书源输出模板_严格模式.md` | Field checklist for create | Create validation schema |
| `assets/真实书源知识库.md` / knowledge_base | Docs | Fingerprint notes per family |
| Old `get_book_source_templates()` | Skill tool (v0.2–0.7) | Replace with `pattern extract` + registry API |
| Current `scripts/repair_*.py` | Layer heuristics | Call adapters after Identify |

**Yes — extract common patterns from live working sources** (enabled + verify OK). That is a first-class capability below (`pattern`).

---

## 2. Capability map (product verbs)

| Capability | Intent | Device truth |
|------------|--------|--------------|
| **gate** | Alive? parked? wall? migrate? | PC L0/L1/L2 |
| **pattern** | Cluster working sources → SiteFamily templates | Optional re-verify samples |
| **identify** | URL/HTML → SiteFamily | PC HTML |
| **create** | URL or HTML → new BookSource JSON | Must verify |
| **optimize** | Working source → faster/cleaner rules (same behavior) | Must verify before/after |
| **repair** | Failing source → patch by family/layer | Must verify |
| **merge** | N sources same site/family → 1 canonical | Must verify winner |
| **migrate** | Host change; rewrite absolute URLs | Must verify on new host |
| **hunt** | Dead host → candidate replacement domains | L2 then migrate+verify |
| **check** | Batch/single validate on device | MCP check |
| **disable** | Mark dead / unrepairable | MCP save enabled=false |
| **video/file** | type=3/4 flows (not novel TOC) | MCP check typed |

Modes for agent UX: **oneshot** (one URL, report) | **batch** (stream `REPORT_JSON`).

---

## 3. Core types

Implementation target: Python 3.11+ `TypedDict` / `Literal` / `Enum` in `source_core/types.py`, with JSON Schema mirrors under `config/repair_contracts/`.  
**Do not invent a second BookSource field vocabulary** — App `BookSource.kt` JSON is the only field SOT.

### 3.1 Identity & opaque payloads

| Type | Definition | Invariants |
|------|------------|------------|
| `Url` | `str`, absolute `http(s)://…` | Trim leading/trailing whitespace on every get/save; never store leading-space URLs |
| `HostKey` | `urlparse(url).netloc.lower()` | No port strip unless App does; IDN left as received |
| `SourceKey` | `bookSourceUrl` after trim | Primary key for MCP get/save/delete and ledger joins |
| `BookSource` | `dict[str, Any]` matching App JSON | Opaque except known touch paths in §3.6; `bookSourceType` ∈ {0,3,4} for v1 novel/file/video |
| `PartialBookSource` | subset of BookSource fields | Only fields an adapter is allowed to set; never a "fake full source" |

### 3.2 Closed enums (extend only via PR + schema bump)

```text
Capability   = gate | pattern | identify | create | optimize | repair
             | merge | migrate | hunt | check | disable | video | file

GateAction   = verify | migrate | skip | disable | video | hunt
# verify|migrate|skip|disable = §3.2 closed set for spine decisions.
# video|hunt extra: match Python L0 / config/verify_skip_rules.json (route, not deep-repair).

Layer        = search | toc | content | explore | file_download | ok | skip
# explore only when user opts in; default verify ignores 发现-only fails.

Mode         = oneshot | batch

ReportStatus = fixed | created | optimized | merged | skipped | failed
             | extracted | disabled | migrated | hunted
# Capability-scoped: e.g. gate may only emit skipped|failed; repair uses fixed|skipped|failed.

PatchOpKind  = set | delete | migrate_host | merge_into | delete_source | disable_source

MergeStrategy = same_host | same_family | manual

OptimizeRisk  = low | medium   # medium ⇒ mandatory A/B device verify

LedgerStep    = gate | diagnose | migrate | hunt | html | probe | patch
              | apply | debug | check | divert | skip | claim | pattern
              | create | optimize | merge | disable
```

`SiteFamily` is an **open string enum** with a curated registry (not a free-form LLM tag):

| Family id | Seed signal (non-exhaustive) |
|-----------|------------------------------|
| `JieqiMobile` | `/modules/article/search.php`, `#sitebox dl` |
| `BookbenxSearch81` | search81 / shukuai-style list |
| `XunsearchPid` | `search.php?q=`, `/novel/$id.html`, xunsearch |
| `FictionListXchina` | `.item.fiction`, `.fiction-body` |
| `EmpireCmsKeyboard` | empire CMS keyboard search |
| `GongzicpApiWebView` | API + webView content |
| `BiqugeClassic` | classic 笔趣阁 DOM |
| `Shuba69` | 69shu-family toc/content |
| `QidianJson` | qidian-style JSON rules |
| `GenericForm` | form found; **no invented bookList** |
| `Unknown` | Identify confidence below threshold |

New family ids require: ≥1 fixture HTML + registry entry + optional adapter module. REPORT `family` must be a registry id, `Unknown`, or provisional `cluster_<hash8>` awaiting promotion.

### 3.3 Gate & diagnose

```text
L1Probe = {
  ok: bool,
  error?: str,           # dns / tcp / timeout
  ip?: str,
  latency_ms?: int
}

L2Probe = {
  ok: bool,
  status?: int,
  final_url?: Url,
  title?: str,
  bytes?: int,
  deadish?: str,         # "wall:…" | "deadish:…" | "shell:…"
  host_migrated?: bool,
  from_host?: HostKey,
  to_host?: HostKey,
  snippet?: str          # short, for ledger — not full HTML
}

GateResult = {
  schema_version: "1",
  url: Url,
  verify: bool,          # True only if action == verify (legacy alias for callers)
  action: GateAction,
  reason: str,           # machine id: passed_l0_l1_l2 | l2_password_or_db_wall | …
  migrate_to?: HostKey | Url,
  l0?: { rule_id: str, action: GateAction, reason: str },
  l1?: L1Probe,
  l2?: L2Probe
}

DiagnoseResult = {
  schema_version: "1",
  url: Url,
  layer: Layer,
  fail_msg?: str,
  fake_detail?: bool,    # search page parsed as 1 book → force layer=search
  reclassified_from?: Layer,
  gate?: GateResult,
  evidence: {
    search_url?: str,
    book_url?: str,
    toc_url?: str,
    debug_snippet?: str
  }
}
```

**Invariants**

1. If `action ∈ {skip, disable, video, hunt}` → never call `debug_source` / full check for that URL in the same attempt.
2. If `fake_detail` → `layer` MUST be `search` (wmp8 class); do not patch toc.
3. `migrate_to` required when `action == migrate`.
4. `verify == true` **only** when `action == verify`. For `migrate` (and skip/disable/video/hunt) Rust sets `verify: false` (§3.3); Python `classify_one` historically left `verify: true` on migrate — Rust is SOT going forward.

### 3.4 Pattern / Identify

```text
Fingerprint = {
  signals: string[],     # ordered, stable ids e.g. "search:xunsearch_q", "list:#sitebox dl"
  structural_hash: str,  # hash of normalized (searchUrl shape, bookList, chapterList, content)
  confidence: number     # 0.0 .. 1.0
}

FingerprintRule = {
  id: str,
  weight: number,        # > 0; Identify sums weights of matched signals
  match: "searchUrl_regex" | "selector_present" | "header_charset" | "type_eq" | "html_regex",
  pattern: str
}

PatternCluster = {
  schema_version: "1",
  family: SiteFamily,    # curated id or provisional "cluster_<hash8>" until promoted
  size: int,             # member count at extract time
  fingerprint: Fingerprint,
  centroid: PartialBookSource,  # modal values for searchUrl / ruleSearch.bookList / toc / content / charset
  exemplars: Url[],      # 2..5 SourceKeys that verified OK
  coverage: { [field: string]: number },  # 0..1 support ratio among members
  extracted_at: string   # ISO-8601
}

IdentifyResult = {
  schema_version: "1",
  url: Url,
  family: SiteFamily,
  fingerprint: Fingerprint,
  evidence_urls: Url[],
  score: number,         # raw weight sum
  runner_up?: { family: SiteFamily, score: number }
}
```

Identify decision: `family = argmax score`; if top score < `IDENTIFY_MIN_SCORE` (default 2.0) or margin to runner-up < `IDENTIFY_MARGIN` (default 0.5) → `Unknown`.

### 3.5 Patch / plans / verify / ledger

```text
JsonPath = string   # dotted App paths: "searchUrl", "ruleSearch.bookList", "header"
                    # NOT RFC6901 JSON Pointer (App field names have no `/` conflict today)

PatchOp = {
  op: PatchOpKind,
  path?: JsonPath,       # required for set|delete
  value?: any,           # required for set
  from_url?: Url,        # migrate_host / merge_into
  to_url?: Url,
  note?: str
}

PatchPlan = {
  schema_version: "1",
  capability: Capability,  # create|optimize|repair|merge|migrate
  family: SiteFamily,
  source_url: Url,
  ops: PatchOp[],          # non-empty unless Unrepairable
  rationale: str,          # human; also goes to ledger.note
  expected_layer?: Layer,  # what failure this plan addresses
  dry_run_ok?: bool
}

OptimizePlan = {
  schema_version: "1",
  before: BookSource,      # snapshot (may omit large loginUi if unchanged — store hash)
  after: BookSource,
  changes: PatchOp[],
  risk: OptimizeRisk,
  before_verify?: VerifyResult,
  after_verify?: VerifyResult
}

MergePlan = {
  schema_version: "1",
  strategy: MergeStrategy,
  survivors: Url[],        # usually length 1 = canonical SourceKey
  drop: Url[],
  canonical: BookSource,
  score_breakdown?: { [url: string]: MergeScore }
}

MergeScore = {
  enabled: bool,
  last_verify_ok: bool,
  respond_time_ms?: int,   # lower better when ok
  rule_completeness: number,  # 0..1 fraction of required fields non-empty
  total: number
}

VerifyResult = {
  schema_version: "1",
  url: Url,
  success: bool,
  message: str,
  mode: Mode,
  check_discovery: bool,   # default false for repair scoring
  duration_ms?: int,
  raw_check?: object       # optional MCP payload ref / truncated
}

LedgerRow = {
  schema_version: "1",
  ts: string,              # ISO-8601 UTC
  url: Url,
  step: LedgerStep,
  result: str,
  note?: str,
  waste?: str,
  capability?: Capability,
  family?: SiteFamily,
  layer?: Layer,
  report_status?: ReportStatus
}

NeedMoreHtml = { kind: "need_more_html", urls: Url[], why: str }
Unrepairable = { kind: "unrepairable", reason: str, suggest: GateAction }  # skip|disable
```

### 3.6 Allowed BookSource touch paths (adapters)

| Path | create | repair | optimize | migrate |
|------|--------|--------|----------|---------|
| `bookSourceUrl` / `bookSourceName` | ✓ | rare | — | ✓ (host) |
| `searchUrl` / `ruleSearch.*` | ✓ | ✓ | centroid align | rewrite host |
| `ruleBookInfo.*` / `tocUrl` | ✓ | ✓ | ✓ | rewrite host |
| `ruleToc.*` / `ruleContent.*` | ✓ | ✓ | ✓ | rewrite host |
| `header` / `headerMap` / charset | ✓ | ✓ | ✓ | — |
| `concurrentRate` | default only | **not** a fix by itself | normalize | — |
| `exploreUrl` / `ruleExplore.*` | optional | only if user opts in | may drop broken | rewrite host |
| `loginUi` / `loginUrl` | playbook | playbook | — | — |
| `downloadUrls` (type 3/4) | video/file adapters | ✓ | ✓ | rewrite host |
| `enabled` / `bookSourceGroup` | ops | claim hygiene | strip 「搜索失效」 tag | — |

**Wave smell rule:** changing only `concurrentRate` never yields `ReportStatus=fixed`.

### 3.7 Error / control unions

Adapter methods return one of:

```text
PatchPlan | OptimizePlan | NeedMoreHtml | Unrepairable | None
```

`None` = no-op (optimize found nothing). Core must not treat `None` as success verify.

---

## 4. End-to-end flows

### 4.1 Shared spine

```
Input URLs / BookSource JSON
        │
        ▼
   ┌─────────┐
   │  GATE   │── skip/disable ──► LEDGER + REPORT
   └────┬────┘
        │ verify | migrate
        ▼
   ┌─────────┐
   │ MIGRATE?│  (if gate says host jumped)
   └────┬────┘
        ▼
   ┌─────────┐
   │IDENTIFY │── Unknown ──► GenericForm or create-from-HTML path
   └────┬────┘
        ▼
   capability-specific propose (create|optimize|repair|merge)
        ▼
   APPLY (save_source / delete_sources)
        ▼
   VERIFY (MCP, checkDiscovery=false unless asked)
        ▼
   LEDGER + REPORT
```

### 4.2 `pattern` — extract families from working sources

```
list_sources (enabled)
  → filter verify-ok (ledger or fresh check sample)
  → normalize fingerprints:
       searchUrl shape, ruleSearch.bookList, ruleToc.chapterList,
       ruleContent.content, header charset, type
  → cluster (structural hash / modal selectors)
  → emit PatternCluster[] + assets/templates/<family>.json
  → register adapters (centroid = default PatchPlan)
```

Rules:

- **Only verify-ok sources** enter clusters (no rotting fails).
- Cluster size ≥ N (e.g. 3) or manually promoted exemplar.
- Output feeds Identify + Create + Repair; refresh on schedule or after big import.

### 4.3 `create` — build new source

```
URL → GATE → fetch home/search/detail/chapter
    → IDENTIFY family
    → if known: adapter.create_from_html(ctx) → BookSource
    → if Unknown: HTML analyzers + template library suggestions → draft
    → VALIDATE against 书源输出模板_严格模式
    → VERIFY → save
```

Encoding: detect once (`charset`) before POST body encode (see encoding docs).

### 4.4 `optimize` — improve without changing meaning

Candidates (deterministic smells first):

- `repair_rule_smells` / webView quotes / empty tocUrl API tip
- Drop broken `exploreUrl` when 发现 not required
- Dedupe headerMap, normalize concurrentRate
- Replace fragile `||`+`##` name patterns (known bad smell)
- Prefer family centroid selectors if current ones are stricter-equivalent

Flow: snapshot → optimize plan → verify → keep only if success and (optional) latency↓.

### 4.5 `repair` — fix failing source

Same as prior adapter design: Diagnose layer + family → `adapter.propose` → verify.  
Distinguish **搜索口不对** (keep fixing via form) vs **搜索口挂了** (skip).

### 4.6 `merge` — consolidate duplicates

```
group by HostKey and/or SiteFamily
  → score each (enabled, last verify ok, respondTime, rule completeness)
  → pick canonical (best score)
  → PatchPlan: union non-conflicting fields (loginUi, explore if both ok)
  → delete_sources(drop) or disable
  → VERIFY canonical
```

Conflict policy: never silently pick conflicting `searchUrl`; prefer verify-ok exemplar.

### 4.7 `hunt` + `migrate`

Existing: `repair_domain_hunt`, `repair_domain_migrate`.  
After migrate: **re-Identify on new host** (theme may change) before repair/create.

### 4.8 `check` / `disable` / queue

Batch check, harvest cheap wins, disable dead, shard multi-device — infra around the spine.

---

## 5. Adapter interface (all capabilities)

```text
trait SiteAdapter {
  family() -> SiteFamily
  fingerprints() -> FingerprintRule[]

  create(ctx)   -> PatchPlan | NeedMoreHtml | Unrepairable
  repair(ctx)   -> PatchPlan | NeedMoreHtml | Unrepairable
  optimize(ctx) -> OptimizePlan | None
  # merge is cross-source; core MergeService uses family() for grouping only
}
```

`GenericForm`: create/repair may only set `searchUrl` from form; no invented bookList.

---

## 6. Script inventory → capability coverage

### Covered by current `scripts/` (map into packages)

| Script | Capability |
|--------|------------|
| `repair_prefilter.py` | gate |
| `precheck_sources.py` | gate (DNS/HTTP bulk) |
| `repair_search_probe.py` | identify assist / repair search |
| `repair_diagnose.py` | repair diagnose |
| `repair_debug_parse.py` | repair layer parse |
| `repair_debug_vs_check.py` | repair debug≠check |
| `repair_rule_smells.py` / `repair_patches.py` | optimize smells + repair auto |
| `repair_one.py` / `repair_deep_loop.py` | repair oneshot/batch |
| `repair_goal15_run.py` | repair batch (deprecated shim → deep_loop) |
| `repair_wave.py` / `repair_search_wave.py` / `repair_deep_wave.py` | repair triage/batch |
| `repair_harvest.py` | check harvest |
| `repair_domain_migrate.py` | migrate |
| `repair_domain_hunt.py` | hunt |
| `repair_classify.py` / `repair_queue.py` / `repair_why_wave.py` | queue / triage |
| `repair_progress.py` / `repair_session_log.py` / `repair_claim.py` | progress / ledger / anti fake-fixed |
| `repair_check.py` / `repair_wait.py` | check helpers |
| `batch_check_mcp.py` / `full_check_runner.py` | check orchestration |
| `disable_dead_sources.py` | disable |
| `shard_urls.py` | check multi-device |
| `mcp_client.py` / `mcp_channel.py` / `mcp_discover.py` | infra MCP |
| `repair_cache.py` / `repair_helpers.py` / `repair_knowledge.py` | infra cache/docs |
| `repair_source.py` | repair CLI umbrella |
| `video_prefilter.py` / `video_repair_one.py` | video/file gate+repair skeleton |
| `repair_bench10.py` | bench |

### Exists outside thin scripts (must be in platform)

| Asset | Capability | Status |
|-------|------------|--------|
| `debugger/` (`analyze_rule`, `analyze_url`, `debug_engine`, cli) | create/optimize/repair **offline parse** | Keep; wrap as `ParseService` |
| `debugger/engine/auto_fixer.py` | repair/optimize auto suggestions | Feed `OptimizePlan` / `PatchPlan` (PC-only; still need device verify) |
| `debugger/legado_checker.py` / `environment_simulator.py` | check offline / JS env | Optional pre-MCP smoke; device MCP remains SOT |
| `debugger/engine/file_organizer.py` | ops import/export folders | Keep under ops |
| `assets/真实书源模板库.txt` + `书源输出模板_严格模式.md` | pattern seed / create validate | Wire to registry + create schema |
| `assets/真实书源知识库.md` / `knowledge_base/` / encoding guides | create assist + charset | `KnowledgeService` + `CharsetService` |
| Login/encrypt JS snippets in `assets/方法-*.md` | create/repair auth sites | Playbooks; Unrepairable vs semi-auto |
| Old LangGraph “驯兽师” tools (smart_* analyzers) | create from HTML | **Rehydrate as optional** `HtmlAnalyzeService` or drop if debugger covers |
| Subscription / 漫画 / 起点 JSON samples in assets | create adjacent types | Note only in v1; novel path first |

### Gaps (not covered as first-class today — must design in)

| Gap | Needed capability | Notes |
|-----|-------------------|-------|
| **Pattern extract from live OK sources** | `pattern` | New CLI; cluster → templates |
| **Create source E2E** | `create` | Templates + identify + verify; not only repair |
| **Merge duplicates** | `merge` | same host/family; delete losers |
| **Optimize pass** | `optimize` | smells + family centroid align + A/B verify |
| **Group/tag hygiene** | optimize/ops | strip 搜索失效 after success; user groups |
| **Explore/发现 repair** | repair(explore) | only if user opts in |
| **Login / captcha / CF playbooks** | create/repair | Unrepairable vs semi-auto (assets JS refs) |
| **Import/export book source files** | ops | organize temp folders (file_organizer) |
| **Subscription/RSS sources** | separate type | out of novel adapter v1; note only |
| **Replace rules / txt toc rules** | optimize adjacent | App entities; optional later |
| **Multi-device consistent hash check** | check | `shard_urls` already; keep |
| **EWMA / rate-limit cooldown** | check/repair | `repair_cache` EWMA; don’t rewrite search on 429 HTML |
| **Type=3 file downloadUrls / type=4 video** | video/file | skeleton exists; finish adapters |
| **Leading-space / URL normalize** | all | `get_source` trim; normalize on save |
| **Anti fake-fixed** | all mutating | `repair_claim` + ledger require verify |
| **Fixture regression per family** | pattern/repair | HTML fixtures + unit tests |

---

## 7. Package layout (target)

```
crates/                          # Rust workspace (implementation SOT)
  Cargo.toml                     # workspace root
  source-types/                  # §3 enums, newtypes, RepairConfig
  source-contracts/              # §8 schema validate (embed JSON Schema)
  source-db/                     # §9 SQLite + dual-write helpers
  source-ports/                  # §14.2 traits only
  source-gate/                   # L0/L1/L2
  source-probe/                  # search probe ranking
  source-cache/                  # EWMA + html meta
  source-patch/                  # smells + PatchOp apply
  source-identify/               # fingerprint + family
  source-pattern/                # cluster extract
  source-spine/                  # orchestrate + ApplyService
  source-adapters/               # family plugins (ISP)
  source-mcp/                    # MCP HTTP client (infra)
  source-video/                  # type 3/4
  source-cli/                    # bins: source-gate, source-repair, …
packages/                        # optional thin PyO3 later — not required for P0
scripts/                         # Python shims → invoke source-cli OR legacy until parity
  parity_selftest.py
  parity_inventory.py
assets/templates/
fixtures/html/<family>/
fixtures/expected/               # golden JSON for parity
config/repair_contracts/
config/repair_config.json
temp/full_fix/repair_state.sqlite
docs/repair-adapter-architecture.md
```

Legacy `repair_*.py` become shims calling `source-cli` after the matching Rust module reaches golden parity (no new business logic in scripts).

---

## 8. Contracts (wire formats)

Contracts are **versioned JSON objects** validated by schemas in `config/repair_contracts/`.  
Python types in §3 are the authoring SOT; schemas are the runtime/CI SOT. Bump `schema_version` only on breaking changes; additive optional fields keep `"1"`.

### 8.1 Schema files (Phase A deliverables)

| File | Validates | Required keys |
|------|-----------|---------------|
| `report_json.schema.json` | stdout `REPORT_JSON:` line | `schema_version`, `capability`, `mode`, `url`, `status`, `message` |
| `gate_result.schema.json` | GateResult | `schema_version`, `url`, `verify`, `action`, `reason` |
| `diagnose_result.schema.json` | DiagnoseResult | `schema_version`, `url`, `layer` |
| `patch_plan.schema.json` | PatchPlan | `schema_version`, `capability`, `family`, `source_url`, `ops`, `rationale` |
| `optimize_plan.schema.json` | OptimizePlan | `schema_version`, `before`, `after`, `changes`, `risk` |
| `merge_plan.schema.json` | MergePlan | `schema_version`, `strategy`, `survivors`, `drop`, `canonical` |
| `verify_result.schema.json` | VerifyResult | `schema_version`, `url`, `success`, `message`, `mode`, `check_discovery` |
| `ledger_row.schema.json` | LedgerRow | `schema_version`, `ts`, `url`, `step`, `result` |
| `pattern_cluster.schema.json` | PatternCluster | `schema_version`, `family`, `size`, `fingerprint`, `centroid`, `exemplars` |
| `identify_result.schema.json` | IdentifyResult | `schema_version`, `url`, `family`, `fingerprint`, `score` |

`additionalProperties: true` on REPORT/Ledger for forward compat; **closed enums** on `action` / `status` / `step` / `capability`.

### 8.2 `REPORT_JSON` (streaming stdout)

Emit exactly one line: `REPORT_JSON:` + compact JSON (no pretty-print). Human `REPORT: […]` may follow.

```text
REPORT_JSON:{
  "schema_version": "1",
  "capability": "repair|create|optimize|merge|pattern|gate|migrate|hunt|check|disable|video|file",
  "mode": "oneshot|batch",
  "url": "https://…",
  "status": "fixed|created|optimized|merged|skipped|failed|extracted|disabled|migrated|hunted",
  "family": "JieqiMobile|Unknown|…",
  "message": "short human summary",
  "layer"?: "search|toc|…",
  "duration_ms"?: 1234,
  "fixed_n"?: 0,                 # batch summary only
  "ops_summary"?: ["set searchUrl", "…"],
  "migrate_to"?: "https://…",
  "verify"?: { …VerifyResult… }  # required when status is fixed|created|optimized|merged
}
```

**Status × capability matrix (invalid pairs fail schema / CI)**

| status ↓ \\ capability → | gate | repair | create | optimize | merge | pattern | migrate | hunt | check | disable |
|--------------------------|------|--------|--------|----------|-------|---------|---------|------|-------|---------|
| skipped / failed | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| fixed | — | ✓ | — | — | — | — | — | — | — | — |
| created | — | — | ✓ | — | — | — | — | — | — | — |
| optimized | — | — | — | ✓ | — | — | — | — | — | — |
| merged | — | — | — | — | ✓ | — | — | — | — | — |
| extracted | — | — | — | — | — | ✓ | — | — | — | — |
| migrated | — | — | — | — | — | — | ✓ | — | — | — |
| hunted | — | — | — | — | — | — | — | ✓ | — | — |
| disabled | — | — | — | — | — | — | — | — | — | ✓ |

**Anti fake-fixed (hard):** `status ∈ {fixed, created, optimized, merged}` REQUIRES nested `verify.success === true` (or equivalent check-json accepted by `assert_fixed_allowed`). Emitters must call the same gate as `repair_claim`.

**Compat:** current `repair_deep_loop` may omit `schema_version` / `capability` until Phase C; parsers treat missing `capability` as `repair`, missing `schema_version` as `"0"` (legacy). New emitters always write `"1"`.

### 8.3 Gate / Patch / Ledger wire notes

- **GateResult.reason**: prefer stable machine ids already used by `repair_prefilter` (`passed_l0_l1_l2`, `l2_password_or_db_wall`, `l2_domain_parked_or_expired`, `l2_bot_shell`, `l2_http_dead`, `l1_unreachable`, `l2_host_redirect`, plus L0 rule ids). Human text belongs in `message` / ledger `note`, not in `reason`.
- **PatchOp.path**: dotted paths only (§3.5). Validators reject empty `ops` for repair/create plans that claim a mutation.
- **LedgerRow**: append-only JSONL today; same object shape inserted into SQLite `ledger_events` (§9). `step` closed enum; unknown steps fail validation in CI but are tolerated at read time with a warning.

### 8.4 Validation entrypoints

| When | How |
|------|-----|
| Unit / parity | `parity_selftest.py --suite schemas` |
| Oneshot/batch emit | `source_core.contracts.validate_report(row)` before print |
| Claim fixed | `assert_fixed_allowed(check)` — unchanged semantics |
| Adapter propose | optional validate PatchPlan before apply |

### 8.5 Non-goals for contracts v1

- Protobuf / gRPC between PC and phone (MCP JSON stays).
- Signing REPORT lines.
- Storing full HTML inside REPORT/Ledger (use cache keys / paths).

---

## 9. Persistence / database

### 9.1 Current state (operational — keep until cutover)

| Store | Path | Role | Limits |
|-------|------|------|--------|
| Session ledger | `temp/full_fix/repair_session_ledger.jsonl` | Append-only attempt log | No index; grep-only; concurrent writers can interleave lines |
| Claim index | session JSON via `repair_claim.append_index` | verified_fixed / skipped / failed | Manual / per-session |
| HTML cache | `temp/full_fix/cache/html/{sha24}.json+.bin` | PC fetch reuse | TTL via `max_age_s`; unbounded growth |
| Host EWMA | `temp/full_fix/cache/host_stats.json` | per-host cooldown | Whole-file rewrite; race under parallel processes |
| Triage blobs | `temp/full_fix/cache/triage/{sha24}.json` | short-lived classify | TTL ~30m |
| Progress / queue files | `temp/full_fix/*.json` | harvest / deep_queue | Ad-hoc shapes |
| Templates | `assets/` + future `assets/templates/` | human + pattern output | Not a DB |

Device BookSource library remains on the **phone** (MCP `list_sources` / `get_source`). PC never becomes the App DB.

### 9.2 Target: SQLite for structured state + files for blobs

**Why SQLite:** queryable ledger (`WHERE family=? AND report_status='fixed'`), atomic host_stats updates, pattern/identify history, parity with multi-agent writers via WAL.  
**Why not put HTML in SQLite:** bodies are large and already keyed on disk; keep `html` as files, store only metadata + `cache_key` in DB.

Default path: `temp/full_fix/repair_state.sqlite` (gitignored). Optional read-only mirror under `docs/parity/` is **not** required.

```text
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;
```

### 9.3 Schema (v1)

```sql
-- identity
CREATE TABLE source_snapshot (
  source_key   TEXT PRIMARY KEY,          -- trimmed bookSourceUrl
  host_key     TEXT NOT NULL,
  name         TEXT,
  type         INTEGER NOT NULL DEFAULT 0, -- bookSourceType
  enabled      INTEGER NOT NULL,
  family       TEXT,                      -- last Identify
  structural_hash TEXT,
  group_name   TEXT,
  respond_time_ms INTEGER,
  payload_json TEXT NOT NULL,            -- full BookSource JSON last seen from MCP
  pulled_at    TEXT NOT NULL              -- ISO-8601
);
CREATE INDEX idx_source_host ON source_snapshot(host_key);
CREATE INDEX idx_source_family ON source_snapshot(family);

-- append-only events (LedgerRow)
CREATE TABLE ledger_events (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  ts           TEXT NOT NULL,
  source_key   TEXT NOT NULL,
  step         TEXT NOT NULL,
  result       TEXT NOT NULL,
  note         TEXT,
  waste        TEXT,
  capability   TEXT,
  family       TEXT,
  layer        TEXT,
  report_status TEXT,
  row_json     TEXT NOT NULL              -- full LedgerRow for forward compat
);
CREATE INDEX idx_ledger_source_ts ON ledger_events(source_key, ts);
CREATE INDEX idx_ledger_status ON ledger_events(report_status, ts);

-- gate / verify history (optional denser than ledger)
CREATE TABLE gate_runs (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  ts           TEXT NOT NULL,
  source_key   TEXT NOT NULL,
  action       TEXT NOT NULL,
  reason       TEXT NOT NULL,
  migrate_to   TEXT,
  result_json  TEXT NOT NULL
);

CREATE TABLE verify_runs (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  ts           TEXT NOT NULL,
  source_key   TEXT NOT NULL,
  success      INTEGER NOT NULL,
  message      TEXT,
  mode         TEXT,
  check_discovery INTEGER NOT NULL DEFAULT 0,
  duration_ms  INTEGER,
  capability   TEXT,
  result_json  TEXT NOT NULL
);
CREATE INDEX idx_verify_source ON verify_runs(source_key, ts);

-- EWMA / host pacing (replaces host_stats.json whole-file rewrite)
CREATE TABLE host_stats (
  host_key     TEXT PRIMARY KEY,
  ewma_gap_s   REAL NOT NULL DEFAULT 3.0,
  hits         INTEGER NOT NULL DEFAULT 0,
  ok           INTEGER NOT NULL DEFAULT 0,
  fail         INTEGER NOT NULL DEFAULT 0,
  rate_limits  INTEGER NOT NULL DEFAULT 0,
  last_rate_limit_at REAL,
  last_duration_ms INTEGER,
  last_at      REAL,
  extra_json   TEXT
);

-- HTML cache metadata only
CREATE TABLE html_cache_meta (
  cache_key    TEXT PRIMARY KEY,          -- sha256 url [:24]
  url          TEXT NOT NULL,
  host_key     TEXT NOT NULL,
  saved_at     REAL NOT NULL,
  status       INTEGER,
  final_url    TEXT,
  content_type TEXT,
  bytes        INTEGER,
  rate_limited INTEGER,
  bin_path     TEXT NOT NULL              -- relative to cache root
);
CREATE INDEX idx_html_host ON html_cache_meta(host_key, saved_at);

-- pattern extract
CREATE TABLE pattern_cluster (
  family       TEXT PRIMARY KEY,
  size         INTEGER NOT NULL,
  structural_hash TEXT,
  confidence   REAL,
  centroid_json TEXT NOT NULL,
  exemplars_json TEXT NOT NULL,
  coverage_json TEXT,
  extracted_at TEXT NOT NULL,
  promoted     INTEGER NOT NULL DEFAULT 0  -- 1 = curated SiteFamily
);

-- claims (anti fake-fixed index)
CREATE TABLE claims (
  source_key   TEXT NOT NULL,
  status       TEXT NOT NULL,             -- fixed|skipped|failed
  ts           TEXT NOT NULL,
  evidence     TEXT,
  agent        TEXT,
  root_cause   TEXT,
  PRIMARY KEY (source_key, status, ts)
);
```

### 9.4 Access layer

```text
source_core/db.py
  connect() -> Connection          # WAL, busy_timeout=5000
  migrate_schema()                 # idempotent CREATE IF NOT EXISTS + version table
  append_ledger(LedgerRow)
  upsert_host_stats(...)
  upsert_source_snapshot(BookSource)
  record_verify(VerifyResult)
  import_jsonl_ledger(path)        # one-shot migration
  export_jsonl_ledger(path, since?)  # compat for agents that still grepping JSONL
```

**Dual-write period (Phase B→C):** every `append_row` writes JSONL **and** SQLite. Readers prefer SQLite when file exists; fall back to JSONL.  
**Cutover:** Skill may keep showing JSONL path; implementation reads DB first. Do not delete JSONL until operators agree (export remains).

### 9.5 What never goes in the PC DB

- Phone App Room/SQLite book library (MCP is the API).
- Secrets / MCP tokens (stay in `config/mcp_defaults.json` / env).
- Full check HTML dumps from device debug (optional files under `temp/`, metadata only in DB).

### 9.6 Migration checklist

1. Create empty DB + schema version `1`.
2. `import_jsonl_ledger` on existing ledger (dedupe by `ts+url+step+result` hash if re-run).
3. Import `host_stats.json` → `host_stats` table.
4. Scan `cache/html/*.json` → `html_cache_meta` (skip missing `.bin`).
5. Golden: row counts and a sample query match pre-migration greps.

---

## 10. Algorithm optimization

Algorithms here are **PC-side** (gate, probe, cluster, queue, cooldown). Device checkalgo (AIMD, token bucket) stays in App; PC must **respect** the same anti-ban intent, not fight it.

### 10.1 Design principles

1. **Fail-fast beats clever:** L0 → L1 → L2 before any debug/check (already discipline).
2. **Never optimize into false fixes:** rate-only / cooldown changes ≠ `fixed`.
3. **Deterministic first:** pattern/identify/probe scoring must be golden-testable; LLM optional later, never SOT.
4. **Measure with §12.6 budgets** before claiming a faster path.

### 10.2 Gate ordering (keep; document costs)

| Stage | Cost target | Decision |
|-------|-------------|----------|
| L0 rules | ≪50ms / URL | skip/disable/migrate from `verify_skip_rules.json` |
| L1 TCP/DNS | ≤1.5s timeout | unreachable → disable |
| L2 HTTP | ≤4s timeout | wall/park/shell/migrate/verify |
| Parallel gate | ThreadPool, default 32 | per-host politeness still via EWMA before deep fetch |

Optimization lever: raise concurrency only when host diversity is high; same-host batch should serialize through cooldown (§10.3).

### 10.3 Host EWMA cooldown (existing `repair_cache`)

```text
alpha = 0.3
default_gap = 3.0s

on_rate_limit(host):
  ewma = (1-alpha)*ewma + alpha*suggested_gap   # suggested default 20s

on_verify_success(host, used_cooldown):
  target = clamp(used_cooldown, default_gap, 30s)
  ewma = (1-alpha)*ewma + alpha*target

cooldown(url) = max(ewma[host], concurrentRate_ms/1000, default_gap)
```

**Rules:** on 429 / rate-limit HTML → update EWMA and **do not** rewrite `searchUrl`. Persist in SQLite `host_stats` (§9) with row-level replace, not whole JSON rewrite, under parallel workers.

### 10.4 Search probe ranking (`repair_search_probe`)

Score candidates (form action / common paths) with additive features; higher wins:

| Feature | Direction | Notes |
|---------|-----------|-------|
| Real result list markers | + | `#sitebox dl`, `.item.fiction`, multiple book links |
| Keyword echo in results | + | query string appears in titles/hrefs |
| xunsearch / pid / search.php shapes | + | known-good families |
| Fake home / site chrome only | − | "search" returns homepage nav |
| HTTP 5xx on form POST/GET | dead | mark `search_endpoint_dead` → skip, not selector churn |
| Captcha / password wall | dead | L2-class skip |

Keep ranking pure functions of (url, html, status) for golden fixtures.

### 10.5 Pattern clustering & Identify

**Fingerprint normalize**

1. Parse `searchUrl` into shape token (`path_template`, method, charset).
2. Take `ruleSearch.bookList`, `ruleToc.chapterList`, `ruleContent.content` strings trimmed.
3. `structural_hash = sha256(join(shape, bookList, chapterList, content, type))[:16]`.

**Cluster**

1. Group verify-ok sources by `structural_hash` (exact) then merge near-duplicates if Jaccard(signals) ≥ 0.8 and same `bookSourceType`.
2. Require `size ≥ N` (default 3) or manual promote exemplar → `PatternCluster`.
3. Centroid field = mode (most common non-empty string) among members; `coverage[field] = support_ratio`.

**Identify**

1. Sum `FingerprintRule` weights for HTML + existing BookSource fields.
2. Apply §3.4 thresholds → family or `Unknown`.
3. Cache last `(source_key → family, score, ts)` in `source_snapshot` to avoid re-cluster every oneshot.

### 10.6 Queue / merge scoring

**Repair queue sort** (preserve `repair_classify.queue_sort_key` semantics unless golden updates): prefer cheap wins (known layer + live host) before deep unknown; walls never enter deep queue.

**MergeScore.total** (suggested weights — tune with fixtures, not vibes):

```text
total =
  50 * last_verify_ok
+ 20 * enabled
+ 20 * rule_completeness
+ 10 * respond_time_score   # 1 if respond_time_ms missing; else inverse-rank within group
```

Conflict: differing `searchUrl` → do not auto-merge; keep verify-ok exemplar and report `failed` with rationale.

### 10.7 Batch / spine scheduling

| Concern | Algorithm |
|---------|-------------|
| MCP single-flight | one check/debug job per device; PC HTML parallel OK |
| Batch REPORT | process sequentially on device verify; overlap next URL's PC gate/probe while waiting |
| Multi-device | keep `shard_urls` consistent hash; no cross-device same URL |
| Time budget | soft 4 min / hard 5 min per source → skip + ledger (discipline) |

Overlap pattern (target after spine):

```text
URL_i:  [gate|probe|patch PC] ----► [MCP verify]
URL_i+1:        [gate|probe PC overlapping verify_i]
```

Never overlap two MCP verifies on one phone.

### 10.8 Optional later (not v1 parity)

| Idea | When allowed |
|------|--------------|
| Rust port of gate/cluster hot paths | After P0–P3 + same JSON schemas (§12 / Phase G) |
| Learned probe weights | Only if beaten by golden + offline A/B; default stay hand weights |
| LLM family naming | Post-cluster labeling only; hash/centroid remain SOT |
| Bloom / hedged L1 on PC | Mirror App checkalgo only if measured win vs current prefilter |

### 10.9 Perf acceptance hooks

Every algorithm change that can affect wall time must update or waive §12.6 (`PERF_BASELINE.json`). Micro-benchmarks for pure functions (probe score, structural_hash, EWMA update) live next to golden fixtures — not a substitute for oneshot p95.

---

## 11. Implementation plan

**Hard rule:** existing `scripts/*.py` public CLIs stay callable with the **same flags and exit semantics** until acceptance §12.3 passes. Refactor = move logic into packages + leave shims; never “rewrite then hope.”

### 11.0 Definition of done (platform)

| Gate | Meaning |
|------|---------|
| **P0 Parity** | Every row in §12.2 parity matrix: shim or package covers the same behaviors |
| **P1 Spine** | Gate → Identify → propose → Apply → Verify → Ledger for repair oneshot |
| **P2 Families** | Top clusters + proven sites have adapters; pattern extract refreshes templates |
| **P3 New caps** | create / optimize / merge CLIs ship with device-verify contracts |
| **P4 Cleanup** | Duplicate wave scripts collapsed; docs/skill point only at new entrypoints |

### 11.1 Phase A — Inventory freeze + contracts (1–2 days)

**Work**

1. Freeze script inventory (this doc §6 + §12.2). Add `docs/parity/SCRIPT_INVENTORY.json` generated from `scripts/*.py` (name, has `__main__`, key public funcs).
2. Add JSON Schema under `config/repair_contracts/` for every row in §8.1
   (report / gate / diagnose / patch / optimize / merge / verify / ledger /
   pattern_cluster / identify_result).
3. Add empty SQLite schema migration (`source_core/db.migrate_schema`) +
   dual-write stub behind a flag (default off until Phase B).
4. Capture **golden fixtures** for deterministic PC paths (no device):
   - L0/L1/L2 classify samples (`repair_prefilter`)
   - search probe ranking samples (`repair_search_probe`)
   - smell/patch samples (`repair_rule_smells`, `repair_patches`)
   - debug text parse samples (`repair_debug_parse`)
   - migrate rewrite samples (`repair_domain_migrate.migrate_payload`)
   - classify/queue sort (`repair_classify`)
   - EWMA cooldown update samples (`repair_cache`)
5. Add `scripts/parity_selftest.py` skeleton: runs fixture suite against current scripts (baseline).

**Exit:** inventory committed; baseline selftest green on current Python.

### 11.2 Phase B — Extract libraries without behavior change (3–5 days)

Move code **as-is** into packages; scripts become thin wrappers.

| Package module | Absorb from |
|----------------|-------------|
| `source_mcp/` | `mcp_client`, `mcp_channel`, `mcp_discover` |
| `source_core/gate.py` | `repair_prefilter` (+ rules load) |
| `source_core/probe.py` | `repair_search_probe` |
| `source_core/diagnose.py` | `repair_diagnose` (orchestration stays in CLI) |
| `source_core/patch.py` | `repair_patches`, `repair_rule_smells` |
| `source_core/check.py` | `repair_check`, `repair_wait` |
| `source_core/ledger.py` | `repair_session_log`, `repair_claim`, progress helpers |
| `source_core/cache.py` | `repair_cache` |
| `source_core/db.py` | new; dual-write ledger + host_stats import |
| `source_core/contracts.py` | schema loaders; validate REPORT/Gate/Patch |
| `source_core/migrate.py` | `repair_domain_migrate` rewrite helpers |
| `source_core/hunt.py` | `repair_domain_hunt` |
| `source_core/queue.py` | `repair_classify`, `repair_queue`, `repair_why_wave` helpers |
| `source_parse/` | wrap `debugger/engine/analyze_*` (import path only) |

**Rule:** no algorithm changes in Phase B. Diff of golden fixture outputs must be empty vs Phase A baseline.

**Exit:** `parity_selftest.py` green; all shims call packages; `git diff` of fixture outputs = empty.

### 11.3 Phase C — Unified repair spine + fail-fast (2–4 days)

1. Implement `source_core/spine.py`: Gate → (Migrate?) → Identify(stub/GenericForm) → repair propose → Apply → Verify → Ledger.
2. Point `repair_deep_loop.py` / `repair_one.py` at spine; keep CLI flags.
3. Codify **搜索口不对 vs 挂了** and L2 walls in gate (already in prefilter/probe — single code path).
4. `REPORT_JSON` schema validation on every oneshot/batch emit.

**Exit:** oneshot parked/password skip &lt;5s; form-5xx → `search_endpoint_dead`; fixture + 1 live smoke URL still match old behavior class.

### 11.4 Phase D — Pattern extract + Identify + adapters (4–7 days)

1. `source_pattern_extract.py` / `source_core/pattern.py`: cluster verify-ok sources → `assets/templates/<family>.json`.
2. `source_core/identify.py` + adapter registry; seed from templates + proven fixes (alicesw, xchina, shukuai/search81, xunsearch/pid, empire keyboard, gongzicp).
3. Repair path: if family known → adapter.propose; else GenericForm / probe.

**Exit:** extract produces ≥K families (K configurable, default 5) with size≥3; Identify hits ≥80% of those exemplars; adapter repair does not regress golden patches.

### 11.5 Phase E — New capabilities (create / optimize / merge) (4–7 days)

| CLI | Depends on |
|-----|------------|
| `source_create.py` | Identify + templates + verify |
| `source_optimize.py` | smells + optional centroid align + A/B verify |
| `source_merge.py` | host/family group + score + delete/disable losers + verify |

These are **additive** — not required for P0 Parity, but must not break shims.

**Exit:** each new CLI has dry-run + verify path; documented in skill; at least one device-verified create/optimize/merge demo logged in ledger with `capability` field.

### 11.6 Phase F — Check / video / debugger wrap + collapse duplicates (3–5 days)

1. Keep `batch_check_mcp`, `full_check_runner`, `precheck_sources`, `disable_dead_sources`, `shard_urls` as shims → `source_check` package.
2. Finish `video_*` behind same gate/verify contracts.
3. Wrap debugger as `ParseService`; charset helpers from encoding docs.
4. Collapse overlapping waves (`repair_search_wave` / `repair_deep_wave` / `repair_wave`) into `source_repair --mode batch` **only after** parity tests for each wave’s JSON report fields.

**Exit:** one recommended entrypoint per capability in skill; deprecated scripts print redirect but still work.

### 11.7 Phase G — Rust is the core (in progress with A–F)

Operator override: **do not wait until after P3** to introduce Rust.  
Workspace under `crates/` lands in Phase A; each Python module moves to a Rust crate with golden parity before the shim switches. Python remains MCP glue / shim only where PyO3 is not worth it yet. Same JSON schemas (§8). Phase diagram still A→B→C then D/E/F in parallel once P0 fixtures exist.

### 11.8 Work sequencing diagram

```
A inventory+fixtures+contracts
        │
        ▼
B extract packages (behavior freeze)
        │
        ▼
C repair spine + REPORT schema     ──► P0 Parity gate (must pass)
        │
        ├──────────────────────────────► D pattern + adapters (P2)
        │
        └──────────────────────────────► E create/optimize/merge (P3)
                        │
                        ▼
                 F check/video/collapse (P4)
                        │
                        ▼
                 G optional Rust
```

---

## 12. Acceptance plan (incl. 100% Python parity)

### 12.1 What “100% parity” means

Parity is **behavioral**, not “same file layout.”

| Layer | Must match |
|-------|------------|
| **CLI** | Same script name, same flags, same exit code classes (0 ok / non-zero fail) for every `__main__` in `scripts/` |
| **Library API** | Public functions listed in §12.2 still importable from shim modules (re-export OK) |
| **Deterministic PC** | Same inputs → same classify/probe/smell/migrate/patch outputs (golden fixtures) |
| **Device I/O** | Same MCP tool names + equivalent argument shapes for verify/save/disable/get; `checkDiscovery=false` default for repair verify |
| **Ledger / anti fake-fixed** | `repair_claim` / session log still reject `fixed` without success check |
| **Side-effect flags** | `--dry-run`, `--keep-old`, `--no-verify`, `--disable` semantics unchanged |

**Explicitly out of parity (additive only):** new create/optimize/merge/pattern CLIs, new REPORT `capability` field (old consumers must ignore unknown keys), adapter-only faster paths that still produce equivalent final BookSource fields for the same repair class.

**Forbidden until P0 green:** deleting a script, changing default flag values, changing skip/fixed message classes in a way that breaks skill docs, or claiming parity without running §12.3.

### 12.2 Parity matrix (every current script)

Status codes: `shim` = thin wrapper; `lib` = library-only re-export; `keep` = stays as orchestration; `dep` = deprecated shim to another CLI.

| Script | Surface to preserve | Target | Acceptance check |
|--------|---------------------|--------|------------------|
| `mcp_client.py` | `resolve_endpoint`, `get_source`, `save_source`, `disable_source`, `tools_call`, trim-URL retry | `source_mcp` | unit: get_source trim/space variants |
| `mcp_channel.py` | `acquire`/`release`/`assert_idle_for_repair`/`status` | `source_mcp.channel` | unit: lock stale detection |
| `mcp_discover.py` | CLI `--write`/`--timeout`; `ensure_reachable` | `source_mcp.discover` | smoke: discover returns URL or exits non-zero cleanly |
| `repair_prefilter.py` | CLI flags; `classify_one` L0/L1/L2; wall sniff | `source_core.gate` | golden: parked/password/DB/migrate fixtures |
| `precheck_sources.py` | CLI DNS/HTTP bulk probe JSON | `source_core.precheck` | golden or snapshot JSON schema |
| `repair_search_probe.py` | `probe_search_forms`, ranking, fake-home penalty, form-5xx dead | `source_core.probe` | golden HTML fixtures |
| `repair_diagnose.py` | CLI `--url/--key/--out`; L2 before debug | `source_core.diagnose` + CLI | fixture + flag smoke |
| `repair_debug_parse.py` | `parse_debug_text`, `layer_from_check_message` | `source_core.debug_parse` | golden debug dumps |
| `repair_debug_vs_check.py` | CLI compare classify | keep orchestration | smoke `--help` + offline classify unit |
| `repair_rule_smells.py` | `apply_safe_rule_fixes`, webView quotes | `source_core.patch` | golden sources |
| `repair_patches.py` | `apply_auto_patches`, `patch_plan` | `source_core.patch` | golden sources |
| `repair_helpers.py` | `layer_for_fail`, `smell_rules`, `header_map`, `fetch_page` | `source_core.helpers` | unit |
| `repair_check.py` | `is_repair_success`, `strip_discovery_failures`, `check_args` | `source_core.check` | unit: discovery-ignored success |
| `repair_wait.py` | `wait_check`, `fetch_all_results` | `source_core.check` | unit with mocked MCP |
| `repair_cache.py` | HTML cache, EWMA cooldown, triage cache | `source_core.cache` | unit temp dir |
| `repair_claim.py` | `assert_fixed_allowed` | `source_core.ledger` | unit: reject without success |
| `repair_session_log.py` | CLI append/show; `append_row` | `source_core.ledger` | unit JSONL |
| `repair_progress.py` | `status`/`next` + L2-gate before return | keep CLI → core | CLI smoke + next skips walls |
| `repair_classify.py` | `decide`, `classify_resolved_url`, `queue_sort_key` | `source_core.queue` | golden |
| `repair_queue.py` | CLI load/sort | keep | `--help` + sample file |
| `repair_why_wave.py` | bucket + report JSON | keep/shim | snapshot buckets |
| `repair_knowledge.py` | `search_knowledge` | `source_core.knowledge` | unit query hit |
| `repair_one.py` | all CLI flags incl. dry-run/no-verify | spine shim | flag smoke + dry-run no MCP write |
| `repair_deep_loop.py` | `--mode oneshot\|batch`, REPORT stream | spine shim | oneshot emits `REPORT_JSON` |
| `repair_goal15_run.py` | dep → deep_loop batch | dep | still exits 0 calling deep_loop |
| `repair_wave.py` | CLI workers/timeout/out fields | shim→batch | report keys subset stable |
| `repair_search_wave.py` | same | shim→batch | report keys subset stable |
| `repair_deep_wave.py` | same | shim→batch | report keys subset stable |
| `repair_harvest.py` | fails file / limit / keyword | keep/shim | dry path with empty queue |
| `repair_domain_migrate.py` | `--from-url/--to-url/--verify/--keep-old/--dry-run` | `source_core.migrate` | golden rewrite + dry-run |
| `repair_domain_hunt.py` | seeds hunt | `source_core.hunt` | fixture seeds |
| `repair_source.py` | subcommands triage/fetch/verify/log/channel/index | umbrella shim | each subcommand `--help` |
| `repair_bench10.py` | bench report shape | keep | `--help` |
| `batch_check_mcp.py` | batch check + classify dump | `source_check` | `--help` + mocked classify |
| `full_check_runner.py` | orchestration entry | shim | `--help` |
| `disable_dead_sources.py` | `--precheck-json/--disable/--tag/--limit` | `source_check` | dry without `--disable` |
| `shard_urls.py` | consistent hash ring | `source_check.shard` | golden URL→node map |
| `video_prefilter.py` | route table | `source_video` | golden routes |
| `video_repair_one.py` | smell + one-shot skeleton | `source_video` | `--help` + dry |
| `debugger/*` | analyze_rule/url CLI paths | `source_parse` wrap | existing debugger tests still pass |
| `parity_inventory.py` | script inventory write/check vs §12.2 | keep | `--write` + `--check` (warn-only default) |
| `parity_selftest.py` | fixtures/cli-help/imports/schemas/inventory/rust-cli/search-parity suites | keep | `python scripts/parity_selftest.py` exit 0 |
| `parity_rust_suite.py` | Rust CLI golden (diagnose/probe/migrate/hunt/gate) | keep | imported by selftest `--suite rust-cli` |
| `parity_search_suite.py` | search-layer form golden vs `source-cli probe` | keep | `--suite search-parity` |
| `repair_retro.py` | per-source retro JSONL append | keep | `--help` + append dry |
| `repair_rt_queue.py` | respondTime queue build | keep | `--help` / dry build |
| `repair_serial.py` | serial oneshot from RT queue | keep/shim→spine | `--help` + dry limit |
| `source_gate_rs.py` | thin shim → `source-cli gate` | keep | `--help` + L0-only default |
| `repair_refresh_phone_index.py` | refresh phone source index helper | keep | `--help` |

**48/48 scripts** must appear above. If a new script is added, update this matrix in the same PR.

### 12.3 Acceptance procedure (run in order)

```text
1) Inventory drift
   python scripts/parity_inventory.py --check
   # fails if scripts/*.py not in matrix / inventory JSON

2) Deterministic golden
   python scripts/parity_selftest.py --suite fixtures
   # must match committed expected/ (Phase A baseline)

3) CLI surface
   python scripts/parity_selftest.py --suite cli-help
   # every __main__ script: --help exits 0 (or documented exception)

4) Shim import
   python scripts/parity_selftest.py --suite imports
   # public symbols from §12.2 still importable

5) Contract schemas
   python scripts/parity_selftest.py --suite schemas
   # sample REPORT_JSON / GateResult / PatchPlan validate

6) Live device smoke (requires MCP; skip in CI if offline)
   # THOROUGH: must include one layer=search broken URL → repair → 校验成功
   # OR evidence-backed skip (search_endpoint_dead / wall). layer=ok-only is NOT enough.
   source-cli diagnose --url <search-fail-url>
   source-cli repair --mode oneshot --url <search-fail-url>
   # plus optional known-ok: diagnose layer=ok → verify

7) Sign-off
   Write docs/parity/ACCEPTANCE_LOG.md only if docs/parity/THOROUGH_ACCEPTANCE.md gates pass
```

**Thorough functional criterion:** steps 1–5 green + `search-parity` suite + step 6 search-layer E2E. Soft CLI inventory alone is **not** sign-off.  
**Cutover criterion:** thorough functional + §12.6 performance + explicit operator sign-off.

**P2/P3 product criterion (beyond parity):**

- Pattern extract: ≥K families size≥3 from verify-ok library.
- Create known-family URL → device 校验成功.
- Merge: duplicate host count ↓; winner verifies.
- Optimize A/B: fail rate not up vs before.
- Repair oneshot: walls fail-fast; 搜索口不对 still attempts form fix.

### 12.4 Regression policy

| Change type | Required tests |
|-------------|----------------|
| Touch gate/probe/patch/migrate | update or add golden fixture in same PR |
| Touch MCP client | mock unit + live-smoke optional |
| Collapse wave scripts | report-key subset test + one batch dry-run |
| New adapter | fixture HTML for family + at least one ledger-verified URL |
| Claim “fixed” path | must go through `assert_fixed_allowed` |

### 12.5 Success metrics (product)

- Parity suites 1–5 always green on main.
- Pattern extract ≥K families; Identify ≥80% on exemplars.
- No new `inspect_*.py`; no capability logic only in chat.
- Skill documents only spine + listed shims **after** cutover; before cutover Skill keeps current script CLIs.

### 12.6 Performance parity (required for cutover)

Functional parity (§12.1–12.3) alone is **not** enough to switch Skill/work-context defaults.

| Scenario | Baseline | New stack must |
|----------|----------|----------------|
| L2 wall skip (parked / password / DB) | current `repair_prefilter` / diagnose fail-fast | wall → skip in ≤ baseline p95 (target &lt;5s oneshot) |
| Oneshot deep attempt (no wall) | `repair_deep_loop --mode oneshot` wall-clock on same URL | ≤ 1.2× baseline p95 (or document accepted regression) |
| Batch N URLs stream REPORT | current deep_loop batch | first `REPORT_JSON` latency ≤ 1.2× baseline; no worse throughput than baseline on same machine/MCP |
| Gate-only classify (PC) | `repair_prefilter` on fixture URL list | ≤ 1.1× baseline CPU wall time |
| MCP verify round-trip | `repair_wait` / check helpers | no extra full-check polls vs current defaults |

Procedure:

1. Record baselines with `repair_bench10.py` and/or timed oneshot on a fixed URL set → `docs/parity/PERF_BASELINE.json` (git sha + machine note).
2. Re-run same set on new entrypoints → `docs/parity/PERF_CANDIDATE.json`.
3. `parity_selftest.py --suite perf` fails if any row exceeds the ratios above without an approved waiver in `ACCEPTANCE_LOG.md`.

No cutover while perf suite red or baseline missing.

---

## 13. Non-goals (v1)

- Rebuilding full LangGraph 驯兽师 monolith.
- Auto-solving captcha/CF without user.
- RSS/replace-rule full platform (document only until novel path is solid).
- Claiming 100% parity without §12.3 steps 1–5 green **and** (for cutover) §12.6 green.
- Behavior changes disguised as “refactors” in Phase B.
- Switching Skill / work-context to new CLIs before cutover sign-off.
- A second BookSource field vocabulary parallel to App JSON (§3).
- Storing HTML bodies or phone library rows in PC SQLite (§9.5).
- Rewriting search rules from rate-limit HTML; EWMA cooldown only (§10.3).
- Treating wave `concurrentRate`-only patches as `fixed` (§3.6).

---

## 14. Design constraints (SOLID + Dev Practices)

This section closes the architecture gaps that types/contracts/DB alone do not cover.  
**Implementers MUST follow these constraints**; product verbs in §2 do not override them.

### 14.1 Layering & dependency direction (DIP)

```text
                    ┌──────────── CLIs (Rust bins / Python shims) ────────────┐
                    │  wiring only: load config, construct ports, run spine   │
                    └───────────────────────┬────────────────────────────────┘
                                            │ depends on
                    ┌───────────────────────▼────────────────────────────────┐
                    │              spine / capability services                 │
                    │     (orchestrate; no reqwest/rusqlite/MCP HTTP here)    │
                    └───────────────────────┬────────────────────────────────┘
                                            │ depends on
          ┌─────────────┬───────────────────┼───────────────────┬────────────┐
          ▼             ▼                   ▼                   ▼            ▼
     gate/probe     patch/identify     adapters (family)    contracts     types
     merge/pattern  optimize           (pure propose)       validate      enums
          │             │                   │
          └─────────────┴───────────────────┘
                          │ use traits only
          ┌───────────────▼───────────────┐
          │  ports (traits): SourceRepo,  │
          │  VerifyPort, HtmlFetchPort,   │
          │  LedgerPort, Clock, Channel   │
          └───────────────┬───────────────┘
                          │ implemented by
          ┌───────────────▼───────────────┐
          │  adapters_infra: mcp, sqlite, │
          │  http_fetch, fs_html_cache    │
          └───────────────────────────────┘
```

**Forbidden imports**

| Crate / module | Must NOT import |
|----------------|-----------------|
| `source-types`, `source-contracts` | mcp, db, http, spine |
| family adapters (`source-adapters-*`) | `mcp_client`, rusqlite, reqwest |
| `source-spine` | concrete MCP/SQLite types — only port traits |
| infra mcp/db | spine, adapters (family) |

Lower modules must not depend on upper modules (Dev Practices).

### 14.2 Ports (hexagonal)

```text
trait SourceRepository {
  fn get(&self, key: &SourceKey) -> Result<BookSource, PortError>;
  fn save(&self, source: &BookSource) -> Result<(), PortError>;
  fn disable(&self, key: &SourceKey) -> Result<(), PortError>;
  fn delete(&self, keys: &[SourceKey]) -> Result<(), PortError>;
}

trait VerifyPort {
  fn check(&self, key: &SourceKey, opts: CheckOpts) -> Result<VerifyResult, PortError>;
  // CheckOpts.check_discovery default false
}

trait HtmlFetchPort {
  fn fetch(&self, url: &Url, headers: &HeaderMap) -> Result<FetchResult, PortError>;
}

trait LedgerPort {
  fn append(&self, row: &LedgerRow) -> Result<(), PortError>;
}

trait ChannelPort {
  fn assert_idle_for_repair(&self) -> Result<(), PortError>;
  fn acquire_repair(&self) -> Result<Guard, PortError>;
}

trait Clock {
  fn now_utc(&self) -> DateTime<Utc>;
  fn sleep(&self, d: Duration);
}
```

CLI constructs concrete infra and injects into spine. Tests inject fakes.

### 14.3 Adapter ISP (split protocols)

Do **not** force every family to implement create+repair+optimize.

```text
trait FamilyPlugin {
  fn family(&self) -> SiteFamily;
  fn fingerprints(&self) -> &[FingerprintRule];
}

trait RepairPlugin: FamilyPlugin {
  fn repair(&self, ctx: &RepairContext) -> AdapterOutcome<PatchPlan>;
}

trait CreatePlugin: FamilyPlugin {
  fn create(&self, ctx: &RepairContext) -> AdapterOutcome<PatchPlan>;
}

trait OptimizePlugin: FamilyPlugin {
  fn optimize(&self, ctx: &RepairContext) -> Option<OptimizePlan>;
}

enum AdapterOutcome<T> { Plan(T), NeedMoreHtml(NeedMoreHtml), Unrepairable(Unrepairable) }
```

Registry maps `SiteFamily → dyn FamilyPlugin` plus optional repair/create/optimize handles.  
`GenericForm` implements `RepairPlugin`/`CreatePlugin` only for `searchUrl` from form — never invents `bookList`.

### 14.4 RepairContext (no MCP bag)

```text
RepairContext {
  source_key: SourceKey,
  source: BookSource,              // snapshot at start
  gate: Option<GateResult>,
  diagnose: Option<DiagnoseResult>,
  family: SiteFamily,
  html: HashMap<Url, Bytes>,       // pre-fetched pages only
  config: RepairConfig,
  dry_run: bool,
  // NO SourceRepository / VerifyPort here — apply/verify stay in spine
}
```

### 14.5 Apply, idempotency, failure matrix

`ApplyService` (spine-adjacent):

1. `validate_patch_plan(plan)` (contracts) — reject empty/illegal ops early.
2. If `dry_run` → return plan, no save.
3. Snapshot `before = repo.get(key)`.
4. Apply ops → `after`.
5. `repo.save(after)` unless dry_run.
6. `verify.check(key)` unless `--no-verify`.
7. Ledger + REPORT.

**Idempotency key:** `sha256(source_key + canonical_ops_json)`; repeated oneshot with same key + already `verify.success` may short-circuit to REPORT fixed without re-patch.

| Stage fail | Action |
|------------|--------|
| validate | no save; REPORT failed |
| save | no verify; REPORT failed; leave phone unchanged if save errored |
| verify fail after save | keep after (repair attempt); REPORT failed; ledger note `verify_failed_after_save`; do **not** claim fixed |
| channel busy | no MCP; REPORT failed `channel_busy` |

Rollback to `before` only when explicitly `--rollback-on-verify-fail` (default **off** — matches current Python caution).

### 14.6 Error model ↔ REPORT / exit

| Kind | Examples | REPORT status | Process exit (oneshot) |
|------|----------|---------------|------------------------|
| `Transient` | MCP timeout, HTTP 429 mid-fetch | failed | 2 (retryable) |
| `Permanent` | L2 wall, parked, search_endpoint_dead | skipped | 0 (expected skip) or 3 |
| `ContractViolation` | bad PatchPlan, fake-fixed | failed | 4 |
| `ChannelBusy` | bulk holds MCP | failed | 5 |
| Success mutate | verify.success | fixed/created/… | 0 |

Exact exit codes must stay stable once published in parity matrix; document in `source-cli --help`.

### 14.7 RepairConfig (single config surface)

| Key | Default | Used by |
|-----|---------|---------|
| `identify_min_score` | 2.0 | identify |
| `identify_margin` | 0.5 | identify |
| `cluster_min_size` | 3 | pattern |
| `ewma_alpha` | 0.3 | cache |
| `default_gap_s` | 3.0 | cache |
| `l1_timeout_s` | 1.5 | gate |
| `l2_timeout_s` | 4.0 | gate |
| `gate_concurrency` | 32 | gate batch |
| `check_discovery` | false | verify |
| `soft_budget_s` | 240 | spine |
| `hard_budget_s` | 300 | spine |
| `dual_write_sqlite` | true (after Phase B) | ledger |

Load order: defaults → `config/repair_config.json` → env `REPAIR_*` → CLI flags.

### 14.8 BookSource typing rule (Any escape hatch)

- Wire format remains App JSON (`serde_json::Value` / opaque map) at MCP boundary.
- **Domain code** mutates only via `PatchOp` + typed path getters/setters (`search_url()`, `rule_search_book_list()`, …). Minimal getters landed on `BookSource` in `source-types`; expand as adapters need them.
- Do not pass bare `dict` through public Rust APIs without `BookSource` newtype. Python shims may keep `dict` until cutover.

**Adapters ↔ spine wiring (2026-07-27):** `RepairContext` lives in `source-types`; plugin traits + `IdentifyPort` in `source-ports`. `AdapterRegistry` implements identify and is injected via `RegistryRepairPlugin` into `run_repair_oneshot`. CLI: `source-cli repair` / `repair-dry`. Not a full §12 cutover.

### 14.9 Observability

Every spine run logs JSON-lines fields: `ts`, `source_key`, `capability`, `step`, `duration_ms`, `family`, `layer`, `cache_hit`, `report_status`.  
Console stays human-readable; files under `temp/full_fix/logs/` (or `MYFORGE_LOG_DIR` if set).  
MCP wait time and gate stage timings are mandatory for §12.6 perf diagnosis.

### 14.10 Concurrency

- One MCP check/debug per device (`ChannelPort`).
- PC gate/probe may parallelize across hosts; same `HostKey` respects EWMA gap.
- SQLite: WAL + `busy_timeout=5000`; host_stats row updates are single-statement UPSERT (no whole-file rewrite).
- HTML cache: read TTL on get; **periodic prune** job deletes meta+bin older than max_age even with no new reads (coding-patterns: idle expiry).
- Two agents must not deep-repair the same `SourceKey` concurrently — claim lease in DB optional (`claims` or `lease` table); v1: document + ledger `divert`.

### 14.11 Video / file bounded context

`bookSourceType` 3/4 use `source-video` plugins and the same ports/contracts, but **do not** share novel TOC adapters. Spine switches on type before Identify novel families.

### 14.12 Test pyramid

| Layer | What | Owner |
|-------|------|-------|
| Unit | pure gate L0, probe score, EWMA, structural_hash, patch apply | Rust `#[test]` / fixtures |
| Contract | JSON Schema samples | `parity_selftest --suite schemas` |
| Golden | PC deterministic outputs vs committed expected/ | fixtures + Rust CLI |
| Port fake | spine with in-memory repo/verify | Rust integration |
| Live smoke | one device URL | optional CI |
| Perf | §12.6 baselines | cutover gate |

### 14.13 Template / adapter versioning

`PatternCluster` and `assets/templates/<family>.json` carry `adapter_version: u32`.  
Identify hit with mismatched major version → treat as `Unknown` / GenericForm fallback rather than forcing stale centroid.

---
