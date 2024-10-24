use std::time::Instant;

use rocket::serde::Deserialize;

pub struct RateLimiter {
    conf: RateLimitConfig,
    burst: isize,
    last_message_instant: Instant,
}
impl RateLimiter {
    pub fn new(conf: RateLimitConfig) -> Self {
        Self {
            conf,
            burst: 0,
            last_message_instant: Instant::now(),
        }
    }

    pub fn update(&mut self) -> bool {
        let last_mesg_sec: isize = self
            .last_message_instant
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(isize::MAX);
        self.last_message_instant = Instant::now();

        if last_mesg_sec < self.conf.min_message_time_hard {
            return false;
        }
        self.burst += self
            .conf
            .min_message_time_hard
            .saturating_sub(last_mesg_sec);
        if self.burst < 0 {
            self.burst = 0;
        }
        if self.burst > self.conf.kick_burst {
            return false;
        }
        if last_mesg_sec < self.conf.min_message_time_soft {
            self.burst += self
                .conf
                .min_message_time_soft
                .saturating_sub(last_mesg_sec)
                * 2.clamp(0, isize::MAX);
        }
        true
    }
}

pub struct SpamLimiter<T> {
    last_message: Option<T>,
    last_message_instant: Instant,
}
impl<T: std::cmp::PartialEq> SpamLimiter<T> {
    pub fn new() -> Self {
        Self {
            last_message: None,
            last_message_instant: Instant::now(),
        }
    }
    pub fn update(&mut self, message: T) -> bool {
        let allow = self.last_message_instant.elapsed().as_secs() > 5
            || Some(&message) != self.last_message.as_ref();

        self.last_message = Some(message);
        self.last_message_instant = Instant::now();
        allow
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct RateLimitConfig {
    pub min_message_time_hard: isize,
    pub min_message_time_soft: isize,
    pub kick_burst: isize,
}
