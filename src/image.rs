use std::sync::mpsc::Sender;

use ratatui::layout::Rect;
use ratatui_image::{protocol::StatefulProtocol, FilterType, Resize};

/// A widget that uses a custom ThreadProtocol as state to offload resizing and encoding
/// to a background thread
pub struct ThreadImage {
    pub resize: Resize,
}

impl ThreadImage {
    pub fn new() -> ThreadImage {
        ThreadImage {
            resize: Resize::Fit(Some(FilterType::Gaussian)),
        }
    }

    pub fn resize(mut self, resize: Resize) -> ThreadImage {
        self.resize = resize;
        self
    }
}

/// The state of a ThreadImage.
///
/// Has `inner` [ResizeProtocol] that is sent off to the `tx` mspc channel to do the
/// `resize_encode()` work.
pub struct ThreadProtocol {
    pub inner: Option<Box<dyn StatefulProtocol>>,
    pub tx: Sender<(Box<dyn StatefulProtocol>, Resize, Rect)>,
}

impl ThreadProtocol {
    pub fn new(
        tx: Sender<(Box<dyn StatefulProtocol>, Resize, Rect)>,
        inner: Box<dyn StatefulProtocol>,
    ) -> ThreadProtocol {
        ThreadProtocol {
            inner: Some(inner),
            tx,
        }
    }
}
