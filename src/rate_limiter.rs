use crate::error::Result;
use governor::{Quota, RateLimiter as GovernorLimiter};
use std::num::NonZeroU32;

type DirectLimiter = GovernorLimiter<
    governor::state::direct::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
>;

pub struct RateLimiter {
    limiter: DirectLimiter,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        let quota =
            Quota::per_second(NonZeroU32::new(requests_per_second).expect("Rate must be non-zero"));
        Self {
            limiter: GovernorLimiter::direct(quota),
        }
    }

    pub async fn wait(&self) -> Result<()> {
        self.limiter.until_ready().await;
        Ok(())
    }
}
