//! stdio ⇄ phone HTTP MCP bridge (`source-cli mcp bridge`).
//!
//! Agent configs (Claude / Cursor / Codex) launch this command instead of
//! pinning the phone's DHCP address. Each JSON-RPC line from stdin is POSTed to
//! the endpoint in `config/mcp_defaults.json`; when the phone is unreachable the
//! bridge runs `ensure_reachable` (remembered endpoints → adb/ARP/subnet scan),
//! replays the client's `initialize` on the new address and retries once.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use source_types::PortError;

use crate::discover::{ensure_reachable, probe_mcp};
use crate::endpoint::McpEndpoint;
use crate::timeouts::McpTimeouts;

const PROBE_TIMEOUT_S: f64 = 2.0;
const DISCOVER_TIMEOUT_S: f64 = 3.0;

#[derive(Debug)]
enum SendErr {
    /// HTTP status from a reachable server (bad token, expired session…).
    Status(u16, String),
    /// Connect / DNS / read failure — the phone may have moved.
    Transport(String),
}

type Rediscover = dyn Fn() -> Result<McpEndpoint, PortError> + Send + Sync;
type Probe = dyn Fn(&McpEndpoint) -> bool + Send + Sync;

struct Conn {
    endpoint: McpEndpoint,
    session: Option<String>,
    /// Bumped on every re-init so concurrent failures recover only once.
    generation: u64,
}

pub struct Bridge {
    conn: Mutex<Conn>,
    /// Client's original `initialize` request, replayed after a reconnect.
    init: Mutex<Option<Value>>,
    timeout: Duration,
    rediscover: Box<Rediscover>,
    probe: Box<Probe>,
}

impl Bridge {
    /// Production bridge backed by `config/mcp_defaults.json` at `path`.
    pub fn from_defaults(path: &Path) -> Self {
        let t = McpTimeouts::load_path(path);
        let timeout = Duration::from_secs_f64(t.http_timeout_s.max(t.debug_timeout_s).max(1.0));
        // Unreadable defaults → placeholder; the first request fails over to discovery.
        let endpoint = McpEndpoint::load_path(path)
            .unwrap_or_else(|_| McpEndpoint::new("http://127.0.0.1:1236/mcp", "1234"));
        let owned: PathBuf = path.to_path_buf();
        Self::new(
            endpoint,
            timeout,
            Box::new(move || ensure_reachable(Some(&owned), DISCOVER_TIMEOUT_S)),
            Box::new(|ep| probe_mcp(&ep.mcp_url, &ep.token, PROBE_TIMEOUT_S)),
        )
    }

    fn new(
        endpoint: McpEndpoint,
        timeout: Duration,
        rediscover: Box<Rediscover>,
        probe: Box<Probe>,
    ) -> Self {
        Self {
            conn: Mutex::new(Conn {
                endpoint,
                session: None,
                generation: 0,
            }),
            init: Mutex::new(None),
            timeout,
            rediscover,
            probe,
        }
    }

    pub fn endpoint(&self) -> McpEndpoint {
        self.conn.lock().expect("conn lock").endpoint.clone()
    }

    /// Forward one client message; returns the JSON lines to write back.
    pub fn handle(&self, msg: &Value) -> Vec<String> {
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        if method == "initialize" {
            *self.init.lock().expect("init lock") = Some(msg.clone());
        }
        let body = msg.to_string();
        let (ep, session, generation) = self.snapshot();
        match self.post(&ep, session.as_deref(), &body) {
            Ok(out) => self.accept(out, generation),
            Err(first) => match self.recover(generation, &first, method == "initialize") {
                Ok(()) => {
                    let (ep, session, generation) = self.snapshot();
                    match self.post(&ep, session.as_deref(), &body) {
                        Ok(out) => self.accept(out, generation),
                        Err(e) => error_reply(msg, &describe(&e)),
                    }
                }
                Err(reason) => error_reply(msg, &reason),
            },
        }
    }

    fn snapshot(&self) -> (McpEndpoint, Option<String>, u64) {
        let c = self.conn.lock().expect("conn lock");
        (c.endpoint.clone(), c.session.clone(), c.generation)
    }

    fn accept(&self, (sid, lines): (Option<String>, Vec<String>), generation: u64) -> Vec<String> {
        if let Some(sid) = sid {
            let mut c = self.conn.lock().expect("conn lock");
            if c.generation == generation {
                c.session = Some(sid);
            }
        }
        lines
    }

    /// Decide whether a failure is worth one retry, and make the retry possible.
    fn recover(&self, generation: u64, err: &SendErr, is_init: bool) -> Result<(), String> {
        let mut c = self.conn.lock().expect("conn lock");
        if c.generation != generation {
            return Ok(()); // another request already reconnected
        }
        match err {
            // Server answered: only an expired session is recoverable.
            SendErr::Status(code, _) if matches!(code, 400 | 404) && c.session.is_some() => {}
            SendErr::Status(..) => return Err(describe(err)),
            SendErr::Transport(_) => {
                if (self.probe)(&c.endpoint) {
                    // Phone still there → likely a slow tool call; never replay it blindly.
                    return Err(describe(err));
                }
                let ep = (self.rediscover)().map_err(|e| {
                    format!("{}; rediscovery failed: {e}", describe(err))
                })?;
                eprintln!("[mcp-bridge] phone moved: {} -> {}", c.endpoint.mcp_url, ep.mcp_url);
                c.endpoint = ep;
            }
        }
        c.session = None;
        c.generation += 1;
        if is_init {
            return Ok(()); // the retried request *is* the initialize
        }
        let init = self.init.lock().expect("init lock").clone();
        let Some(mut init) = init else {
            return Ok(());
        };
        init["id"] = json!("mcp-bridge-reinit");
        let (sid, _) = self
            .post(&c.endpoint, None, &init.to_string())
            .map_err(|e| format!("re-initialize failed: {}", describe(&e)))?;
        c.session = sid;
        let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        let _ = self.post(&c.endpoint, c.session.as_deref(), &note.to_string());
        Ok(())
    }

    fn post(
        &self,
        ep: &McpEndpoint,
        session: Option<&str>,
        body: &str,
    ) -> Result<(Option<String>, Vec<String>), SendErr> {
        let connect = Duration::from_secs_f64(PROBE_TIMEOUT_S * 2.0);
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(connect)
            .timeout(self.timeout)
            .build();
        let mut req = agent
            .post(&ep.mcp_url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json, text/event-stream")
            .set("X-Legado-Token", &ep.token);
        if let Some(sid) = session {
            req = req.set("Mcp-Session-Id", sid);
        }
        match req.send_string(body) {
            Ok(resp) => {
                let sid = resp.header("Mcp-Session-Id").map(str::to_string);
                let text = resp
                    .into_string()
                    .map_err(|e| SendErr::Transport(format!("read body: {e}")))?;
                Ok((sid, split_messages(&text)))
            }
            Err(ureq::Error::Status(code, resp)) => {
                Err(SendErr::Status(code, resp.into_string().unwrap_or_default()))
            }
            Err(e) => Err(SendErr::Transport(e.to_string())),
        }
    }
}

/// Plain JSON body or SSE stream → one JSON-RPC message per element.
fn split_messages(body: &str) -> Vec<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return vec![trimmed.to_string()];
    }
    body.lines()
        .filter_map(|l| l.strip_prefix("data:").map(str::trim))
        .filter(|d| !d.is_empty())
        .map(str::to_string)
        .collect()
}

fn describe(err: &SendErr) -> String {
    match err {
        SendErr::Status(code, body) => {
            format!("phone MCP http {code}: {}", body.chars().take(200).collect::<String>())
        }
        SendErr::Transport(e) => format!("phone MCP unreachable: {e}"),
    }
}

/// JSON-RPC error for requests; notifications get no reply.
fn error_reply(msg: &Value, reason: &str) -> Vec<String> {
    match msg.get("id") {
        Some(id) if !id.is_null() => vec![json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32000, "message": reason},
        })
        .to_string()],
        _ => Vec::new(),
    }
}

/// Serve stdin → phone → stdout until stdin closes. Logs go to stderr only.
pub fn run_stdio_bridge(path: &Path) -> Result<(), PortError> {
    let bridge = Arc::new(Bridge::from_defaults(path));
    eprintln!("[mcp-bridge] endpoint {}", bridge.endpoint().mcp_url);
    let out = Arc::new(Mutex::new(io::stdout()));
    let mut inflight = Vec::new();
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|e| PortError::Transient(format!("stdin: {e}")))?;
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[mcp-bridge] skip non-JSON line: {e}");
                continue;
            }
        };
        let (bridge, out) = (Arc::clone(&bridge), Arc::clone(&out));
        // `initialize` must finish before later requests reuse its session.
        let serial = msg.get("method").and_then(Value::as_str) == Some("initialize");
        let job = move || {
            let replies = bridge.handle(&msg);
            let mut w = out.lock().expect("stdout lock");
            for r in replies {
                let _ = writeln!(w, "{r}");
            }
            let _ = w.flush();
        };
        if serial {
            job();
        } else {
            inflight.retain(|h: &thread::JoinHandle<()>| !h.is_finished());
            inflight.push(thread::spawn(job));
        }
    }
    // Client closed stdin: let in-flight replies drain before exiting.
    for h in inflight {
        let _ = h.join();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Minimal fake phone: answers every POST with `{"id":<id>,"result":{"n":k}}`
    /// over SSE and hands out session `s1`.
    fn fake_phone() -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut s = stream.unwrap();
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                // read headers + body (Content-Length)
                loop {
                    let n = s.read(&mut chunk).unwrap();
                    buf.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&buf).to_string();
                    if let Some(pos) = text.find("\r\n\r\n") {
                        let len = text
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if buf.len() >= pos + 4 + len {
                            break;
                        }
                    }
                }
                let text = String::from_utf8_lossy(&buf).to_string();
                let body = &text[text.find("\r\n\r\n").unwrap() + 4..];
                let req: Value = serde_json::from_str(body).unwrap();
                let k = counter.fetch_add(1, Ordering::SeqCst);
                let reply = if req.get("id").is_some() {
                    format!(
                        "event: message\ndata: {}\n\n",
                        json!({"jsonrpc":"2.0","id":req["id"],"result":{"n":k,"m":req["method"]}})
                    )
                } else {
                    String::new()
                };
                let status = if reply.is_empty() { "202 Accepted" } else { "200 OK" };
                let _ = write!(
                    s,
                    "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nMcp-Session-Id: s1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        (url, hits)
    }

    fn dead_url() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", l.local_addr().unwrap());
        drop(l);
        url
    }

    fn bridge_with(start: String, moved_to: Option<String>) -> Bridge {
        Bridge::new(
            McpEndpoint::new(start, "1234"),
            Duration::from_secs(5),
            Box::new(move || {
                moved_to
                    .clone()
                    .map(|u| McpEndpoint::new(u, "1234"))
                    .ok_or_else(|| PortError::Transient("nothing found".into()))
            }),
            Box::new(|ep| probe_mcp(&ep.mcp_url, &ep.token, 0.5)),
        )
    }

    #[test]
    fn split_messages_plain_sse_and_empty() {
        assert_eq!(split_messages(" {\"a\":1} "), vec!["{\"a\":1}"]);
        assert_eq!(
            split_messages("event: message\ndata: {\"a\":1}\n\ndata: {\"b\":2}\n"),
            vec!["{\"a\":1}", "{\"b\":2}"]
        );
        assert!(split_messages("").is_empty());
    }

    #[test]
    fn error_reply_only_for_requests() {
        let req = json!({"jsonrpc":"2.0","id":7,"method":"tools/list"});
        let r: Value = serde_json::from_str(&error_reply(&req, "x")[0]).unwrap();
        assert_eq!(r["id"], 7);
        assert_eq!(r["error"]["code"], -32000);
        let note = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert!(error_reply(&note, "x").is_empty());
    }

    #[test]
    fn forwards_and_keeps_session() {
        let (url, _) = fake_phone();
        let b = bridge_with(url, None);
        let out = b.handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(b.snapshot().1.as_deref(), Some("s1"));
        // notification → 202, nothing written back
        assert!(b
            .handle(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .is_empty());
    }

    #[test]
    fn dead_endpoint_rediscovers_and_replays_initialize() {
        let (live, hits) = fake_phone();
        let b = bridge_with(dead_url(), Some(live.clone()));
        let out = b.handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
        assert_eq!(serde_json::from_str::<Value>(&out[0]).unwrap()["id"], 1);
        assert_eq!(b.endpoint().mcp_url, live);

        // phone moves again mid-session: tools/list must still succeed after replaying init
        let (live2, hits2) = fake_phone();
        let b2 = bridge_with(dead_url(), Some(live2));
        *b2.init.lock().unwrap() = Some(json!({"jsonrpc":"2.0","id":1,"method":"initialize"}));
        b2.conn.lock().unwrap().session = Some("old".into());
        let out = b2.handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        assert_eq!(v["id"], 2);
        assert_eq!(v["result"]["m"], "tools/list");
        // reinit + initialized + retried request
        assert_eq!(hits2.load(Ordering::SeqCst), 3);
        assert!(hits.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn unreachable_everywhere_returns_jsonrpc_error() {
        let b = bridge_with(dead_url(), None);
        let out = b.handle(&json!({"jsonrpc":"2.0","id":9,"method":"tools/list"}));
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        assert_eq!(v["id"], 9);
        assert!(v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("rediscovery failed"));
    }
}
