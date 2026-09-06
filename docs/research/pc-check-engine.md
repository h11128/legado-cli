# PC Reuse of Legado Book-Source Check — Research

Canonical copy also lives in the app repo:

`E:/Projects/legado/docs/pc-check-engine-research.md`

See that file for the full write-up. Summary:

- **Calling** App check via MCP is easy and already exists.
- **Running** the same Kotlin engine on PC is a multi-week extract (WebView / cookie / `appCtx`).
- This skill’s path: PC precheck + batched MCP check; device remains authoritative.
- Device schedulers: AIMD / token bucket / work-stealing / Bloom / EWMA (see app `model/checkalgo/`).
- Multi-device URL split helper: `scripts/shard_urls.py`.
