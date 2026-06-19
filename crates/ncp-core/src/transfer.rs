use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs::{self, File as StdFile};
use std::io::{BufReader as StdBufReader, BufWriter as StdBufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use tempfile::{NamedTempFile, TempPath};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::utils::{derive_key, format_size, parse_timeout_duration, sha256_file_sync, xor_keystream_in_place};

const MAGIC: &str = "NCP1";

#[derive(Debug, Serialize, Deserialize)]
struct TransferHeader {
    magic: String,
    name: String,
    payload_size: u64,
    decoded_size: u64,
    sha256: String,
    compress: bool,
    encrypt: bool,
    is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferAck {
    resume_from: u64,
}

struct PreparedPayload {
    name: String,
    payload_path: PathBuf,
    payload_size: u64,
    decoded_size: u64,
    decoded_sha256: String,
    compress: bool,
    encrypt: bool,
    is_dir: bool,
    _temps: Vec<TempPath>,
}

pub async fn send(
    host: String,
    port: u16,
    path_input: String,
    compress: bool,
    encrypt: bool,
    passphrase: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let src = Path::new(&path_input);
    if !src.exists() {
        return Err(format!("Path not found: {}", path_input).into());
    }

    let prepared = prepare_payload(src, compress, encrypt, passphrase.as_deref())?;
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).await?;
    let (read_half, mut write_half) = stream.into_split();

    println!(
        "Sending {} ({}) -> {}",
        prepared.name,
        format_size(prepared.decoded_size),
        addr
    );

    let header = TransferHeader {
        magic: MAGIC.to_string(),
        name: prepared.name.clone(),
        payload_size: prepared.payload_size,
        decoded_size: prepared.decoded_size,
        sha256: prepared.decoded_sha256.clone(),
        compress: prepared.compress,
        encrypt: prepared.encrypt,
        is_dir: prepared.is_dir,
    };
    let header_line = format!("{}\n", serde_json::to_string(&header)?);
    write_half.write_all(header_line.as_bytes()).await?;

    let mut reader = BufReader::new(read_half);
    let mut ack_line = String::new();
    reader.read_line(&mut ack_line).await?;
    let ack: TransferAck = serde_json::from_str(ack_line.trim())?;

    let mut payload_file = File::open(&prepared.payload_path).await?;
    if ack.resume_from > 0 {
        payload_file.seek(std::io::SeekFrom::Start(ack.resume_from)).await?;
        println!("Resuming from {}", format_size(ack.resume_from));
    }

    let pb = ProgressBar::new(prepared.payload_size);
    pb.set_position(ack.resume_from);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, ETA {eta})",
        )?
        .progress_chars("██░"),
    );

    let mut sent = ack.resume_from;
    let mut buf = vec![0_u8; 64 * 1024];
    while sent < prepared.payload_size {
        let n = payload_file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        write_half.write_all(&buf[..n]).await?;
        sent += n as u64;
        pb.set_position(sent);
    }
    write_half.flush().await?;

    pb.finish_with_message("Transfer complete");
    println!("SHA256 (decoded): {}", prepared.decoded_sha256);
    Ok(())
}

pub async fn receive(
    port: u16,
    out_dir: Option<String>,
    passphrase: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {} ...", addr);

    let (stream, peer) = listener.accept().await?;
    println!("Connection from {}", peer);

    let output_dir = out_dir
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    fs::create_dir_all(&output_dir)?;

    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let mut header_line = String::new();
    reader.read_line(&mut header_line).await?;
    let header: TransferHeader = serde_json::from_str(header_line.trim())?;
    if header.magic != MAGIC {
        return Err("Invalid protocol header".into());
    }

    let part_path = output_dir.join(format!("{}.part", header.name));
    let mut resume_from = if part_path.exists() {
        fs::metadata(&part_path)?.len()
    } else {
        0
    };

    if resume_from > header.payload_size {
        fs::remove_file(&part_path)?;
        resume_from = 0;
    }

    let ack = TransferAck { resume_from };
    let ack_line = format!("{}\n", serde_json::to_string(&ack)?);
    write_half.write_all(ack_line.as_bytes()).await?;

    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part_path)
        .await?;

    println!(
        "Receiving {} ({})",
        header.name,
        format_size(header.decoded_size)
    );

    let pb = ProgressBar::new(header.payload_size);
    pb.set_position(resume_from);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, ETA {eta})",
        )?
        .progress_chars("██░"),
    );

    let mut received = resume_from;
    let mut buf = vec![0_u8; 64 * 1024];
    while received < header.payload_size {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).await?;
        received += n as u64;
        pb.set_position(received);
    }
    out.flush().await?;
    pb.finish_with_message("Payload received");

    if received != header.payload_size {
        return Err(format!(
            "Transfer interrupted: expected {}, got {}",
            header.payload_size, received
        )
        .into());
    }

    let final_path = decode_payload(
        &part_path,
        &output_dir,
        &header,
        passphrase.as_deref(),
    )?;

    println!("Saved -> {}", final_path.display());
    Ok(())
}

pub async fn send_text(
    host: String,
    port: u16,
    message: String,
    repeat: usize,
    interval: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let repeats = repeat.max(1);
    let wait = parse_timeout_duration(&interval)?;
    let addr = format!("{}:{}", host, port);

    for idx in 0..repeats {
        let mut stream = TcpStream::connect(&addr).await?;
        stream.write_all(message.as_bytes()).await?;
        stream.shutdown().await?;
        println!("Sent text message {}/{} to {}", idx + 1, repeats, addr);
        if idx + 1 < repeats && !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

    Ok(())
}

pub async fn listen_text(
    port: u16,
    keep_alive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!(
        "Listening for text on {}{}",
        addr,
        if keep_alive { " (keep-alive)" } else { "" }
    );

    loop {
        let (mut stream, peer) = listener.accept().await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        let msg = String::from_utf8_lossy(&buf);
        println!("\n[from {}]\n{}", peer, msg);

        if !keep_alive {
            break;
        }
    }

    Ok(())
}

fn prepare_payload(
    src: &Path,
    compress: bool,
    encrypt: bool,
    passphrase: Option<&str>,
) -> Result<PreparedPayload, Box<dyn std::error::Error>> {
    let mut temp_paths = Vec::new();
    let is_dir = src.is_dir();

    let (name, decoded_path, decoded_size) = if is_dir {
        let dir_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bundle")
            .to_string();
        let tar_tmp = NamedTempFile::new()?;
        let tar_path = tar_tmp.into_temp_path();
        create_tar(src, &tar_path)?;
        let size = fs::metadata(&tar_path)?.len();
        let tar_path_buf = tar_path.to_path_buf();
        temp_paths.push(tar_path);
        (dir_name, tar_path_buf, size)
    } else {
        let file_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file.bin")
            .to_string();
        (file_name, src.to_path_buf(), fs::metadata(src)?.len())
    };

    let decoded_sha256 = sha256_sync(&decoded_path)?;

    let mut payload_path = decoded_path.clone();
    if compress {
        let gz_tmp = NamedTempFile::new()?;
        let gz_path = gz_tmp.into_temp_path();
        gzip_file(&decoded_path, &gz_path)?;
        let gz_path_buf = gz_path.to_path_buf();
        temp_paths.push(gz_path);
        payload_path = gz_path_buf;
    }

    if encrypt {
        let key = derive_key(passphrase_or_env(passphrase)?);
        let enc_tmp = NamedTempFile::new()?;
        let enc_path = enc_tmp.into_temp_path();
        xor_file(&payload_path, &enc_path, &key)?;
        let enc_path_buf = enc_path.to_path_buf();
        temp_paths.push(enc_path);
        payload_path = enc_path_buf;
    }

    let payload_size = fs::metadata(&payload_path)?.len();

    Ok(PreparedPayload {
        name,
        payload_path,
        payload_size,
        decoded_size,
        decoded_sha256,
        compress,
        encrypt,
        is_dir,
        _temps: temp_paths,
    })
}

fn decode_payload(
    part_path: &Path,
    output_dir: &Path,
    header: &TransferHeader,
    passphrase: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut stage_path = part_path.to_path_buf();
    let mut temps: Vec<TempPath> = Vec::new();

    if header.encrypt {
        let key = derive_key(passphrase_or_env(passphrase)?);
        let dec_tmp = NamedTempFile::new()?;
        let dec_path = dec_tmp.into_temp_path();
        xor_file(&stage_path, &dec_path, &key)?;
        let dec_path_buf = dec_path.to_path_buf();
        temps.push(dec_path);
        stage_path = dec_path_buf;
    }

    if header.compress {
        let out_tmp = NamedTempFile::new()?;
        let out_path = out_tmp.into_temp_path();
        gunzip_file(&stage_path, &out_path)?;
        let out_path_buf = out_path.to_path_buf();
        temps.push(out_path);
        stage_path = out_path_buf;
    }

    let actual_sha = sha256_sync(&stage_path)?;
    if actual_sha != header.sha256 {
        return Err(format!("Checksum mismatch: expected {}, got {}", header.sha256, actual_sha).into());
    }

    if header.is_dir {
        let file = StdFile::open(&stage_path)?;
        let mut archive = Archive::new(file);
        archive.unpack(output_dir)?;
        fs::remove_file(part_path)?;
        return Ok(output_dir.join(&header.name));
    }

    let final_path = output_dir.join(&header.name);
    fs::copy(&stage_path, &final_path)?;
    fs::remove_file(part_path)?;
    Ok(final_path)
}

fn passphrase_or_env(passphrase: Option<&str>) -> Result<&str, Box<dyn std::error::Error>> {
    if let Some(p) = passphrase {
        return Ok(p);
    }
    if let Ok(p) = std::env::var("NCP_PASSPHRASE") {
        if !p.is_empty() {
            let static_str: &'static str = Box::leak(p.into_boxed_str());
            return Ok(static_str);
        }
    }
    Err("Encryption enabled but no passphrase provided (--passphrase or NCP_PASSPHRASE)".into())
}

fn create_tar(src_dir: &Path, out_tar: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let out = StdFile::create(out_tar)?;
    let mut builder = Builder::new(out);
    let name = src_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("bundle");
    builder.append_dir_all(name, src_dir)?;
    builder.finish()?;
    Ok(())
}

fn gzip_file(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = StdBufReader::new(StdFile::open(input)?);
    let writer = StdBufWriter::new(StdFile::create(output)?);
    let mut encoder = GzEncoder::new(writer, Compression::default());
    std::io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

fn gunzip_file(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let reader = StdBufReader::new(StdFile::open(input)?);
    let mut decoder = GzDecoder::new(reader);
    let mut writer = StdBufWriter::new(StdFile::create(output)?);
    std::io::copy(&mut decoder, &mut writer)?;
    writer.flush()?;
    Ok(())
}

fn xor_file(input: &Path, output: &Path, key: &[u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = StdBufReader::new(StdFile::open(input)?);
    let mut writer = StdBufWriter::new(StdFile::create(output)?);
    let mut counter = 0_u64;
    let mut buf = vec![0_u8; 64 * 1024];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let mut chunk = buf[..n].to_vec();
        xor_keystream_in_place(&mut chunk, key, &mut counter);
        writer.write_all(&chunk)?;
    }
    writer.flush()?;
    Ok(())
}

fn sha256_sync(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(sha256_file_sync(path)?)
}
