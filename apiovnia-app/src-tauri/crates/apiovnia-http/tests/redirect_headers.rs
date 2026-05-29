//! End-to-end checks that the manual redirect follower strips request secrets
//! when a redirect leaves the origin, but keeps them on a same-origin hop.
//!
//! We stand up throwaway `tokio` TCP servers that speak just enough HTTP/1.1 to
//! drive reqwest through one redirect, and inspect the headers the *final*
//! server actually received.

use std::sync::{Arc, Mutex};

use apiovnia_core::ids::{CollectionId, RequestId};
use apiovnia_core::model::{AuthConfig, HttpMethod, KeyValue, Request};
use apiovnia_http::{Executor, ExecutorConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Read the request head (up to and including the blank line).
async fn read_head(stream: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

async fn write_redirect(stream: &mut TcpStream, location: &str) {
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;
}

async fn write_ok(stream: &mut TcpStream) {
    let body = "ok";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn request_to(url: String) -> Request {
    let mut req = Request::new_blank(
        RequestId::from_trusted("req_test"),
        CollectionId::from_trusted("col_test"),
        "redirect-test".into(),
        0,
        0,
    );
    req.url = url;
    req.method = HttpMethod::Get;
    req.auth = AuthConfig::Bearer {
        token: "supersecret-token".into(),
    };
    req.headers = vec![KeyValue {
        key: "X-Api-Key".into(),
        value: "leak-me".into(),
        enabled: true,
    }];
    req
}

#[tokio::test]
async fn cross_host_redirect_strips_auth_and_custom_headers() {
    // Final target: records the head it receives, replies 200.
    let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_port = target.local_addr().unwrap().port();
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let cap = captured.clone();
    tokio::spawn(async move {
        if let Ok((mut s, _)) = target.accept().await {
            let head = read_head(&mut s).await;
            *cap.lock().unwrap() = Some(head);
            write_ok(&mut s).await;
        }
    });

    // Redirector on a *different* port → cross-origin hop.
    let redirector = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let redir_port = redirector.local_addr().unwrap().port();
    let location = format!("http://127.0.0.1:{target_port}/landed");
    tokio::spawn(async move {
        if let Ok((mut s, _)) = redirector.accept().await {
            let _ = read_head(&mut s).await;
            write_redirect(&mut s, &location).await;
        }
    });

    let req = request_to(format!("http://127.0.0.1:{redir_port}/start"));
    let exec = Executor::new(&ExecutorConfig::default()).unwrap();
    let result = exec.execute(&req).await.expect("request should complete");
    assert_eq!(result.status, 200);

    let head = captured
        .lock()
        .unwrap()
        .clone()
        .expect("target server should have been reached")
        .to_ascii_lowercase();
    assert!(
        !head.contains("authorization:"),
        "Authorization leaked across host boundary:\n{head}"
    );
    assert!(
        !head.contains("x-api-key:"),
        "custom secret header leaked across host boundary:\n{head}"
    );
}

#[tokio::test]
async fn same_host_redirect_preserves_headers() {
    // One server, two sequential connections: 302 to a same-host path, then
    // record + 200. Both hops share host:port → secrets must survive.
    let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = server.local_addr().unwrap().port();
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let cap = captured.clone();
    tokio::spawn(async move {
        if let Ok((mut s, _)) = server.accept().await {
            let _ = read_head(&mut s).await;
            write_redirect(&mut s, "/landed").await;
        }
        if let Ok((mut s, _)) = server.accept().await {
            let head = read_head(&mut s).await;
            *cap.lock().unwrap() = Some(head);
            write_ok(&mut s).await;
        }
    });

    let req = request_to(format!("http://127.0.0.1:{port}/start"));
    let exec = Executor::new(&ExecutorConfig::default()).unwrap();
    let result = exec.execute(&req).await.expect("request should complete");
    assert_eq!(result.status, 200);

    let head = captured
        .lock()
        .unwrap()
        .clone()
        .expect("landing endpoint should have been reached")
        .to_ascii_lowercase();
    assert!(
        head.contains("authorization:"),
        "Authorization should survive a same-origin redirect:\n{head}"
    );
    assert!(
        head.contains("x-api-key:"),
        "custom header should survive a same-origin redirect:\n{head}"
    );
}
