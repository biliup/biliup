//! Test Bilibili danmaku connection with actual WebSocket

use danmaku::protocols::{Platform, PlatformContext, RegistrationData, bilibili::Bilibili};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let cookie = "";

    let uid: u64 = 0;

    let room_url = "https://live.bilibili.com/6154037";
    println!("Testing Bilibili danmaku for: {}\n", room_url);

    // Get connection info via Platform trait
    let platform = Bilibili::new();
    // Pass cookie and uid through PlatformContext so HTTP requests include Cookie header
    // and the auth_packet contains the correct uid
    let ctx = PlatformContext::new().with_cookie(cookie).with_uid(uid);

    println!("1. Getting connection info...");
    let info = platform.get_connection_info(room_url, &ctx).await?;
    println!("   WebSocket URL: {}", info.ws_url);
    println!("   Registration packets: {}", info.registration_data.len());

    // Connect to WebSocket
    println!("\n2. Connecting to WebSocket...");
    let (ws_stream, _) = connect_async(&info.ws_url).await?;
    let (mut write, mut read) = ws_stream.split();
    println!("   Connected!");

    // Send registration (auth) packet
    println!("\n3. Sending authentication packet...");
    for reg_data in &info.registration_data {
        match reg_data {
            RegistrationData::Binary(data) => {
                write.send(Message::Binary(data.clone().into())).await?;
                println!("   Sent binary auth packet ({} bytes)", data.len());
            }
            RegistrationData::Text(text) => {
                write.send(Message::Text(text.clone().into())).await?;
                println!("   Sent text auth packet");
            }
        }
    }

    // Set up heartbeat
    let heartbeat_config = platform.heartbeat_config();
    println!("\n4. Heartbeat interval: {:?}", heartbeat_config.interval);

    // Listen for messages
    println!("\n5. Listening for messages (30 seconds)...\n");

    // let timeout = tokio::time::sleep(std::time::Duration::from_secs(30));
    // tokio::pin!(timeout);

    let mut message_count = 0;
    let mut heartbeat_interval = tokio::time::interval(heartbeat_config.interval);

    loop {
        tokio::select! {
            // _ = &mut timeout => {
            //     println!("\n--- Timeout reached ---");
            //     break;
            // }
            _ = heartbeat_interval.tick() => {
                // Send heartbeat
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
                        // Decode the message
                        match platform.decode_message(&data) {
                            Ok(result) => {
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
                                        danmaku::message::DanmakuEvent::SuperChat(sc) => {
                                            println!("[SC] {}: {} ({})", sc.name, sc.content, sc.price);
                                        }
                                        danmaku::message::DanmakuEvent::GuardBuy(guard) => {
                                            println!("[舰长] {}: {}", guard.name, guard.gift_name);
                                        }
                                        danmaku::message::DanmakuEvent::Enter(enter) => {
                                            println!("[进入] {}", enter.name);
                                        }
                                        danmaku::message::DanmakuEvent::Other { raw_data } => {
                                            // Skip other messages
                                            let _ = raw_data;
                                        }
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
    Ok(())
}
