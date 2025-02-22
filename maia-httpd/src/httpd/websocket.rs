use anyhow::Result;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use bytes::Bytes;
use futures::stream::StreamExt;
use tokio::sync::broadcast;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tracing::Instrument;

pub async fn handler(
    State(sender): State<broadcast::Sender<Bytes>>,
    ws: WebSocketUpgrade,
) -> Response {
    let span = tracing::debug_span!("websocket");
    let receiver = sender.subscribe();
    ws.on_upgrade(move |socket| handle(socket, receiver).instrument(span))
}

async fn handle(socket: WebSocket, receiver: broadcast::Receiver<Bytes>) {
    if let Err(error) = handle_socket(socket, receiver).await {
        tracing::error!(%error, "client error");
    }
}

async fn handle_socket(socket: WebSocket, receiver: broadcast::Receiver<Bytes>) -> Result<()> {
    tracing::info!("websocket handshake");
    let (ws_send, ws_recv) = socket.split();
    // Future to forward messages from the receiver to the websocket.
    let send = BroadcastStream::new(receiver)
        .filter_map(|x| async move {
            match x {
                Ok(bytes) => Some(Ok(Message::Binary(bytes))),
                Err(BroadcastStreamRecvError::Lagged(lagged)) => {
                    tracing::info!("client lagged {} items", lagged);
                    None
                }
            }
        })
        .forward(ws_send);
    // Future to receive messages form the websocket and ignore them. This
    // is needed to make the lower layers reply to ping messages automatically.
    let mut receive = ws_recv.skip_while(|r| futures::future::ready(r.is_ok()));
    tokio::select! {
        ret = send => ret?,
        ret = receive.next() => match ret {
            None => anyhow::bail!("no more websocket messages to receive"),
            Some(Ok(_)) => unreachable!(), // we've skipped all the Ok messages
            Some(Err(e)) => Err(e)?,
        },
    };
    Ok(())
}
