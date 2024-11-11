use governor::{
    Quota,
    RateLimiter as Governor,
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct RateLimiter {
    limiters: std::collections::HashMap<String, Arc<Governor<NotKeyed, InMemoryState, DefaultClock>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
                let mut limiters = std::collections::HashMap::new();

                // Configure rate limits for each engine
                limiters.insert(
                        "Google".to_string(),
                        Arc::new(Governor::new(
                                Quota::per_second(NonZeroU32::new(5).unwrap()),
                                InMemoryState::default(),
                                DefaultClock::default(),
                        )),
                );
                limiters.insert(
                        "DuckDuckGo".to_string(),
                        Arc::new(Governor::new(
                                Quota::per_second(NonZeroU32::new(5).unwrap()),
                                InMemoryState::default(),
                                DefaultClock::default(),
                        )),
                );

        Self { limiters }
    }

    pub async fn check_rate_limit(&self, engine: &str) -> bool {
        if let Some(limiter) = self.limiters.get(engine) {
            limiter.check().is_ok()
        } else {
            true
        }
    }
}
