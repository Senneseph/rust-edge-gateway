//! Simple in-memory rate limiter for authentication endpoints

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Rate limit entry tracking attempts and last reset time
#[derive(Debug, Clone)]
struct RateLimitEntry {
    attempts: u32,
    window_start: Instant,
}

/// In-memory rate limiter using sliding window
pub struct RateLimiter {
    /// Map of identifier (IP or username) to rate limit entry
    entries: Arc<DashMap<String, RateLimitEntry>>,
    /// Maximum attempts allowed in the window
    max_attempts: u32,
    /// Time window duration
    window_duration: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_attempts: u32, window_duration: Duration) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_attempts,
            window_duration,
        }
    }

    /// Check if the identifier is rate limited
    /// Returns Ok(()) if allowed, Err with retry_after duration if rate limited
    pub fn check(&self, identifier: &str) -> Result<(), Duration> {
        let now = Instant::now();
        
        // Get or create entry
        let mut entry = self.entries.entry(identifier.to_string()).or_insert(RateLimitEntry {
            attempts: 0,
            window_start: now,
        });

        // Check if window has expired
        if now.duration_since(entry.window_start) >= self.window_duration {
            // Reset window
            entry.attempts = 0;
            entry.window_start = now;
        }

        // Check if rate limited
        if entry.attempts >= self.max_attempts {
            let retry_after = self.window_duration
                .saturating_sub(now.duration_since(entry.window_start));
            return Err(retry_after);
        }

        // Increment attempts
        entry.attempts += 1;
        Ok(())
    }

    /// Reset rate limit for an identifier (e.g., after successful login)
    pub fn reset(&self, identifier: &str) {
        self.entries.remove(identifier);
    }

    /// Clean up expired entries (should be called periodically)
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| {
            now.duration_since(entry.window_start) < self.window_duration * 2
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // Default: 5 attempts per 15 minutes
        Self::new(5, Duration::from_secs(15 * 60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_err());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        
        limiter.reset("user1");
        
        assert!(limiter.check("user1").is_ok());
    }

    #[test]
    fn test_rate_limiter_window_expiry() {
        let limiter = RateLimiter::new(2, Duration::from_millis(100));
        
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_err());
        
        sleep(Duration::from_millis(150));
        
        assert!(limiter.check("user1").is_ok());
    }

    #[test]
    fn test_rate_limiter_different_users() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user1").is_ok());
        assert!(limiter.check("user2").is_ok());
        assert!(limiter.check("user2").is_ok());
        
        assert!(limiter.check("user1").is_err());
        assert!(limiter.check("user2").is_err());
    }
}

