use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage, ChatMessageResponseStream},
    Ollama,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tokio_stream::StreamExt;
use warp::{reject, Filter}; // Removed unused import Reply

#[derive(Deserialize)]
struct InputMessage {
    message: String,
}

#[derive(Serialize)]
struct OutputMessage {
    response: String,
}

// Define a custom rejection type
#[derive(Debug)]
struct CustomReject {
    message: String,
}

impl reject::Reject for CustomReject {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ollama = Ollama::default();

    let chat_route = warp::post()
        .and(warp::path("chat"))
        .and(warp::body::json())
        .and_then(move |input: InputMessage| {
            let ollama = ollama.clone(); // Clone Ollama instance for each request
            async move {
                let messages: Vec<ChatMessage> = vec![ChatMessage::user(input.message)];
                let mut stream: ChatMessageResponseStream = ollama
                    .send_chat_messages_stream(ChatMessageRequest::new(
                        "llama2:latest".to_string(),
                        messages.clone(),
                    ))
                    .await
                    .map_err(|_| warp::reject::not_found())?;

                let mut response = String::new();
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(res) => {
                            if let Some(assistant_message) = res.message {
                                response += &assistant_message.content;
                                print!("{}", assistant_message.content); // Print as it's processed
                                io::stdout().flush().unwrap(); // Flush stdout to ensure immediate output
                            }
                        }
                        Err(e) => {
                            let error_message = format!("Streaming error: {:?}", e);
                            eprintln!("{}", error_message); // Log error
                            return Err(warp::reject::custom(CustomReject {
                                message: error_message.clone(),
                            }));
                        }
                    }
                }

                let output = OutputMessage { response };
                Ok::<_, warp::Rejection>(warp::reply::json(&output))
            }
        });

    // Add the custom rejection handler to the route
    let routes = chat_route
        .recover(handle_rejection) // Integrate the rejection handler
        .with(warp::cors().allow_any_origin());

    println!("Server running on http://localhost:3030/chat");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

// Implement a handler for custom rejections
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(CustomReject { message }) = err.find() {
        let json = warp::reply::json(&OutputMessage {
            response: message.clone(),
        });
        return Ok(json); // Return the JSON response directly
    }
    Err(err) // Fallback to default error response
}
