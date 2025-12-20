//! Test Twitcasting danmaku connection with actual WebSocket
//!
//! Twitcasting requires a movie_id to connect. You need to:
//! 1. Find a live stream on twitcasting.tv
//! 2. Get the movie_id from the page (inspect network requests or HTML)
//! 3. Pass it via PlatformContext

use danmaku::protocols::{twitcasting::Twitcasting, Platform, PlatformContext};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Twitcasting requires movie_id - you need to get this from a live stream
    // Example: from https://twitcasting.tv/username the movie_id is in the page
    let room_url = "https://twitcasting.tv/example_user";

    // You must set the movie_id manually
    let movie_id = std::env::var("TWITCASTING_MOVIE_ID")
        .unwrap_or_else(|_| "".to_string());

    if movie_id.is_empty() {
        println!("Twitcasting requires a movie_id to connect.");
        println!("\nUsage:");
        println!("  TWITCASTING_MOVIE_ID=123456789 cargo run -p danmaku --example test_twitcasting");
        println!("\nHow to get movie_id:");
        println!("  1. Go to a live Twitcasting stream");
        println!("  2. Open browser DevTools > Network tab");
        println!("  3. Look for 'movie_id' in requests or page HTML");
        return Ok(());
    }

    println!("Testing Twitcasting danmaku for: {}", room_url);
    println!("Movie ID: {}\n", movie_id);

    let platform = Twitcasting::new();
    let ctx = PlatformContext::new()
        .with_room_id(movie_id.clone());

    // Create context with movie_id
    let ctx = PlatformContext {
        movie_id: Some(movie_id),
        ..ctx
    };

    println!("1. Getting connection info...");
    let info = platform.get_connection_info(room_url, &ctx).await?;
    println!("   WebSocket URL: {}", info.ws_url);

    println!("\n2. Connecting to WebSocket...");
    let (ws_stream, _) = connect_async(&info.ws_url).await?;
    let (mut write, mut read) = ws_stream.split();
    println!("   Connected!");

    let heartbeat_config = platform.heartbeat_config();
    println!("\n3. Heartbeat config: {:?}", heartbeat_config.data.is_some());

    println!("\n4. Listening for messages (Ctrl+C to stop)...\n");

    let mut message_count = 0;

    loop {
        tokio::select! {
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
