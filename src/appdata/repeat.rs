use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct Repeat {
    state: Option<RepeatState>,
    config: Option<RepeatConfig>,
}

impl Repeat {
    pub fn get_repeat(&mut self) -> Option<xkeysym::KeyCode> {
        let (Some(config), Some(state)) = (&self.config, &mut self.state) else {
            return None;
        };

        let now = Instant::now();

        let delay = Duration::from_millis(config.delay as u64);
        let interval = Duration::from_secs_f64(1.0 / config.rate as f64);

        if now.duration_since(state.started_at) >= delay
            && now.duration_since(state.last_repeat) >= interval
        {
            state.last_repeat = now;
            return Some(state.key);
        }

        None
    }

    pub fn reset_state(&mut self) {
        self.state = None;
    }

    pub fn set_state(&mut self, key: xkeysym::KeyCode) {
        self.state = Some(RepeatState::new(key));
    }

    pub fn set_config(&mut self, rate: u32, delay: u32) {
        self.config = Some(RepeatConfig::new(rate, delay));
    }
}

#[derive(Debug)]
pub struct RepeatState {
    key: xkeysym::KeyCode,
    started_at: Instant,
    last_repeat: Instant,
}

impl RepeatState {
    pub fn new(key: xkeysym::KeyCode) -> Self {
        Self {
            key,
            started_at: Instant::now(),
            last_repeat: Instant::now(),
        }
    }
}

#[derive(Debug)]
pub struct RepeatConfig {
    pub rate: u32,
    pub delay: u32,
}

impl RepeatConfig {
    pub fn new(rate: u32, delay: u32) -> Self {
        Self { rate, delay }
    }
}
