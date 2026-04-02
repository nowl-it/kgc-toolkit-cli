//! MITM Proxy server
//! Full implementation with TLS interception

use std::sync::Arc;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, RwLock};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use rustls::{ClientConfig, ServerConfig};
use rustls::pki_types::ServerName;
use thiserror::Error;
use chrono::Utc;
use regex::Regex;

use super::cert_manager::CertificateManager;
use super::storage;
use super::types::{CapturedRequest, ProxyStatus};
use crate::core::kgc::CDN_REGEX;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Server error: {0}")]
    Server(String),
    #[error("Bind error: {0}")]
    Bind(String),
    #[error("TLS error: {0}")]
    Tls(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

static SERVER_STATUS: std::sync::OnceLock<Arc<RwLock<Option<ProxyStatus>>>> = std::sync::OnceLock::new();
static SHUTDOWN_TX: std::sync::OnceLock<Arc<RwLock<Option<watch::Sender<bool>>>>> = std::sync::OnceLock::new();

fn get_status_lock() -> &'static Arc<RwLock<Option<ProxyStatus>>> {
    SERVER_STATUS.get_or_init(|| Arc::new(RwLock::new(None)))
}

fn get_shutdown_lock() -> &'static Arc<RwLock<Option<watch::Sender<bool>>>> {
    SHUTDOWN_TX.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// Start the proxy server
pub async fn start(port: u16, _bind_addr: &str) -> Result<(), ServerError> {
    // Initialize storage
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kgc-toolkit");

    std::fs::create_dir_all(&data_dir)?;

    storage::initialize(&data_dir)
        .map_err(|e| ServerError::Server(e.to_string()))?;

    // Initialize certificate manager
    let cert_manager = Arc::new(CertificateManager::new(&data_dir));
    cert_manager.initialize().await
        .map_err(|e| ServerError::Tls(e.to_string()))?;

    // Get local IP
    let local_ip = get_local_ip();

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    *get_shutdown_lock().write().await = Some(shutdown_tx);

    // Update status
    let status = ProxyStatus {
        running: true,
        port,
        local_ip: local_ip.clone(),
        cert_download_url: format!("http://{}:{}/cert", local_ip, port + 1),
    };
    *get_status_lock().write().await = Some(status.clone());

    println!("========================================");
    println!("MITM Proxy listening on {}:{}", local_ip, port);
    println!("Configure your device proxy to: {}:{}", local_ip, port);
    println!("Certificate download: http://{}:{}/cert", local_ip, port + 1);
    println!("========================================");

    // Start HTTP server for cert download
    let cert_manager_clone = Arc::clone(&cert_manager);
    let cert_port = port + 1;
    tokio::spawn(async move {
        serve_certificate(cert_manager_clone, cert_port).await;
    });

    // Bind TCP listener for proxy
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()
        .map_err(|_| ServerError::Bind("Invalid address".into()))?;
    
    let listener = TcpListener::bind(addr).await
        .map_err(|e| ServerError::Bind(e.to_string()))?;

    // Main proxy loop
    let mut shutdown_rx_clone = shutdown_rx.clone();
    
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, client_addr)) => {
                        let cert_manager = Arc::clone(&cert_manager);
                        let mut shutdown_rx = shutdown_rx.clone();
                        
                        tokio::spawn(async move {
                            tokio::select! {
                                _ = handle_connection(stream, client_addr, cert_manager) => {}
                                _ = shutdown_rx.changed() => {}
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx_clone.changed() => {
                if *shutdown_rx_clone.borrow() {
                    println!("Received shutdown signal");
                    break;
                }
            }
        }
    }

    // Update status
    if let Some(status) = get_status_lock().write().await.as_mut() {
        status.running = false;
    }

    Ok(())
}

/// Handle a single connection
async fn handle_connection(
    mut client_stream: TcpStream,
    _client_addr: SocketAddr,
    cert_manager: Arc<CertificateManager>,
) {
    let mut reader = BufReader::new(&mut client_stream);
    let mut first_line = String::new();
    
    if reader.read_line(&mut first_line).await.is_err() {
        return;
    }

    let parts: Vec<&str> = first_line.trim().split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }

    let _method = parts[0];
    let _target = parts[1];
    let _version = parts[2];

    // Read headers
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.is_err() {
            return;
        }
        if line.trim().is_empty() {
            break;
        }
        headers.push(line);
    }

    let method = parts[0].to_string();
    let target = parts[1].to_string();

    if method == "CONNECT" {
        // HTTPS tunneling
        handle_connect(client_stream, &target, &headers, cert_manager).await;
    } else {
        // Direct HTTP request
        handle_http_request(client_stream, &method, &target, &headers).await;
    }
}

/// Handle CONNECT request (HTTPS tunneling)
async fn handle_connect(
    mut client_stream: TcpStream,
    target: &str,
    _headers: &[String],
    cert_manager: Arc<CertificateManager>,
) {
    // Parse host:port
    let (host, port) = if let Some(colon_pos) = target.rfind(':') {
        let h = &target[..colon_pos];
        let p = target[colon_pos + 1..].parse().unwrap_or(443);
        (h.to_string(), p)
    } else {
        (target.to_string(), 443)
    };

    // Send 200 Connection Established
    if client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.is_err() {
        return;
    }

    // Check if this is a KGC CDN request (we want to intercept these)
    let is_kgc = host.contains("kgc-cdn") || host.contains("awesomepiece.com");

    if is_kgc {
        // Full MITM - intercept TLS
        if let Err(e) = handle_tls_intercept(client_stream, &host, port, cert_manager).await {
            eprintln!("TLS intercept error for {}: {:?}", host, e);
        }
    } else {
        // Just tunnel without interception
        if let Err(e) = tunnel_connection(client_stream, &host, port).await {
            eprintln!("Tunnel error for {}: {:?}", host, e);
        }
    }
}

/// Tunnel connection without TLS interception
async fn tunnel_connection(
    mut client_stream: TcpStream,
    host: &str,
    port: u16,
) -> Result<(), ServerError> {
    let server_addr = format!("{}:{}", host, port);
    let mut server_stream = TcpStream::connect(&server_addr).await?;

    let (mut client_read, mut client_write) = client_stream.split();
    let (mut server_read, mut server_write) = server_stream.split();

    let client_to_server = tokio::io::copy(&mut client_read, &mut server_write);
    let server_to_client = tokio::io::copy(&mut server_read, &mut client_write);

    tokio::select! {
        _ = client_to_server => {}
        _ = server_to_client => {}
    }

    Ok(())
}

/// Handle TLS interception for KGC traffic
async fn handle_tls_intercept(
    client_stream: TcpStream,
    host: &str,
    port: u16,
    cert_manager: Arc<CertificateManager>,
) -> Result<(), ServerError> {
    // Generate certificate for this domain
    let (cert_pem, key_pem) = cert_manager.generate_domain_cert(host).await
        .map_err(|e| ServerError::Tls(e.to_string()))?;

    // Create server config with generated cert
    let cert = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .filter_map(|c| c.ok())
        .collect::<Vec<_>>();
    
    let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
        .map_err(|_| ServerError::Tls("Failed to parse key".into()))?
        .ok_or_else(|| ServerError::Tls("No key found".into()))?;

    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert, key)
        .map_err(|e| ServerError::Tls(e.to_string()))?;

    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    // Accept TLS from client
    let mut tls_client = acceptor.accept(client_stream).await
        .map_err(|e| ServerError::Tls(e.to_string()))?;

    // Connect to actual server
    let server_addr = format!("{}:{}", host, port);
    let server_stream = TcpStream::connect(&server_addr).await?;

    // Create TLS connector for server
    let root_store = rustls::RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
    );
    
    let client_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name: ServerName<'_> = host.to_string().try_into()
        .map_err(|_| ServerError::Tls("Invalid server name".into()))?;
    
    let mut tls_server = connector.connect(server_name, server_stream).await
        .map_err(|e| ServerError::Tls(e.to_string()))?;

    // Now proxy HTTP over TLS
    let mut buf = vec![0u8; 8192];
    
    loop {
        tokio::select! {
            result = tls_client.read(&mut buf) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        let request_data = &buf[..n];
                        
                        // Parse and log request
                        if let Some(request) = parse_http_request(request_data, host) {
                            // Check if matches KGC CDN pattern
                            if let Ok(cdn_regex) = Regex::new(CDN_REGEX) {
                                if cdn_regex.is_match(&request.url) {
                                    println!("[PROXY] KGC CDN request: {}", request.url);
                                }
                            }
                            
                            // Forward to server
                            if tls_server.write_all(request_data).await.is_err() {
                                break;
                            }
                            
                            // Read response
                            let mut response_buf = vec![0u8; 65536];
                            match tls_server.read(&mut response_buf).await {
                                Ok(0) => break,
                                Ok(resp_n) => {
                                    let response_data = &response_buf[..resp_n];
                                    
                                    // Parse response and store
                                    let captured = parse_and_capture(request, response_data);
                                    if let Err(e) = storage::store_request(&captured) {
                                        eprintln!("Storage error: {}", e);
                                    } else {
                                        println!("[PROXY] Captured: {} {}", captured.method, captured.url);
                                    }
                                    
                                    // Forward to client
                                    if tls_client.write_all(response_data).await.is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        } else {
                            // Forward without parsing
                            if tls_server.write_all(request_data).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    Ok(())
}

/// Parse HTTP request from bytes
fn parse_http_request(data: &[u8], host: &str) -> Option<CapturedRequest> {
    let text = String::from_utf8_lossy(data);
    let lines: Vec<&str> = text.lines().collect();
    
    if lines.is_empty() {
        return None;
    }

    let first_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
    if first_line_parts.len() < 2 {
        return None;
    }

    let method = first_line_parts[0].to_string();
    let path = first_line_parts[1];
    let url = format!("https://{}{}", host, path);

    // Find headers
    let mut headers = Vec::new();
    for line in lines.iter().skip(1) {
        if line.is_empty() {
            break;
        }
        headers.push(line.to_string());
    }

    Some(CapturedRequest {
        id: 0,
        timestamp: Utc::now().to_rfc3339(),
        method,
        url,
        request_headers: headers.join("\n"),
        request_body: None,
        response_status: 0,
        response_headers: String::new(),
        response_body: None,
    })
}

/// Parse response and create captured request
fn parse_and_capture(mut request: CapturedRequest, response_data: &[u8]) -> CapturedRequest {
    let text = String::from_utf8_lossy(response_data);
    let lines: Vec<&str> = text.lines().collect();
    
    if !lines.is_empty() {
        let status_parts: Vec<&str> = lines[0].split_whitespace().collect();
        if status_parts.len() >= 2 {
            request.response_status = status_parts[1].parse().unwrap_or(0);
        }
    }

    // Find header/body split
    let mut header_end = 0;
    for (i, &byte) in response_data.iter().enumerate() {
        if i >= 3 && 
           response_data[i-3] == b'\r' && 
           response_data[i-2] == b'\n' && 
           response_data[i-1] == b'\r' && 
           byte == b'\n' {
            header_end = i + 1;
            break;
        }
    }

    if header_end > 0 && header_end < response_data.len() {
        let headers = String::from_utf8_lossy(&response_data[..header_end]);
        request.response_headers = headers.to_string();
        request.response_body = Some(response_data[header_end..].to_vec());
    } else {
        request.response_body = Some(response_data.to_vec());
    }

    request
}

/// Handle direct HTTP request (non-CONNECT)
async fn handle_http_request(
    mut client_stream: TcpStream,
    method: &str,
    target: &str,
    headers: &[String],
) {
    // Parse URL
    let url = if target.starts_with("http://") {
        target.to_string()
    } else {
        // Find Host header
        let host = headers.iter()
            .find(|h| h.to_lowercase().starts_with("host:"))
            .map(|h| h.split(':').nth(1).unwrap_or("").trim())
            .unwrap_or("localhost");
        format!("http://{}{}", host, target)
    };

    // Parse host and port from URL
    let url_parsed = match url::Url::parse(&url) {
        Ok(u) => u,
        Err(_) => return,
    };

    let host = url_parsed.host_str().unwrap_or("localhost");
    let port = url_parsed.port().unwrap_or(80);
    let path = url_parsed.path();
    let query = url_parsed.query().map(|q| format!("?{}", q)).unwrap_or_default();

    // Connect to server
    let server_addr = format!("{}:{}", host, port);
    let mut server_stream = match TcpStream::connect(&server_addr).await {
        Ok(s) => s,
        Err(_) => return,
    };

    // Forward request
    let request = format!(
        "{} {}{} HTTP/1.1\r\n{}\r\n",
        method,
        path,
        query,
        headers.join("")
    );

    if server_stream.write_all(request.as_bytes()).await.is_err() {
        return;
    }

    // Read response
    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    
    loop {
        match server_stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
        
        // Simple check for complete response
        if response.len() > 4 {
            break;
        }
    }

    // Store captured request
    let captured = CapturedRequest {
        id: 0,
        timestamp: Utc::now().to_rfc3339(),
        method: method.to_string(),
        url: url.clone(),
        request_headers: headers.join(""),
        request_body: None,
        response_status: 200,
        response_headers: String::new(),
        response_body: Some(response.clone()),
    };

    let _ = storage::store_request(&captured);

    // Forward response to client
    let _ = client_stream.write_all(&response).await;
}

/// Stop the proxy server
pub async fn stop() -> Result<(), ServerError> {
    if let Some(tx) = get_shutdown_lock().write().await.take() {
        let _ = tx.send(true);
    }

    // Update status
    if let Some(status) = get_status_lock().write().await.as_mut() {
        status.running = false;
    }

    Ok(())
}

/// Get proxy status
pub async fn get_status() -> Result<Option<ProxyStatus>, ServerError> {
    Ok(get_status_lock().read().await.clone())
}

/// Get local IP address
fn get_local_ip() -> String {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok();

    if let Some(socket) = socket {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip().to_string();
            }
        }
    }

    "127.0.0.1".to_string()
}

/// Serve certificate download page
async fn serve_certificate(cert_manager: Arc<CertificateManager>, port: u16) {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response, StatusCode};
    use hyper_util::rt::TokioIo;
    use http_body_util::Full;
    use bytes::Bytes;

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind cert server: {}", e);
            return;
        }
    };

    println!("Certificate server on port {}", port);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(_) => continue,
        };

        let io = TokioIo::new(stream);
        let cert_manager = Arc::clone(&cert_manager);

        tokio::spawn(async move {
            let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                let cert_manager = Arc::clone(&cert_manager);
                async move {
                    if req.uri().path() == "/cert" || req.uri().path() == "/" {
                        if let Some(pem) = cert_manager.get_ca_cert_pem().await {
                            let response = Response::builder()
                                .status(StatusCode::OK)
                                .header("Content-Type", "application/x-x509-ca-cert")
                                .header("Content-Disposition", "attachment; filename=kgc-ca.crt")
                                .body(Full::new(Bytes::from(pem)))
                                .unwrap();
                            return Ok::<_, hyper::Error>(response);
                        }
                    }

                    let response = Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::new(Bytes::from("Not Found")))
                        .unwrap();
                    Ok(response)
                }
            });

            let _ = http1::Builder::new()
                .serve_connection(io, service)
                .await;
        });
    }
}

pub struct ProxyServer;

impl ProxyServer {
    pub async fn store_request(request: &CapturedRequest) -> Result<i64, storage::StorageError> {
        storage::store_request(request)
    }
}
