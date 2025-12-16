//! Actor system utilities
//!
//! Provides the foundational traits and types for building actor-based services.
//! Each service runs as a long-lived async task that processes messages via channels.

use std::fmt;
use tokio::sync::{mpsc, oneshot};
use anyhow::Result;

/// Trait for actor messages - all commands sent to actors must implement this
pub trait ActorMessage: Send + 'static {}

/// Generic actor handle that can send commands to an actor
pub struct ActorHandle<C: ActorMessage> {
    sender: mpsc::Sender<C>,
}

// Manual Clone implementation that doesn't require C: Clone
impl<C: ActorMessage> Clone for ActorHandle<C> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<C: ActorMessage> ActorHandle<C> {
    /// Create a new actor handle from a sender
    pub fn new(sender: mpsc::Sender<C>) -> Self {
        Self { sender }
    }
    
    /// Send a command to the actor
    pub async fn send(&self, cmd: C) -> Result<()> {
        self.sender.send(cmd).await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))
    }
    
    /// Check if the actor is still alive (channel not closed)
    pub fn is_alive(&self) -> bool {
        !self.sender.is_closed()
    }
}

impl<C: ActorMessage> fmt::Debug for ActorHandle<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorHandle")
            .field("is_alive", &self.is_alive())
            .finish()
    }
}

/// Generic command wrapper for actors with reply channel
pub struct ActorCommand<T: Send, R: Send> {
    pub payload: T,
    pub reply: oneshot::Sender<R>,
}

impl<T: Send + 'static, R: Send + 'static> ActorMessage for ActorCommand<T, R> {}

/// Helper trait for creating request/reply patterns
pub trait RequestReply {
    type Request: Send;
    type Response: Send;
}

/// Spawn an actor task and return its handle
pub fn spawn_actor<C, F, Fut>(
    buffer_size: usize,
    actor_fn: F,
) -> ActorHandle<C>
where
    C: ActorMessage,
    F: FnOnce(mpsc::Receiver<C>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel(buffer_size);
    tokio::spawn(actor_fn(rx));
    ActorHandle::new(tx)
}

/// Actor shutdown signal
#[derive(Debug, Clone, Copy)]
pub enum ShutdownSignal {
    /// Graceful shutdown - finish current work
    Graceful,
    /// Immediate shutdown
    Immediate,
}

/// Result type for actor operations with timeout
pub type ActorResult<T> = Result<T, ActorError>;

/// Errors that can occur in actor operations
#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("Actor channel closed")]
    ChannelClosed,
    
    #[error("Operation timed out")]
    Timeout,
    
    #[error("Actor panicked")]
    Panicked,
    
    #[error("Service not available: {0}")]
    ServiceUnavailable(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

impl From<oneshot::error::RecvError> for ActorError {
    fn from(_: oneshot::error::RecvError) -> Self {
        ActorError::ChannelClosed
    }
}

impl<T> From<mpsc::error::SendError<T>> for ActorError {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        ActorError::ChannelClosed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug)]
    struct PingCommand {
        reply: oneshot::Sender<String>,
    }
    
    impl ActorMessage for PingCommand {}
    
    #[tokio::test]
    async fn test_actor_spawn_and_send() {
        let handle = spawn_actor(10, |mut rx: mpsc::Receiver<PingCommand>| async move {
            while let Some(cmd) = rx.recv().await {
                let _ = cmd.reply.send("pong".to_string());
            }
        });
        
        assert!(handle.is_alive());
        
        let (tx, rx) = oneshot::channel();
        handle.send(PingCommand { reply: tx }).await.unwrap();
        
        let response = rx.await.unwrap();
        assert_eq!(response, "pong");
    }
}