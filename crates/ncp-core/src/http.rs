use colored::*;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, LOCATION};
use reqwest::{Client, Method, StatusCode, Url};
use serde::Serialize;
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::utils::format_size;

#[derive(Debug, Clone, Serialize)]
struct TimingBreakdown {
    dns_ms: u128,
    tcp_connect_ms: u128,
    tls_handshake_ms: Option<u128>,
    first_byte_ms: u128,
    total_ms: u128,
}

#[derive(Debug, Clone)]
struct HttpExecution {
    final_url: Url,
    chain: Vec<(StatusCode, Url)>,
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    timing: TimingBreakdown,
}

pub async fn run(
    url: String,
    method_str: String,
    show_headers: bool,
    header_list: Vec<String>,
    body: Option<String>,
    repeat: usize,
    out: Option<String>,
    follow: bool,
    max_redirects: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let method = Method::from_bytes(method_str.to_uppercase().as_bytes())?;
    let custom_headers = parse_custom_headers(&header_list)?;
    let repeats = repeat.max(1);

    let mut latencies = Vec::with_capacity(repeats);
    let mut last_result: Option<HttpExecution> = None;

    for _ in 0..repeats {
        let result = execute_once(
            &url,
            method.clone(),
            custom_headers.clone(),
            body.clone(),
            follow,
            max_redirects,
        )
        .await?;
        latencies.push(result.timing.total_ms);
        last_result = Some(result);
    }

    let result = last_result.expect("run should always execute at least once");
    print_result(
        &method,
        &url,
        show_headers,
        repeats,
        &latencies,
        &result,
    );

    if let Some(path) = out {
        std::fs::write(&path, &result.body)?;
        println!("\n  Saved response body to {}", path.yellow());
    }

    Ok(())
}

async fn execute_once(
    url: &str,
    method: Method,
    headers: HeaderMap,
    body: Option<String>,
    follow: bool,
    max_redirects: usize,
) -> Result<HttpExecution, Box<dyn std::error::Error>> {
    let mut chain = Vec::new();
    let mut current = Url::parse(url)?;

    let preflight = preflight_timings(&current)?;

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let request_started = Instant::now();
    let mut first_byte_ms;

    for redirect_count in 0..=max_redirects {
        let mut req = client.request(method.clone(), current.clone()).headers(headers.clone());
        if let Some(ref b) = body {
            req = req.body(b.clone());
            if !headers.contains_key("content-type") {
                req = req.header("Content-Type", "application/json");
            }
        }

        let ttfb_start = Instant::now();
        let response = req.send().await?;
        first_byte_ms = ttfb_start.elapsed().as_millis();
        let status = response.status();
        chain.push((status, current.clone()));

        if follow && status.is_redirection() {
            if let Some(loc) = response.headers().get(LOCATION) {
                let location = loc.to_str()?;
                current = current.join(location)?;
                if redirect_count == max_redirects {
                    return Err("Maximum redirects reached".into());
                }
                continue;
            }
        }

        let final_url = current.clone();
        let headers = response.headers().clone();
        let body = response.bytes().await?.to_vec();

        let timing = TimingBreakdown {
            dns_ms: preflight.0,
            tcp_connect_ms: preflight.1,
            tls_handshake_ms: preflight.2,
            first_byte_ms,
            total_ms: request_started.elapsed().as_millis(),
        };

        return Ok(HttpExecution {
            final_url,
            chain,
            status,
            headers,
            body,
            timing,
        });
    }

    Err("Request did not complete".into())
}

fn preflight_timings(url: &Url) -> Result<(u128, u128, Option<u128>), Box<dyn std::error::Error>> {
    let host = url.host_str().ok_or("URL missing host")?;
    let port = url.port_or_known_default().ok_or("Cannot resolve URL port")?;

    let dns_start = Instant::now();
    let addrs: Vec<SocketAddr> = format!("{host}:{port}").to_socket_addrs()?.collect();
    let dns_ms = dns_start.elapsed().as_millis();

    if addrs.is_empty() {
        return Ok((dns_ms, 0, None));
    }

    let connect_start = Instant::now();
    let _ = std::net::TcpStream::connect_timeout(&addrs[0], Duration::from_secs(3));
    let tcp_ms = connect_start.elapsed().as_millis();

    let tls_ms = if url.scheme() == "https" { Some(0) } else { None };
    Ok((dns_ms, tcp_ms, tls_ms))
}

fn parse_custom_headers(input: &[String]) -> Result<HeaderMap, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    for raw in input {
        let Some((k, v)) = raw.split_once(':') else {
            return Err(format!("Invalid header format: {raw}. Expected Key: Value").into());
        };
        let name = HeaderName::from_str(k.trim())?;
        let value = HeaderValue::from_str(v.trim())?;
        headers.insert(name, value);
    }
    Ok(headers)
}

fn print_result(
    method: &Method,
    initial_url: &str,
    show_headers: bool,
    repeats: usize,
    latencies_ms: &[u128],
    result: &HttpExecution,
) {
    println!("{} {}", method.as_str().cyan().bold(), initial_url.white());
    println!();

    let status_color = if result.status.is_success() {
        result.status.to_string().green()
    } else if result.status.is_client_error() {
        result.status.to_string().yellow()
    } else {
        result.status.to_string().red()
    };

    println!("  {:<12} {}", "Status".bright_black(), status_color);
    println!("  {:<12} {}", "Final URL".bright_black(), result.final_url.to_string().white());
    println!(
        "  {:<12} {}",
        "Size".bright_black(),
        format_size(result.body.len() as u64)
    );

    println!("  {:<12} {} ms", "DNS".bright_black(), result.timing.dns_ms);
    println!(
        "  {:<12} {} ms",
        "TCP".bright_black(),
        result.timing.tcp_connect_ms
    );
    if let Some(tls) = result.timing.tls_handshake_ms {
        println!("  {:<12} {} ms (estimated)", "TLS".bright_black(), tls);
    }
    println!(
        "  {:<12} {} ms",
        "First Byte".bright_black(),
        result.timing.first_byte_ms
    );
    println!("  {:<12} {} ms", "Total".bright_black(), result.timing.total_ms);

    if repeats > 1 {
        let sum: u128 = latencies_ms.iter().sum();
        let avg = sum as f64 / latencies_ms.len() as f64;
        println!("  {:<12} {:.2} ms ({} runs)", "Average".bright_black(), avg, repeats);
    }

    if result.chain.len() > 1 {
        println!("\n  {}", "Redirect Chain".cyan());
        for (code, url) in &result.chain {
            println!("  - {} {}", code.as_str().magenta(), url);
        }
    }

    if show_headers || !result.headers.is_empty() {
        println!("\n  {}", "Headers".cyan());
        for (k, v) in &result.headers {
            println!("  {:<22} {}", k.as_str().cyan(), v.to_str().unwrap_or("[binary]"));
        }
    }

    println!("\n  {}", "Body".cyan());
    let body_text = String::from_utf8_lossy(&result.body);
    let content_type = result
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("application/json") {
        match serde_json::from_str::<serde_json::Value>(&body_text) {
            Ok(v) => {
                if let Ok(pretty) = serde_json::to_string_pretty(&v) {
                    for line in pretty.lines() {
                        println!("  {}", line.green());
                    }
                }
            }
            Err(_) => {
                for line in body_text.lines() {
                    println!("  {}", line);
                }
            }
        }
    } else {
        for line in body_text.lines() {
            println!("  {}", line);
        }
    }
}
