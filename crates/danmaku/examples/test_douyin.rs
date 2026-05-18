//! Test Douyin danmaku connection with actual WebSocket
//!
//! NOTE: Douyin requires a valid signature to connect, which normally
//! requires JavaScript execution (webmssdk.js). Without proper signature,
//! connections will likely be rejected.

use danmaku::protocols::{douyin::Douyin, Platform, PlatformContext};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Use a Douyin live room (adjust as needed)
    let room_url = "https://live.douyin.com/123456789";
    println!("Testing Douyin danmaku for: {}\n", room_url);
    println!("WARNING: Douyin requires valid signature. Connection may fail.\n");

    let platform = Douyin::new();
    let ctx = PlatformContext::new();

    println!("1. Getting connection info...");
    let info = platform.get_connection_info(room_url, &ctx).await?;
    println!("   WebSocket URL: {}", info.ws_url);

    println!("\n2. Connecting to WebSocket...");
    let result = connect_async(&info.ws_url).await;

    match result {
        Ok((ws_stream, response)) => {
            println!("   Connected! Status: {}", response.status());
            let (mut write, mut read) = ws_stream.split();

            let heartbeat_config = platform.heartbeat_config();
            println!("\n3. Heartbeat interval: {:?}", heartbeat_config.interval);

            println!("\n4. Listening for messages (Ctrl+C to stop)...\n");

            let mut message_count = 0;
            let mut heartbeat_interval = tokio::time::interval(heartbeat_config.interval);

            loop {
                tokio::select! {
                    _ = heartbeat_interval.tick() => {
                        if let Some(ref hb_data) = heartbeat_config.data {
                            match hb_data {
                                danmaku::protocols::HeartbeatData::Binary(data) => {
                                    if let Err(e) = write.send(Message::Binary(data.clone().into())).await {
                                        println!("   Heartbeat send error: {}", e);
                                    } else {
                                        println!("   [Heartbeat sent]");
                                    }
                                }
                                danmaku::protocols::HeartbeatData::Text(text) => {
                                    let _ = write.send(Message::Text(text.clone().into())).await;
                                }
                            }
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Binary(data))) => {
                                match platform.decode_message(&data) {
                                    Ok(result) => {
                                        // Send ACK if needed
                                        if let Some(ack) = result.ack {
                                            let _ = write.send(Message::Binary(ack.into())).await;
                                        }

                                        for event in result.events {
                                            message_count += 1;
                                            match event {
                                                danmaku::message::DanmakuEvent::Chat(chat) => {
                                                    println!("[弹幕] {}: {}",
                                                        chat.name.as_deref().unwrap_or("Anonymous"),
                                                        chat.content);
                                                }
                                                danmaku::message::DanmakuEvent::Gift(gift) => {
                                                    println!("[礼物] {}", gift.content);
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
                            Some(Ok(Message::Close(_))) => {
                                println!("   WebSocket closed by server");
                                break;
                            }
                            Some(Err(e)) => {
                                println!("   WebSocket error: {}", e);
                                break;
                            }
                            None => {
                                println!("   WebSocket stream ended");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            println!("\nTotal messages received: {}", message_count);
        }
        Err(e) => {
            println!("   Failed to connect: {}", e);
            println!("\nThis is expected - Douyin requires valid signature.");
            println!("To fix: implement proper signature calculation using webmssdk.js");
        }
    }

    Ok(())
}
