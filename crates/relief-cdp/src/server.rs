//! Lokaler HTTP-Server für Aufgabendateien (`url: server:<Pfad>`, Paket 64).
//!
//! Eine Testseite als `file://` kann kein iframe fremder Herkunft ohne Netz
//! laden. Der Host liefert deshalb das Verzeichnis der Seite über HTTP auf
//! `127.0.0.1` mit freiem Port aus und öffnet sie als
//! `http://localhost:<Port>/<Datei>`; ein iframe, das von
//! `127.0.0.1:<Port>` lädt, stammt dann von einer anderen Site. Ein Server je
//! Verzeichnis, gestartet beim ersten Aufruf, bis zum Ende des Laufs. Nur
//! Dateien unterhalb des Verzeichnisses, nur der Pfad der Anfrage zählt.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Präfix einer Aufgabenzeile `url:` für Seiten über den lokalen Server.
pub const PREFIX: &str = "server:";

#[derive(Default)]
pub struct Servers {
    ports: HashMap<PathBuf, u16>,
}

impl Servers {
    /// URL der Datei `path` über den Server ihres Verzeichnisses.
    pub async fn url(&mut self, path: &Path) -> Result<String> {
        let path = std::fs::canonicalize(path)?;
        let (Some(root), Some(name)) = (path.parent(), path.file_name()) else {
            return Err(anyhow!("{}: keine Datei", path.display()));
        };
        let port = match self.ports.get(root) {
            Some(&port) => port,
            None => {
                let port = start(root.to_path_buf()).await?;
                self.ports.insert(root.to_path_buf(), port);
                port
            }
        };
        Ok(format!(
            "http://localhost:{port}/{}",
            name.to_string_lossy()
        ))
    }
}

async fn start(root: PathBuf) -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let root = root.clone();
            tokio::spawn(async move {
                serve(stream, &root).await.ok();
            });
        }
    });
    Ok(port)
}

/// Eine Anfrage beantworten: die Datei oder 404.
async fn serve(mut stream: TcpStream, root: &Path) -> Result<()> {
    let mut request = Vec::new();
    let mut buf = [0; 4096];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        request.extend_from_slice(&buf[..n]);
    }
    let request = String::from_utf8_lossy(&request);
    let target = request.split_whitespace().nth(1).unwrap_or("/");
    let path = target
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_start_matches('/');
    let body = if path.split('/').any(|part| part == "..") {
        None
    } else {
        tokio::fs::read(root.join(path)).await.ok()
    };
    let head = match &body {
        Some(body) => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            content_type(path),
            body.len()
        ),
        None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
    };
    stream.write_all(head.as_bytes()).await?;
    if let Some(body) = body {
        stream.write_all(&body).await?;
    }
    stream.shutdown().await?;
    Ok(())
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}
