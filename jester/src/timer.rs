use std::time::Duration;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimerMode {
    Once,
    #[default]
    Loop,
}

#[derive(Debug)]
pub struct Timer {
    preset: Duration,
    remaining: Duration,
    mode: TimerMode,
}

impl Timer {
    pub fn new(preset: Duration, mode: TimerMode) -> Self {
        Self {
            preset,
            remaining: preset,
            mode,
        }
    }

    pub fn tick(&mut self, dt: Duration) -> bool {
        if self.remaining == Duration::ZERO {
            return false;
        }

        self.remaining = self.remaining.saturating_sub(dt);

        if self.remaining == Duration::ZERO {
            match self.mode {
                TimerMode::Once => { /* stay at zero */ }
                TimerMode::Loop => self.remaining = self.preset,
            }
            return true;
        }
        false
    }

    pub fn finished(&self) -> bool {
        self.remaining == Duration::ZERO
    }

    pub fn reset(&mut self) {
        self.remaining = self.preset;
    }

    pub fn set(&mut self, new_preset: Duration) {
        self.preset = new_preset;
        self.reset();
    }

    pub fn remaining(&self) -> Duration {
        self.remaining
    }
}
