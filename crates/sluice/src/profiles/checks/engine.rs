//! The check profile's own talk with the host's container engine, beyond
//! record 0074's look: the containers a place left, found and removed
//! (34; previews-1 clause 101; record 0114; rule core.10). The place
//! profile is handed [`end_containers`] by the binding and never calls
//! this profile itself (rule core.2).
//!
//! A container is a place's by what the engine reports of it and the
//! place holds, and by nothing else (record 0114, decision 2): a mount
//! within its checkout or scratch, a port of its own published on the
//! host, or its `sluice.place` label.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Result};
use sluice_seams::Place;

use super::{Endpoint, EngineTls, ENGINE_WAIT};

/// The label a container carries to be a place's whatever it mounts or
/// publishes: its value, the place's `SLUICE_PLACE`.
pub const PLACE_LABEL: &str = "sluice.place";

/// How long a listing of the engine's containers may take to answer.
const LIST_WAIT: Duration = Duration::from_secs(10);

/// How long a removal may take to answer: the engine stops a running
/// container first.
const REMOVE_WAIT: Duration = Duration::from_secs(30);

/// The most of a listing read: 16 MiB.
const LIST_MOST: u64 = 16 << 20;

/// What an answer's head may take beyond its body, over a unix socket.
const HEAD_MOST: u64 = 64 << 10;

/// 34, previews-1 clause 101, record 0114 (decisions 2, 4, 5): remove every
/// container `place` left at each engine of `endpoints` that answers, as
/// [`owned`] tells them, each with its anonymous volumes, a running one
/// stopped. Each unix socket is asked once by the path it resolves to; one
/// that does not exist, and an endpoint that does not answer `GET /_ping`
/// with `200`, is passed over, so a host with no engine has nothing to end.
/// Each call is one try (rule core.29), bounded: the connection by
/// [`ENGINE_WAIT`], a listing by ten seconds and 16 MiB, a removal by
/// thirty. A container the engine says is gone counts as removed. Where a
/// listing or a removal fails, the rest go on, and then this fails with
/// each container left, by its first name, and the engine's words.
pub fn end_containers(endpoints: &[Endpoint], place: &Place) -> Result<()> {
    let mut asked = BTreeSet::new();
    let mut left = Vec::new();
    for e in endpoints {
        if let Endpoint::Unix(socket) = e {
            if !socket.exists() {
                continue;
            }
            // A path and the path it resolves to are one socket.
            let real = std::fs::canonicalize(socket).unwrap_or_else(|_| socket.clone());
            if !asked.insert(real) {
                continue;
            }
        }
        let answers = match e {
            Endpoint::Unix(socket) => super::ping(socket),
            Endpoint::Tcp { port, tls, .. } => super::ping_tcp(&e.host_part(), *port, tls.as_ref()),
        };
        if answers.is_err() {
            continue;
        }
        let listed = match call(e, "GET", "/containers/json?all=true", LIST_WAIT, LIST_MOST) {
            Ok((200, body)) => match serde_json::from_slice::<serde_json::Value>(&body) {
                Ok(serde_json::Value::Array(all)) => all,
                _ => {
                    left.push(format!(
                        "{e} could not list its containers: it answered no list of them"
                    ));
                    continue;
                }
            },
            Ok((status, body)) => {
                left.push(format!(
                    "{e} could not list its containers: {}",
                    words(status, &body)
                ));
                continue;
            }
            Err(why) => {
                left.push(format!("{e} could not list its containers: it {why}"));
                continue;
            }
        };
        for c in listed.iter().filter(|c| owned(c, place)) {
            let name = first_name(c);
            let Some(id) = c["Id"]
                .as_str()
                .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()))
            else {
                left.push(format!(
                    "{name}: {e} listed it with no id it can be removed by"
                ));
                continue;
            };
            let path = format!("/containers/{id}?force=true&v=true");
            match call(e, "DELETE", &path, REMOVE_WAIT, HEAD_MOST) {
                // 404: gone already, which is what was asked.
                Ok((204 | 404, _)) => tracing::info!(
                    sluice.place = %place.name,
                    container = %name,
                    "ended a container the place left"
                ),
                Ok((status, body)) => left.push(format!("{name}: {}", words(status, &body))),
                Err(why) => left.push(format!("{name}: {e} {why}")),
            }
        }
    }
    if !left.is_empty() {
        bail!(
            "could not remove the containers {} left: {}",
            place.name,
            left.join("; ")
        );
    }
    Ok(())
}

/// A listed container's first name, as `docker ps` shows it, else the
/// start of its id.
fn first_name(c: &serde_json::Value) -> String {
    match c["Names"][0].as_str() {
        Some(n) => n.trim_start_matches('/').to_string(),
        None => c["Id"].as_str().unwrap_or("").chars().take(12).collect(),
    }
}

/// What the engine said in an answer other than the one asked for: its
/// `message`, else its status and the start of its body.
fn words(status: u16, body: &[u8]) -> String {
    let message = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["message"].as_str().map(str::to_string));
    if let Some(m) = message {
        return m;
    }
    let text = String::from_utf8_lossy(body);
    let text: String = text.trim().chars().take(200).collect();
    match text.is_empty() {
        true => format!("answered {status}"),
        false => format!("answered {status}: {text}"),
    }
}

/// One call to the engine at `e`: its status and body, the body read to
/// at most `most` bytes, the answer waited for up to `wait`; else what
/// went wrong, in words.
fn call(
    e: &Endpoint,
    method: &str,
    path: &str,
    wait: Duration,
    most: u64,
) -> std::result::Result<(u16, Vec<u8>), String> {
    match e {
        Endpoint::Unix(socket) => unix_call(socket, method, path, wait, most),
        Endpoint::Tcp { port, tls, .. } => tcp_call(
            &e.host_part(),
            *port,
            tls.as_ref(),
            method,
            path,
            wait,
            most,
        ),
    }
}

/// [`call`] over a unix socket, spoken as [`super::ping`] speaks, in
/// HTTP/1.0 asking the engine to close the connection, the answer read to
/// the end it gives itself ([`read_answer`]), on a thread of its own, so a
/// socket that never takes the connection, or never answers, holds
/// nothing.
fn unix_call(
    socket: &Path,
    method: &str,
    path: &str,
    wait: Duration,
    most: u64,
) -> std::result::Result<(u16, Vec<u8>), String> {
    use std::io::Write;
    enum Step {
        Connected,
        Said(std::io::Result<Vec<u8>>),
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let at = socket.to_path_buf();
    let ask = format!("{method} {path} HTTP/1.0\r\nHost: docker\r\nConnection: close\r\n\r\n");
    let limit = most + HEAD_MOST;
    std::thread::spawn(move || {
        let said = (|| {
            let mut s = std::os::unix::net::UnixStream::connect(&at)?;
            let _ = tx.send(Step::Connected);
            s.set_read_timeout(Some(wait))?;
            s.set_write_timeout(Some(wait))?;
            s.write_all(ask.as_bytes())?;
            read_answer(&mut s, limit)
        })();
        let _ = tx.send(Step::Said(said));
    });
    let secs = |d: Duration| format!("{}s", d.as_secs());
    let said = match rx.recv_timeout(ENGINE_WAIT) {
        Ok(Step::Connected) => match rx.recv_timeout(wait) {
            Ok(Step::Said(said)) => said,
            _ => return Err(format!("gave no answer within {}", secs(wait))),
        },
        Ok(Step::Said(said)) => said,
        Err(_) => return Err(format!("took no connection within {}", secs(ENGINE_WAIT))),
    };
    let got = said.map_err(|e| super::io_words(&e, &secs(wait)))?;
    if got.len() as u64 > limit {
        return Err(format!("answered more than {} MiB", most >> 20));
    }
    answer(&got)
}

/// An answer read from `s` up to the end it gives itself, else to the
/// connection's close, and never past `limit` bytes and one more. An
/// engine may hold the connection open once it has answered, whatever the
/// ask said (one host's, 2026-11-11: an HTTP/1.1 answer to an HTTP/1.0 ask,
/// the connection held until the asker closed it), so a read that waited
/// for the close would wait out its bound on every call.
fn read_answer(s: &mut impl std::io::Read, limit: u64) -> std::io::Result<Vec<u8>> {
    let mut got = Vec::new();
    let mut buf = vec![0u8; 64 << 10];
    let mut ends = None;
    loop {
        let n = match s.read(&mut buf) {
            Ok(0) => return Ok(got),
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        got.extend_from_slice(&buf[..n]);
        if got.len() as u64 > limit {
            return Ok(got);
        }
        if ends.is_none() {
            ends = body_at(&got).map(|at| (at, ending(&got[..at])));
        }
        let whole = match &ends {
            None | Some((_, Ending::Close)) => false,
            Some((_, Ending::Now)) => true,
            Some((at, Ending::After(n))) => got.len() - at >= *n,
            Some((at, Ending::LastChunk)) => last_chunk_read(&got[*at..]),
        };
        if whole {
            return Ok(got);
        }
    }
}

/// Where an answer's body ends, as its head says.
enum Ending {
    /// It has none: a `1xx`, `204` or `304`.
    Now,
    /// After as many bytes as its `Content-Length` says.
    After(usize),
    /// At its last chunk, a chunk of no size.
    LastChunk,
    /// With the connection.
    Close,
}

/// Where `got`'s body begins, once it holds the whole head.
fn body_at(got: &[u8]) -> Option<usize> {
    got.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|at| at + 4)
}

/// A header's value in `head`, by its name in any case.
fn header(head: &str, name: &str) -> Option<String> {
    head.lines().skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim()
            .eq_ignore_ascii_case(name)
            .then(|| v.trim().to_string())
    })
}

/// Where the body of an answer whose head is `head` ends.
fn ending(head: &[u8]) -> Ending {
    let head = String::from_utf8_lossy(head);
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok());
    if matches!(status, Some(100..=199 | 204 | 304)) {
        return Ending::Now;
    }
    if header(&head, "transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
        return Ending::LastChunk;
    }
    match header(&head, "content-length").and_then(|v| v.parse().ok()) {
        Some(n) => Ending::After(n),
        None => Ending::Close,
    }
}

/// Whether a chunked `body` holds its last chunk's line.
fn last_chunk_read(mut body: &[u8]) -> bool {
    loop {
        let Some(line) = body.windows(2).position(|w| w == b"\r\n") else {
            return false;
        };
        let size = String::from_utf8_lossy(&body[..line]);
        let size = size.split(';').next().unwrap_or("").trim();
        let Ok(size) = usize::from_str_radix(size, 16) else {
            // Not a chunk: read on to the close, and [`answer`] says so.
            return false;
        };
        if size == 0 {
            return true;
        }
        match body.get(line + 2 + size + 2..) {
            Some(rest) => body = rest,
            None => return false,
        }
    }
}

/// An HTTP answer read whole: its status and its body, the body as its
/// `Content-Length` or its chunks give it.
fn answer(got: &[u8]) -> std::result::Result<(u16, Vec<u8>), String> {
    let Some(at) = body_at(got) else {
        let first = String::from_utf8_lossy(got);
        let first = first.lines().next().unwrap_or("").trim();
        return Err(match first.is_empty() {
            true => "closed the connection unanswered".into(),
            false => format!("answered {first:?}"),
        });
    };
    let head = String::from_utf8_lossy(&got[..at - 4]);
    let first = head.lines().next().unwrap_or("").trim();
    let status = first
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| format!("answered {first:?}"))?;
    let body = &got[at..];
    match ending(&got[..at]) {
        Ending::Now => Ok((status, Vec::new())),
        Ending::LastChunk => Ok((status, unchunked(body)?)),
        Ending::After(n) if body.len() < n => Err("cut its answer short".into()),
        Ending::After(n) => Ok((status, body[..n].to_vec())),
        Ending::Close => Ok((status, body.to_vec())),
    }
}

/// A chunked body joined.
fn unchunked(mut body: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let short = || "cut its answer short".to_string();
    let mut out = Vec::new();
    loop {
        let line = body
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(short)?;
        let size = String::from_utf8_lossy(&body[..line]);
        let size = size.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size, 16).map_err(|_| short())?;
        body = &body[line + 2..];
        if size == 0 {
            return Ok(out);
        }
        if body.len() < size + 2 {
            return Err(short());
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
}

/// [`call`] at a TCP address, plain or over `tls`, as
/// [`super::ping_tcp`] asks.
fn tcp_call(
    host: &str,
    port: u16,
    tls: Option<&EngineTls>,
    method: &str,
    path: &str,
    wait: Duration,
    most: u64,
) -> std::result::Result<(u16, Vec<u8>), String> {
    let mut config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(ENGINE_WAIT))
        .timeout_global(Some(wait))
        .user_agent("sluice");
    let scheme = match tls {
        Some(t) => {
            config = config.tls_config(super::engine_tls_config(t)?);
            "https"
        }
        None => "http",
    };
    let agent: ureq::Agent = config.build().into();
    let url = format!("{scheme}://{host}:{port}{path}");
    let asked = match method {
        "DELETE" => agent.delete(&url).call(),
        _ => agent.get(&url).call(),
    };
    let wait = format!("{}s", wait.as_secs());
    let mut r = asked.map_err(|e| super::tcp_words(e, &wait))?;
    let status = r.status().as_u16();
    let body = r
        .body_mut()
        .with_config()
        .limit(most)
        .read_to_vec()
        .map_err(|e| match e {
            ureq::Error::BodyExceedsLimit(_) => {
                format!("answered more than {} MiB", most >> 20)
            }
            e => super::tcp_words(e, &wait),
        })?;
    Ok((status, body))
}

/// Record 0114, decision 2: whether the engine's `container`, as `GET
/// /containers/json` lists it, is `place`'s: a mount's `Source` is the
/// place's checkout or scratch or lies beneath one, by whole components,
/// each path as given and as it resolves; one of its ports is published on
/// the host at one of the place's `SLUICE_PORT_<NAME>`; or its
/// [`PLACE_LABEL`] is the place's `SLUICE_PLACE`. No name, image or time
/// is read.
pub(crate) fn owned(container: &serde_json::Value, place: &Place) -> bool {
    let roots: Vec<PathBuf> = [&place.path, &place.tmp]
        .into_iter()
        .filter(|p| !p.as_os_str().is_empty())
        .flat_map(|p| super::given_and_resolved(p))
        .collect();
    let mounted = container["Mounts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["Source"].as_str().filter(|s| !s.is_empty()))
        .flat_map(|s| super::given_and_resolved(Path::new(s)))
        .any(|s| roots.iter().any(|r| s.starts_with(r)));
    if mounted {
        return true;
    }
    let ports: BTreeSet<u64> = place
        .env
        .iter()
        .filter(|(k, _)| k.starts_with("SLUICE_PORT_"))
        .filter_map(|(_, v)| v.parse().ok())
        .collect();
    let published = container["Ports"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["PublicPort"].as_u64())
        .any(|p| ports.contains(&p));
    if published {
        return true;
    }
    match (
        container["Labels"][PLACE_LABEL].as_str(),
        place.env.get("SLUICE_PLACE"),
    ) {
        (Some(label), Some(own)) => label == own,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A preview's place under `dir`: its checkout, its scratch, its `web`
    /// port 20000 and its `SLUICE_PLACE`.
    fn place(dir: &Path) -> Place {
        Place {
            name: "preview/0123456789ab".into(),
            path: dir.join("previews/preview+2F0123456789ab"),
            tmp: dir.join("scratch/0123456789"),
            env: BTreeMap::from([
                ("SLUICE_PLACE".into(), "sluice-0a1b2c3d4e".into()),
                ("SLUICE_PORT_WEB".into(), "20000".into()),
                (
                    "SLUICE_NAME_WEB".into(),
                    "web-0123456-main-repo-abcdef".into(),
                ),
            ]),
        }
    }

    fn mounting(source: &Path) -> serde_json::Value {
        serde_json::json!({
            "Id": "m", "Names": ["/m"], "Labels": {}, "Ports": [],
            "Mounts": [{"Type": "bind", "Source": source, "Destination": "/tls"}],
        })
    }

    /// Record 0114, decision 2: a container that mounts a file of the
    /// checkout, one that mounts the scratch, one that publishes the
    /// place's port, and one with its label are each the place's, the
    /// checkout's path matched as it resolves too.
    #[test]
    fn a_container_is_the_places_by_a_mount_a_port_or_its_label() {
        let dir = tempdir();
        let p = place(&dir);
        std::fs::create_dir_all(&p.path).unwrap();
        assert!(owned(&mounting(&p.path.join("tls/cert.pem")), &p));
        assert!(owned(&mounting(&p.path), &p));
        assert!(owned(&mounting(&p.tmp.join("socket")), &p));
        let port = serde_json::json!({
            "Id": "p", "Names": ["/p"], "Labels": null, "Mounts": [],
            "Ports": [
                {"PrivatePort": 4566, "Type": "tcp"},
                {"IP": "0.0.0.0", "PrivatePort": 80, "PublicPort": 20000, "Type": "tcp"},
            ],
        });
        assert!(owned(&port, &p));
        let label = serde_json::json!({
            "Id": "l", "Names": ["/l"], "Ports": [], "Mounts": [],
            "Labels": {"sluice.place": "sluice-0a1b2c3d4e", "other": "x"},
        });
        assert!(owned(&label, &p));

        // `/tmp` is `/private/tmp` on macOS: the engine may report either.
        let link = dir.join("link");
        std::os::unix::fs::symlink(dir.join("previews"), &link).unwrap();
        let through = link.join("preview+2F0123456789ab/tls/cert.pem");
        assert!(owned(&mounting(&through), &p));
        let linked = Place {
            path: link.join("preview+2F0123456789ab"),
            ..place(&dir)
        };
        assert!(owned(&mounting(&p.path.join("tls")), &linked));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Record 0114, decision 2: a container of a sibling directory whose
    /// name begins with the checkout's, one publishing another port or
    /// only privately, one labelled with another place, and one with
    /// nothing at all are left.
    #[test]
    fn a_container_of_another_place_is_left() {
        let dir = tempdir();
        let p = place(&dir);
        let sibling = dir.join("previews/preview+2F0123456789ab-other/tls");
        assert!(!owned(&mounting(&sibling), &p));
        assert!(!owned(&mounting(&dir.join("previews")), &p));
        assert!(!owned(&mounting(&dir.join("scratch/01234567890")), &p));
        let ports = serde_json::json!({
            "Id": "p", "Names": ["/p"], "Labels": {}, "Mounts": [],
            "Ports": [
                {"PrivatePort": 20000, "Type": "tcp"},
                {"IP": "0.0.0.0", "PrivatePort": 80, "PublicPort": 20001, "Type": "tcp"},
            ],
        });
        assert!(!owned(&ports, &p));
        let label = serde_json::json!({
            "Id": "l", "Names": ["/l"], "Ports": [], "Mounts": [],
            "Labels": {"sluice.place": "sluice-ffffffffff"},
        });
        assert!(!owned(&label, &p));
        assert!(!owned(&serde_json::json!({"Id": "n"}), &p));

        // A place with no checkout, scratch or environment owns nothing.
        let bare = Place {
            name: "bare".into(),
            path: PathBuf::new(),
            tmp: PathBuf::new(),
            env: BTreeMap::new(),
        };
        assert!(!owned(&mounting(Path::new("/")), &bare));
        assert!(!owned(&mounting(&sibling), &bare));
    }

    /// A directory of the test's own, resolved.
    fn tempdir() -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sluice-engine-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }
}
