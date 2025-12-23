//! Test Twitch danmaku connection with actual WebSocket

use danmaku::protocols::{twitch::Twitch, Platform, PlatformContext, RegistrationData};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Use a popular Twitch channel (adjust as needed)
    let room_url = "https://www.twitch.tv/shroud";
    println!("Testing Twitch danmaku for: {}\n", room_url);

    let platform = Twitch::new();
    let ctx = PlatformContext::new();

    println!("1. Getting connection info...");
    let info = platform.get_connection_info(room_url, &ctx).await?;
    println!("   WebSocket URL: {}", info.ws_url);
    println!("   Registration packets: {}", info.registration_data.len());

    println!("\n2. Connecting to WebSocket...");
    let (ws_stream, _) = connect_async(&info.ws_url).await?;
    let (mut write, mut read) = ws_stream.split();
    println!("   Connected!");

    println!("\n3. Sending registration packets...");
    for reg_data in &info.registration_data {
        match reg_data {
            RegistrationData::Text(text) => {
                write.send(Message::Text(text.clone().into())).await?;
                println!("   Sent: {}", text);
            }
            RegistrationData::Binary(data) => {
                write.send(Message::Binary(data.clone().into())).await?;
                println!("   Sent binary packet ({} bytes)", data.len());
            }
        }
    }

    let heartbeat_config = platform.heartbeat_config();
    println!("\n4. Heartbeat interval: {:?}", heartbeat_config.interval);

    println!("\n5. Listening for messages (Ctrl+C to stop)...\n");

    let mut message_count = 0;
    let mut heartbeat_interval = tokio::time::interval(heartbeat_config.interval);

    loop {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                if let Some(ref hb_data) = heartbeat_config.data {
                    match hb_data {
                        danmaku::protocols::HeartbeatData::Text(text) => {
                            if let Err(e) = write.send(Message::Text(text.clone().into())).await {
                                println!("   Heartbeat send error: {}", e);
                            } else {
                                println!("   [Heartbeat sent: {}]", text);
                            }
                        }
                        danmaku::protocols::HeartbeatData::Binary(data) => {
                            let _ = write.send(Message::Binary(data.clone().into())).await;
                        }
                    }
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match platform.decode_message(text.as_bytes()) {
                            Ok(result) => {
                                for event in result.events {
                                    message_count += 1;
                                    match event {
                                        danmaku::message::DanmakuEvent::Chat(chat) => {
                                            println!("[弹幕] {}: {}",
                                                chat.name.as_deref().unwrap_or("Anonymous"),
                                                chat.content);
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
                    Some(Ok(Message::Ping(data))) => {
                        let _ = write.send(Message::Pong(data)).await;
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
