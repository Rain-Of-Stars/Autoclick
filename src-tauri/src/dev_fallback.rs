use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use anyhow::{Context, bail};
use tauri::{App, Manager, Url};
use tracing::{info, warn};

static DEV_FALLBACK_STARTED: AtomicBool = AtomicBool::new(false);

pub fn ensure_dev_server<R: tauri::Runtime>(app: &mut App<R>) -> anyhow::Result<()> {
    if !tauri::is_dev() {
        return Ok(());
    }

    let Some(dev_url) = app.config().build.dev_url.clone() else {
        return Ok(());
    };

    if wait_for_dev_server(&dev_url, 5, Duration::from_millis(120)) {
        return Ok(());
    }

    let dist_dir = resolve_dist_dir()?;
    let listen_addr = resolve_socket_addr(&dev_url)?;

    if !DEV_FALLBACK_STARTED.swap(true, Ordering::SeqCst) {
        start_static_server(listen_addr, dist_dir.clone())?;
        info!(
            "开发服务器不可用，已启动前端兜底静态服务: {} -> {}",
            listen_addr,
            dist_dir.display()
        );
    }

    if !wait_for_dev_server(&dev_url, 20, Duration::from_millis(120)) {
        bail!("开发态前端兜底服务启动失败: {dev_url}");
    }

    if let Some(window) = app.get_webview_window("main") {
        window
            .navigate(dev_url.clone())
            .with_context(|| format!("无法重新导航主窗口到 {dev_url}"))?;
    }

    Ok(())
}

fn resolve_dist_dir() -> anyhow::Result<PathBuf> {
    let dist_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("无法解析仓库根目录")?
        .join("dist");
    if !dist_dir.join("index.html").is_file() {
        bail!("未找到前端构建产物: {}", dist_dir.display());
    }
    Ok(dist_dir)
}

fn resolve_socket_addr(dev_url: &Url) -> anyhow::Result<SocketAddr> {
    let host = dev_url
        .host_str()
        .context("devUrl 缺少主机名，无法启动兜底服务")?;
    let port = dev_url
        .port_or_known_default()
        .context("devUrl 缺少端口，无法启动兜底服务")?;
    let addr_text = format!("{host}:{port}");
    addr_text
        .to_socket_addrs()
        .with_context(|| format!("无法解析 devUrl 地址: {addr_text}"))?
        .find(|addr| addr.is_ipv4())
        .or_else(|| {
            addr_text
                .to_socket_addrs()
                .ok()
                .and_then(|mut items| items.next())
        })
        .with_context(|| format!("无法解析可用的监听地址: {addr_text}"))
}

fn wait_for_dev_server(dev_url: &Url, retries: u32, interval: Duration) -> bool {
    let Ok(addr) = resolve_socket_addr(dev_url) else {
        return false;
    };

    for _ in 0..retries {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(250)).is_ok() {
            return true;
        }
        thread::sleep(interval);
    }

    false
}

fn start_static_server(listen_addr: SocketAddr, dist_dir: PathBuf) -> anyhow::Result<()> {
    let listener = TcpListener::bind(listen_addr)
        .with_context(|| format!("无法绑定前端兜底服务端口: {listen_addr}"))?;

    thread::Builder::new()
        .name("autoclick-dev-fallback".to_string())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        if let Err(err) = handle_connection(stream, &dist_dir) {
                            warn!("处理前端兜底请求失败: {err}");
                        }
                    }
                    Err(err) => warn!("前端兜底服务接受连接失败: {err}"),
                }
            }
        })
        .context("无法启动前端兜底服务线程")?;

    Ok(())
}

fn handle_connection(mut stream: TcpStream, dist_dir: &Path) -> anyhow::Result<()> {
    let mut request_line = String::new();
    {
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .context("无法复制前端兜底连接用于读取请求")?,
        );
        reader
            .read_line(&mut request_line)
            .context("无法读取前端兜底请求头")?;
        loop {
            let mut line = String::new();
            let bytes = reader
                .read_line(&mut line)
                .context("无法读取前端兜底请求头内容")?;
            if bytes == 0 || line == "\r\n" {
                break;
            }
        }
    }

    let request_line = request_line.trim();
    if request_line.is_empty() {
        return Ok(());
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let request_path = parts.next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        write_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            b"method not allowed",
            method == "HEAD",
        )?;
        return Ok(());
    }

    let Some(asset_path) = resolve_asset_path(dist_dir, request_path) else {
        write_response(
            &mut stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            b"bad request",
            method == "HEAD",
        )?;
        return Ok(());
    };

    let response_path = if asset_path.is_file() {
        asset_path
    } else if !has_extension(request_path) {
        dist_dir.join("index.html")
    } else {
        asset_path
    };

    if !response_path.is_file() {
        write_response(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found",
            method == "HEAD",
        )?;
        return Ok(());
    }

    let body = fs::read(&response_path)
        .with_context(|| format!("无法读取前端兜底资源: {}", response_path.display()))?;
    let content_type = guess_content_type(&response_path);
    write_response(&mut stream, "200 OK", content_type, &body, method == "HEAD")
}

fn resolve_asset_path(dist_dir: &Path, request_path: &str) -> Option<PathBuf> {
    let clean_path = request_path.split('?').next().unwrap_or("/");
    if clean_path == "/" {
        return Some(dist_dir.join("index.html"));
    }

    let mut relative = PathBuf::new();
    for component in Path::new(clean_path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }

    Some(dist_dir.join(relative))
}

fn has_extension(request_path: &str) -> bool {
    Path::new(request_path.split('?').next().unwrap_or_default())
        .extension()
        .is_some()
}

fn guess_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> anyhow::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .context("无法写入前端兜底响应头")?;
    if !head_only {
        stream.write_all(body).context("无法写入前端兜底响应体")?;
    }
    stream.flush().context("无法刷新前端兜底响应")?;
    Ok(())
}
