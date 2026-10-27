//! A fake HTTP server on a port the system picks, answering each request
//! with what the test's handler says and keeping every request it saw.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// An answer: its status, its headers and its body.
pub type Answer = (u16, Vec<(String, String)>, String);

/// The status of an answer never sent: the connection is closed after the
/// request, as one cut is.
pub const CUT: u16 = 0;

pub struct Fake {
    url: String,
    seen: Arc<Mutex<Vec<Request>>>,
}

impl Fake {
    pub fn start(handler: impl Fn(&Request) -> Answer + Send + Sync + 'static) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let kept = seen.clone();
        let handler = Arc::new(handler);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let (kept, handler) = (kept.clone(), handler.clone());
                // A connection of its own each: a client may open one and
                // send nothing on it.
                std::thread::spawn(move || {
                    let Some(req) = read(&mut BufReader::new(&stream)) else {
                        return;
                    };
                    kept.lock().unwrap().push(req.clone());
                    let (status, headers, body) = handler(&req);
                    if status == CUT {
                        return;
                    }
                    let mut out = format!("HTTP/1.1 {status} X\r\nConnection: close\r\n");
                    for (k, v) in headers {
                        out.push_str(&format!("{k}: {v}\r\n"));
                    }
                    out.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
                    let _ = stream.write_all(out.as_bytes());
                });
            }
        });
        Fake { url, seen }
    }

    /// `http://127.0.0.1:<port>`
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn requests(&self) -> Vec<Request> {
        self.seen.lock().unwrap().clone()
    }
}

fn read(r: &mut impl BufRead) -> Option<Request> {
    let mut line = String::new();
    r.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        r.read_line(&mut h).ok()?;
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        let (k, v) = h.split_once(':')?;
        headers.push((k.trim().to_string(), v.trim().to_string()));
    }
    let len = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; len];
    r.read_exact(&mut body).ok()?;
    Some(Request {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into(),
    })
}

/// A JSON answer.
pub fn json(status: u16, body: serde_json::Value) -> Answer {
    (
        status,
        vec![("Content-Type".into(), "application/json".into())],
        body.to_string(),
    )
}
