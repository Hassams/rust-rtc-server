use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use warp::Filter;
use warp::ws::{Message, WebSocket};
use warp::Filter as _;

#[tokio::main]
async fn main() {
    // Map to store WebSocket connections
    let sockets: Arc<Mutex<HashMap<String, broadcast::Sender<Message>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // WebSocket filter
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let sockets = Arc::clone(&sockets);
            ws.on_upgrade(|socket| handle_socket(socket, sockets))
        });

    // Serve static files
    let static_files = warp::fs::dir("public");

    // Combine routes
    let routes = ws_route.or(static_files);

    // Start the warp server
    warp::serve(routes).run(([0, 0, 0, 0], 3001)).await;
}

async fn handle_socket(ws: WebSocket, sockets: Arc<Mutex<HashMap<String, broadcast::Sender<Message>>>>) {
    // Create a broadcast channel for this WebSocket
    let (tx, _) = broadcast::channel::<Message>(32);

    // Get the client's unique ID
    let socket_id = ws.remote_addr().unwrap().ip().to_string();

    // Add the sender to the map of WebSocket connections
    sockets.lock().unwrap().insert(socket_id.clone(), tx.clone());

    // Wrap the WebSocket in a warp filter
    let ws_filter = warp::ws().map(move |msg: Message| {
        // Broadcast the message to all other connected clients
        let _ = tx.send(msg);
    });

    // Combine WebSocket filter with WebSocket route
    let ws_route = warp::path("ws").and(ws_filter);

    // Serve the WebSocket route
    warp::serve(ws_route).run(ws).await;

    // Remove the sender from the map when the WebSocket disconnects
    sockets.lock().unwrap().remove(&socket_id);
}