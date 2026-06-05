//! Test Douyu danmaku connection with actual TCP endpoint.

use std::path::PathBuf;
use std::time::Duration;

use danmaku::output::xml::{XmlWriter, XmlWriterConfig};
use danmaku::protocols::{
    HeartbeatData, Platform, PlatformContext, RegistrationData, douyu::Douyu,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let room_url = std::env::var("DOUYU_TEST_URL")
        .unwrap_or_else(|_| "https://www.douyu.com/9999".to_string());
    let seconds = std::env::var("DOUYU_TEST_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(120);
    let output_path = std::env::var("DOUYU_TEST_XML")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/tmp/douyu_test.xml"));
    println!("Testing Douyu danmaku for: {}\n", room_url);

    let platform = Douyu::new();
    let ctx = PlatformContext::new();

    println!("1. Getting connection info...");
    let info = platform.get_connection_info(&room_url, &ctx).await?;
    println!("   TCP URL: {}", info.ws_url);
    if !info.fallback_ws_urls.is_empty() {
        println!("   Fallback URLs: {:?}", info.fallback_ws_urls);
    }
    println!("   Registration packets: {}", info.registration_data.len());

    println!("\n2. Connecting to TCP endpoint...");
    let (stream, connected_url) = connect_with_fallback(&info).await?;
    let (mut reader, mut writer) = stream.into_split();
    println!("   Connected via {}!", connected_url);

    println!("\n3. Sending registration packets...");
    for reg_data in &info.registration_data {
        match reg_data {
            RegistrationData::Binary(data) => {
                writer.write_all(data).await?;
                println!("   Sent binary packet ({} bytes)", data.len());
            }
            RegistrationData::Text(text) => {
                writer.write_all(text.as_bytes()).await?;
                println!("   Sent text packet");
            }
        }
    }

    let heartbeat_config = platform.heartbeat_config();
    println!("\n4. Heartbeat interval: {:?}", heartbeat_config.interval);

    let mut xml_writer = XmlWriter::new(&output_path, XmlWriterConfig::default())?;
    println!("\n5. Listening for messages ({} seconds)...", seconds);
    println!("   XML output: {}\n", output_path.display());

    let mut decoded_event_count = 0;
    let mut heartbeat_interval = tokio::time::interval(heartbeat_config.interval);
    let timeout = tokio::time::sleep(Duration::from_secs(seconds));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                println!("\n--- Timeout reached ---");
                break;
            }
            _ = heartbeat_interval.tick() => {
                if let Some(ref hb_data) = heartbeat_config.data {
                    match hb_data {
                        HeartbeatData::Binary(data) => {
                            if let Err(e) = writer.write_all(data).await {
                                println!("   Heartbeat send error: {}", e);
                            } else {
                                println!("   [Heartbeat sent]");
                            }
                        }
                        HeartbeatData::Text(text) => {
                            let _ = writer.write_all(text.as_bytes()).await;
                        }
                    }
                }
            }
            frame = read_frame(&mut reader) => {
                let frame = match frame {
                    Ok(frame) => frame,
                    Err(e) => {
                        println!("   TCP read error: {}", e);
                        break;
                    }
                };

                match platform.decode_message(&frame) {
                    Ok(result) => {
                        for event in result.events {
                            decoded_event_count += 1;
                            xml_writer.write_event(&event)?;
                            match event {
                                danmaku::message::DanmakuEvent::Chat(chat) => {
                                    println!("[弹幕] {}: {}",
                                        chat.name.as_deref().unwrap_or("Anonymous"),
                                        chat.content);
                                }
                                danmaku::message::DanmakuEvent::Gift(gift) => {
                                    println!("[礼物] {}", gift.content);
                                }
                                danmaku::message::DanmakuEvent::Enter(enter) => {
                                    println!("[进入] {}", enter.name);
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        println!("   Decode error: {:?}", e);
                    }
                }
            }
        }
    }

    let xml_message_count = xml_writer.message_count();
    let final_path = xml_writer.finish()?;
    println!("\nDecoded events received: {}", decoded_event_count);
    println!("XML messages written: {}", xml_message_count);
    println!("XML file: {}", final_path.display());

    if xml_message_count == 0 {
        return Err("no XML danmaku messages were written".into());
    }

    Ok(())
}

async fn connect_with_fallback(
    info: &danmaku::protocols::ConnectionInfo,
) -> Result<(TcpStream, String), Box<dyn std::error::Error>> {
    let urls = std::iter::once(&info.ws_url).chain(info.fallback_ws_urls.iter());
    let mut last_error = None;

    for tcp_url in urls {
        let addr = tcp_url
            .strip_prefix("tcp://")
            .ok_or_else(|| format!("invalid TCP endpoint: {tcp_url}"))?;
        match TcpStream::connect(addr).await {
            Ok(stream) => return Ok((stream, tcp_url.clone())),
            Err(err) => {
                println!("   Connect to {} failed: {}", tcp_url, err);
                last_error = Some(err);
            }
        }
    }

    Err(Box::new(last_error.unwrap_or_else(|| {
        std::io::Error::other("no TCP endpoints configured")
    })))
}

async fn read_frame(reader: &mut tokio::net::tcp::OwnedReadHalf) -> std::io::Result<Vec<u8>> {
    let mut header = [0u8; 12];
    reader.read_exact(&mut header).await?;

    let length = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    if length < 8 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid frame length: {length}"),
        ));
    }

    let mut frame = Vec::with_capacity(4 + length);
    frame.extend_from_slice(&header);
    frame.resize(4 + length, 0);
    reader.read_exact(&mut frame[12..]).await?;
    Ok(frame)
}
