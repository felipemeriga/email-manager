use crate::errors::ApiError;
use imap::Session;
use native_tls::{TlsConnector, TlsStream};
use std::collections::VecDeque;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// IMAP connection pool to reuse connections
pub struct ImapConnectionPool {
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    email: String,
    password: String,
    max_size: usize,
    max_idle_time: Duration,
}

struct PooledConnection {
    session: Session<TlsStream<TcpStream>>,
    #[allow(dead_code)]
    created_at: Instant,
    last_used: Instant,
}

impl ImapConnectionPool {
    pub fn new(email: String, password: String) -> Self {
        Self {
            connections: Arc::new(Mutex::new(VecDeque::new())),
            email,
            password,
            max_size: 5,                             // Keep up to 5 connections
            max_idle_time: Duration::from_secs(300), // 5 minutes idle timeout
        }
    }

    /// Get a connection from the pool or create a new one
    pub async fn get(&self) -> Result<Session<TlsStream<TcpStream>>, ApiError> {
        let mut pool = self.connections.lock().await;
        let now = Instant::now();

        // Remove expired connections
        pool.retain(|conn| now.duration_since(conn.last_used) < self.max_idle_time);

        // Try to get an existing connection
        if let Some(mut pooled) = pool.pop_front() {
            pooled.last_used = now;
            return Ok(pooled.session);
        }

        // Create new connection if pool is empty
        drop(pool); // Release lock before creating connection
        self.create_connection()
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, mut session: Session<TlsStream<TcpStream>>) {
        let mut pool = self.connections.lock().await;

        // Only keep connection if pool isn't full
        if pool.len() < self.max_size {
            pool.push_back(PooledConnection {
                session,
                created_at: Instant::now(),
                last_used: Instant::now(),
            });
        } else {
            // Let the connection drop if pool is full
            let _ = session.logout();
        }
    }

    /// Create a new IMAP connection
    fn create_connection(&self) -> Result<Session<TlsStream<TcpStream>>, ApiError> {
        let tls = TlsConnector::builder()
            .build()
            .map_err(|e| ApiError::InternalError(format!("TLS error: {}", e)))?;

        let client = imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls)
            .map_err(|e| ApiError::ConnectionError(format!("IMAP connection failed: {}", e)))?;

        let mut session = client
            .login(&self.email, &self.password)
            .map_err(|e| {
                ApiError::AuthenticationError(format!(
                    "IMAP authentication failed: {}. Make sure you're using an App Password, not your regular password.
                    Go to https://myaccount.google.com/apppasswords to create one.",
                    e.0
                ))
            })?;

        // Select INBOX
        session
            .select("INBOX")
            .map_err(|e| ApiError::InternalError(format!("Failed to select INBOX: {}", e)))?;

        Ok(session)
    }

    /// Clean up idle connections
    pub async fn cleanup(&self) {
        let mut pool = self.connections.lock().await;
        let now = Instant::now();

        // Remove and logout expired connections
        while let Some(conn) = pool.front() {
            if now.duration_since(conn.last_used) >= self.max_idle_time {
                if let Some(mut pooled) = pool.pop_front() {
                    let _ = pooled.session.logout();
                }
            } else {
                break; // Connections are ordered by last_used
            }
        }
    }
}
