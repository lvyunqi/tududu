//! 进程内的固定窗口限流。
//!
//! 只挡「同一个键在短时间里来得太密」这一种情况，比如密码爆破、刷上传、刷任务。
//! 状态在内存里，重启即清零，多实例各自计数——够用，但不适合当作精确配额。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 键一多就顺手清一次过期桶，避免长期运行的进程无限吃内存。
const PURGE_THRESHOLD: usize = 10_000;

struct Bucket {
    start: Instant,
    count: u32,
}

pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// 记一次访问。超限时返回还要等多久。
    pub fn check(&self, key: &str, limit: u32, window: Duration) -> Result<(), Duration> {
        let now = Instant::now();
        let mut buckets = match self.buckets.lock() {
            Ok(guard) => guard,
            // 锁中毒说明前面有 panic，这里不放大问题，直接放行
            Err(_) => return Ok(()),
        };

        if buckets.len() > PURGE_THRESHOLD {
            buckets.retain(|_, bucket| now.duration_since(bucket.start) < window);
        }

        let entry = buckets.entry(key.to_owned()).or_insert(Bucket {
            start: now,
            count: 0,
        });

        let elapsed = now.duration_since(entry.start);
        if elapsed >= window {
            entry.start = now;
            entry.count = 0;
        }

        if entry.count >= limit {
            return Err(window.saturating_sub(elapsed));
        }

        entry.count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit() {
        let limiter = RateLimiter::new();
        let window = Duration::from_secs(60);

        for _ in 0..3 {
            assert!(limiter.check("k", 3, window).is_ok());
        }

        assert!(limiter.check("k", 3, window).is_err());
    }

    #[test]
    fn keys_are_independent() {
        let limiter = RateLimiter::new();
        let window = Duration::from_secs(60);

        for _ in 0..3 {
            assert!(limiter.check("a", 3, window).is_ok());
        }

        assert!(limiter.check("a", 3, window).is_err());
        assert!(limiter.check("b", 3, window).is_ok());
    }

    #[test]
    fn window_resets_after_expiry() {
        let limiter = RateLimiter::new();
        let window = Duration::from_millis(10);

        assert!(limiter.check("k", 1, window).is_ok());
        assert!(limiter.check("k", 1, window).is_err());

        std::thread::sleep(Duration::from_millis(20));
        assert!(limiter.check("k", 1, window).is_ok());
    }
}
