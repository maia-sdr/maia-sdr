use axum::body::Body;
use bytes::{Bytes, BytesMut};
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub async fn get_zeros() -> Body {
    Body::from_stream(Zeros::new())
}

pub struct Zeros {
    zeros: Bytes,
}

impl Zeros {
    fn new() -> Zeros {
        Zeros {
            zeros: Bytes::from(BytesMut::zeroed(4096)),
        }
    }
}

impl Stream for Zeros {
    type Item = Result<Bytes, std::convert::Infallible>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(Ok(self.zeros.clone())))
    }
}
