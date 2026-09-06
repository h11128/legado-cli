# Repair pipeline design (画本 + 御书屋 / wave20)

## What went wrong

| Approach | Result |
|----------|--------|
| `repair_wave` auto-patch mostly `concurrentRate` | Almost no real fixes |
| Treat 「搜索目录失效」 as search broken | Often wrong (画本 = TOC) |
| Treat books=1 + empty toc as TOC | Often wrong (**wmp8 = fake detail / dead search**) |
| Agent skips checklist / improvises HTML | Misses form in JS (`search_win`) |

## What worked

| Source | Diagnose truth | Fix |
|--------|----------------|-----|
| 画本 `so.ihuaben.com` | search OK, toc empty | tocUrl list + `.chapter-row` |
| 御书屋 `m.wmp8.com` | fake_detail (`s.php`), search 404 | `modules/article/search.php` + `#sitebox dl` + 目录 toc + `#YiJianZhan` |

## Target architecture

```
                ┌─────────────────┐
   URL list ──► │ TRIAGE (wave)   │  needs_deep by layer
                └────────┬────────┘
                         │
                         ▼
              Deep-fix CHECKLIST (skill)
              channel → diagnose → branch → verify → ledger
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
      search           toc            content
   (+ form probe)   (real detail)   (chapter HTML)
```

### Script roles

| Role | Script | Must not |
|------|--------|----------|
| Triage | `repair_wave.py` | Claim rate-only fixed |
| Diagnose | `repair_diagnose.py` | Skip fake_detail reclassify |
| Form probe | `repair_search_probe.py` | Ignore JS-injected forms |
| Parse | `repair_debug_parse.py` | Treat fake detail as toc |

### Skill rules

1. **Checklist is mandatory** — not optional flavor text.
2. **Layer before selector** — including fake_detail → search.
3. **Fetch the failing page** — search → home+JS+result; toc → detail/list.
4. **发现 off** unless asked.
5. Budgets: 2–3 min deep; ≤1 min known fields; hard stop 5 min.
