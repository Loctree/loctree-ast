//! End-to-end proof of the bind-aware bearer-auth posture on the HTTP transport.
//!
//! The property under test is fail-safe-by-default: a non-loopback bind with no
//! configured tokens must never become a listening socket. Every case here
//! drives the real binary, so it exercises arg parsing, the startup gate, the
//! axum layer, and both routes (`/context_pack` and the nested `/mcp`).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

const LISTEN_PREFIX: &str = "loctree-mcp http listening on ";

struct TestServer {
    child: Child,
    addr: SocketAddr,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn loopback_without_auth_starts_and_serves_open() {
    let project = sample_project();
    let store = TempDir::new().expect("store dir");
    let server = start_server(
        project.path(),
        &["--bind", "127.0.0.1:0"],
        &store.path().join("tokens.json"),
        &[],
    );

    let response = http_get(server.addr, &context_pack_path(project.path()), None);
    assert_eq!(
        response.status, 200,
        "loopback with no tokens must keep the zero-config UX; body: {}",
        response.body
    );
}

#[test]
fn non_loopback_without_auth_refuses_to_start() {
    let store = TempDir::new().expect("store dir");
    let outcome = run_to_completion(&[
        "--transport",
        "http",
        "--bind",
        "0.0.0.0:0",
        "--token-store",
        &store.path().join("tokens.json").display().to_string(),
    ]);

    assert!(
        !outcome.success,
        "a non-loopback bind with no tokens must not start; stdout={} stderr={}",
        outcome.stdout, outcome.stderr
    );
    assert!(
        outcome.stderr.contains("refusing to start"),
        "stderr: {}",
        outcome.stderr
    );
    assert!(
        outcome.stderr.contains("--allow-unauthenticated"),
        "the refusal must name the explicit override; stderr: {}",
        outcome.stderr
    );
    assert!(
        outcome.stderr.contains("token create"),
        "the refusal must name the way to mint a token; stderr: {}",
        outcome.stderr
    );
    assert!(
        !outcome.stdout.contains(LISTEN_PREFIX),
        "no socket may be announced on the refusal path; stdout: {}",
        outcome.stdout
    );
}

#[test]
fn non_loopback_with_token_gates_both_routes() {
    let project = sample_project();
    let store = TempDir::new().expect("store dir");
    let store_path = store.path().join("tokens.json");
    let token = mint_token(&store_path, "gate");

    let server = start_server(project.path(), &["--bind", "0.0.0.0:0"], &store_path, &[]);
    let addr = loopback_of(server.addr);
    let path = context_pack_path(project.path());

    let missing = http_get(addr, &path, None);
    assert_eq!(missing.status, 401, "body: {}", missing.body);
    assert!(
        missing.headers_lower.contains("www-authenticate: bearer"),
        "401 must advertise the bearer scheme; head: {}",
        missing.headers_lower
    );

    let wrong = http_get(addr, &path, Some("loct_definitely_not_a_real_token"));
    assert_eq!(wrong.status, 401, "body: {}", wrong.body);

    let malformed = http_get(addr, &path, Some(""));
    assert_eq!(malformed.status, 401, "body: {}", malformed.body);

    let ok = http_get(addr, &path, Some(&token));
    assert_eq!(
        ok.status, 200,
        "a valid bearer must pass; body: {}",
        ok.body
    );

    // The nested /mcp service is behind the same layer, not just /context_pack.
    let mcp_unauthed = http_get(addr, "/mcp", None);
    assert_eq!(
        mcp_unauthed.status, 401,
        "the MCP route must be gated too; body: {}",
        mcp_unauthed.body
    );
}

#[test]
fn loopback_with_tokens_configured_also_enforces() {
    let project = sample_project();
    let store = TempDir::new().expect("store dir");
    let store_path = store.path().join("tokens.json");
    let token = mint_token(&store_path, "local");

    let server = start_server(project.path(), &["--bind", "127.0.0.1:0"], &store_path, &[]);
    let path = context_pack_path(project.path());

    assert_eq!(http_get(server.addr, &path, None).status, 401);
    assert_eq!(http_get(server.addr, &path, Some(&token)).status, 200);
}

#[test]
fn explicit_flag_is_the_only_way_to_an_open_non_loopback_port() {
    let project = sample_project();
    let store = TempDir::new().expect("store dir");
    let server = start_server(
        project.path(),
        &["--bind", "0.0.0.0:0", "--allow-unauthenticated"],
        &store.path().join("tokens.json"),
        &[],
    );

    let response = http_get(
        loopback_of(server.addr),
        &context_pack_path(project.path()),
        None,
    );
    assert_eq!(
        response.status, 200,
        "the explicit override must actually open the port; body: {}",
        response.body
    );
}

#[test]
fn explicit_env_override_matches_the_flag() {
    let project = sample_project();
    let store = TempDir::new().expect("store dir");
    let server = start_server(
        project.path(),
        &["--bind", "0.0.0.0:0"],
        &store.path().join("tokens.json"),
        &[("LOCTREE_MCP_ALLOW_UNAUTHENTICATED", "1")],
    );

    let response = http_get(
        loopback_of(server.addr),
        &context_pack_path(project.path()),
        None,
    );
    assert_eq!(response.status, 200, "body: {}", response.body);
}

#[test]
fn shared_env_token_satisfies_the_gate_without_a_token_file() {
    let project = sample_project();
    let store = TempDir::new().expect("store dir");
    let server = start_server(
        project.path(),
        &["--bind", "0.0.0.0:0"],
        &store.path().join("tokens.json"),
        &[("LOCTREE_MCP_AUTH_TOKEN", "shared-secret")],
    );

    let addr = loopback_of(server.addr);
    let path = context_pack_path(project.path());
    assert_eq!(http_get(addr, &path, None).status, 401);
    assert_eq!(http_get(addr, &path, Some("nope")).status, 401);
    assert_eq!(http_get(addr, &path, Some("shared-secret")).status, 200);
}

#[test]
fn revoked_tokens_stop_working_after_a_restart() {
    let project = sample_project();
    let store = TempDir::new().expect("store dir");
    let store_path = store.path().join("tokens.json");
    let token = mint_token(&store_path, "temporary");
    mint_token(&store_path, "keeper");

    let revoke = run_to_completion(&[
        "token",
        "revoke",
        "--id",
        "temporary",
        "--token-store",
        &store_path.display().to_string(),
    ]);
    assert!(revoke.success, "stderr: {}", revoke.stderr);

    let server = start_server(project.path(), &["--bind", "127.0.0.1:0"], &store_path, &[]);
    let path = context_pack_path(project.path());
    assert_eq!(
        http_get(server.addr, &path, Some(&token)).status,
        401,
        "a revoked token must not authenticate"
    );
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

struct HttpResponse {
    status: u16,
    headers_lower: String,
    body: String,
}

struct RunOutcome {
    success: bool,
    stdout: String,
    stderr: String,
}

fn sample_project() -> TempDir {
    let tmp = TempDir::new().expect("temp dir");
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"http-auth-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    fs::create_dir_all(tmp.path().join("src")).expect("src dir");
    fs::write(
        tmp.path().join("src/lib.rs"),
        "pub fn alpha() -> &'static str { beta() }\nfn beta() -> &'static str { \"beta\" }\n",
    )
    .expect("write lib.rs");
    tmp
}

/// Run the binary to completion (no server), returning captured output.
fn run_to_completion(args: &[&str]) -> RunOutcome {
    let output = Command::new(env!("CARGO_BIN_EXE_loctree-mcp"))
        .args(args)
        .env("LOCT_ALLOW_NON_GIT_ROOT", "1")
        .env_remove("LOCTREE_MCP_ALLOW_UNAUTHENTICATED")
        .env_remove("LOCTREE_MCP_AUTH_TOKEN")
        .env_remove("LOCTREE_MCP_TOKEN_STORE")
        .stdin(Stdio::null())
        .output()
        .expect("run loctree-mcp");

    RunOutcome {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

/// Mint a token through the operator CLI and return its plaintext.
fn mint_token(store_path: &Path, id: &str) -> String {
    let outcome = run_to_completion(&[
        "token",
        "create",
        "--id",
        id,
        "--scope",
        "context-read",
        "--token-store",
        &store_path.display().to_string(),
    ]);
    assert!(
        outcome.success,
        "token create failed: stdout={} stderr={}",
        outcome.stdout, outcome.stderr
    );

    let token = outcome
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("token:"))
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| panic!("no token line in: {}", outcome.stdout));
    assert!(token.starts_with("loct_"), "unexpected token: {token}");
    token
}

fn start_server(
    project: &Path,
    extra_args: &[&str],
    store_path: &Path,
    env: &[(&str, &str)],
) -> TestServer {
    let mut command = Command::new(env!("CARGO_BIN_EXE_loctree-mcp"));
    command
        .args(["--transport", "http", "--log-level", "error"])
        .args(extra_args)
        .args(["--token-store", &store_path.display().to_string()])
        .env("LOCT_CACHE_DIR", project.join(".loctree-cache"))
        // Fixtures live in TMPDIR, outside any git checkout; the scan guard
        // (loctree snapshot.rs) documents this env var as its test-side counterpart.
        .env("LOCT_ALLOW_NON_GIT_ROOT", "1")
        .env_remove("LOCTREE_MCP_ALLOW_UNAUTHENTICATED")
        .env_remove("LOCTREE_MCP_AUTH_TOKEN")
        .env_remove("LOCTREE_MCP_TOKEN_STORE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }

    let mut child = command.spawn().expect("spawn loctree-mcp");
    let addr = read_announced_addr(&mut child);
    TestServer { child, addr }
}

/// Read the address the server announces once it has bound its socket. Bounded
/// by a deadline so a wedged child cannot hang the whole suite (cargo test has
/// no per-test timeout).
fn read_announced_addr(child: &mut Child) -> SocketAddr {
    const DEADLINE: Duration = Duration::from_secs(15);

    let stdout = child.stdout.take().expect("child stdout piped");
    let (tx, rx) = mpsc::channel::<Result<SocketAddr, String>>();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(Err(
                        "server exited before announcing a listening address".into()
                    ));
                    return;
                }
                Ok(_) => {
                    if let Some(rest) = line.trim().strip_prefix(LISTEN_PREFIX) {
                        let _ = tx.send(
                            rest.parse::<SocketAddr>()
                                .map_err(|e| format!("parse announced address {rest:?}: {e}")),
                        );
                        return;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("read server stdout: {e}")));
                    return;
                }
            }
        }
    });

    match rx.recv_timeout(DEADLINE) {
        Ok(Ok(addr)) => addr,
        Ok(Err(msg)) => panic!("{msg}"),
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("server did not announce a listening address within {DEADLINE:?}");
        }
    }
}

/// A `0.0.0.0:port` announcement is not connectable as-is; dial loopback.
fn loopback_of(addr: SocketAddr) -> SocketAddr {
    if addr.ip().is_unspecified() {
        SocketAddr::from(([127, 0, 0, 1], addr.port()))
    } else {
        addr
    }
}

fn context_pack_path(project: &Path) -> String {
    format!(
        "/context_pack?project={}",
        percent_encode(&project.to_string_lossy())
    )
}

fn http_get(addr: SocketAddr, path: &str, bearer: Option<&str>) -> HttpResponse {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("read timeout");

    let auth_header = match bearer {
        Some(token) => format!("Authorization: Bearer {token}\r\n"),
        None => String::new(),
    };
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\n{auth_header}Connection: close\r\n\r\n"
    )
    .expect("write request");

    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .unwrap_or_else(|e| panic!("read response from {addr} path={path}: {e}; raw={raw:?}"));
    parse_response(&raw)
}

fn parse_response(raw: &str) -> HttpResponse {
    let (head, body) = raw.split_once("\r\n\r\n").expect("http separator");
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .expect("status code");
    HttpResponse {
        status,
        headers_lower: head.to_ascii_lowercase(),
        body: body.to_string(),
    }
}

fn percent_encode(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
