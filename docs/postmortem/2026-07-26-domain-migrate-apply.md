# Domain migrate apply (2026-07-26)

Script: `scripts/repair_domain_migrate.py` (save new URL → delete old → optional verify).

## Novel migrations (device-verified)

| Old | New | Result |
|-----|-----|--------|
| http://www.zxcs.info | https://www.zxcs.click/ | **校验成功** after search `/search?q={{key}}` + `book-list`/`book-li` + new explore paths |
| https://www.627txt.com | https://www.aiqu226.com/ | **校验成功** after `.search-card` search + `body@text` content for .txt |

Also enabled `https://www.zxcs.click##@鱼` with same search/explore patches.

## Video flow progress

| Source | URL | Status |
|--------|-----|--------|
| U酷 | https://ukuzy.com/ | **校验成功** — `downloadUrls=input[name=copy_sel]@value` + 加固 bookUrl（避免搜索页当详情） |
| 淘片 | https://taopianzy.com/home/index.html | search list empty; PC SSL hostname mismatch — still open |
| 南瓜 | https://www.nanguady.cc | type=0 + empty search — still open |

Note: App `bookSourceType` **3 = file（下载）**, **4 = video**. 淘片/U酷是 type=3 文件源，验收看 `downloadUrls`/校验成功即可.
