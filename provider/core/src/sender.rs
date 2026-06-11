use std::sync::{Arc, Mutex};

type SendCallback<T> = Box<dyn FnMut(T) + Send + 'static>;
type SharedSendCallback<T> = Arc<Mutex<SendCallback<T>>>;

/// Sends values from a provider to its consumer callback.
pub struct ProviderSender<T> {
    send: SharedSendCallback<T>,
}

impl<T> Clone for ProviderSender<T> {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
        }
    }
}

impl<T> ProviderSender<T> {
    /// Creates a sender backed by the provided callback.
    pub fn new(send: impl FnMut(T) + Send + 'static) -> Self {
        Self {
            send: Arc::new(Mutex::new(Box::new(send))),
        }
    }

    /// Delivers a value to the consumer callback.
    pub fn send(&self, value: T) {
        let mut send = self.send.lock().expect("provider sender lock");
        send(value);
    }
}
