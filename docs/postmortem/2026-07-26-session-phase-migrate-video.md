# Session phase log — migrate + video/file (2026-07-26 evening)

Append-only chronicle for the “继续 / 换域 / 影视 flow” slice.  
Parent chat: `f14f2834-eeb7-45bb-b325-9ba29e01c2db`.  
Retro: `docs/source-repair-retro-migrate-video-2026-07-26.md`.

Use `python scripts/repair_session_log.py` for new lines going forward.

---

## Timeline

| UTC~ / local | Step | Result | Note |
|--------------|------|--------|------|
| ~20:04 | Domain hunt trial | zxcs/aiqu migrate candidates; tiexue no_mirror; dddw weak | L2 only |
| ~20:06 | Video skill + video_prefilter + L0 `video`/`hunt` | Divert path created | Infra |
| ~20:09 | User: 继续 | Start device migrate | Scope: migrate+video |
| ~20:11 | `repair_domain_migrate` zxcs→zxcs.click | save+delete OK; verify **发现/搜索失效** | Old rules |
| ~20:12 | migrate aiqu→aiqu226; zxcs##@鱼 | same fail pattern | |
| ~20:12 | debug zxcs search | list empty; home==search HTML | Wrong searchUrl |
| ~20:13 | Find form `/search?q=` + `book-list` | HTML truth | Should be first after migrate |
| ~20:14 | Patch zxcs search/explore | debug list=10 | |
| ~20:14 | Patch aiqu `.search-card` | list=20 name broken → fix a.searchtitle | |
| ~20:15 | aiqu content `.layui-code` empty on .txt | → `body@text` | |
| ~20:15 | **zxcs + aiqu 校验成功** | Device | Good |
| ~20:16 | List device video URLs | taopian / ukuzy / nanguady | |
| ~20:16 | U酷 debug: search OK, 下载链接为空 | type=3 file needs downloadUrls | |
| ~20:17–20:20 | Loop: `\|\|` `@css` JS downloadUrls | debug OK / check fail ~108ms | **Detour** |
| ~20:21 | HTTP logs: check often **search only** | infoHtml trap | Root cause |
| ~20:21 | Harden bookUrl + `input[name=copy_sel]@value` | **U酷 校验成功** | |
| ~20:22 | User: 为什么这么久 + 要记录反思改进 | This log + retro + scripts | |

---

## Per-source final

| URL | Status | Root fix |
|-----|--------|----------|
| https://www.zxcs.click/ | FIXED verified | migrate + `/search?q=` + book-list/li explore |
| https://www.zxcs.click##@鱼 | patched same rules | not re-verified alone |
| https://www.aiqu226.com/ | FIXED verified | migrate + search-card + body@text |
| https://ukuzy.com/ | FIXED verified | downloadUrls + bookUrl detail |
| taopianzy… | OPEN | empty search list / SSL on PC |
| nanguady.cc | OPEN | type0 + empty search |

---

## Reflection (one paragraph)

本阶段最大浪费是 **把「debug 有下载链接、check 没有」当成选择器语法问题反复试**，而不是看 HTTP 是否打开了详情页。其次是 **换域后不先看新站搜索表单就 verify**。再次是 **一次「继续」塞了基建+两小说+三影视**。以后：每源 session_log 一行、migrate 后强制 HTML、debug≠check 先跑 `repair_debug_vs_check.py`、目标 2–3 分钟 / 硬停 5 分钟。
