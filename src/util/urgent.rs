use std::time::{Duration, Instant};

#[derive(Default)]
pub struct UrgentTimer {
    /// If set, then the timer is active. Marks the start of the timer.
    started: Option<Instant>,
    /// Whether or not the urgent bg should be swapped with the urgent fg.
    swapped: bool,
}

impl UrgentTimer {
    #[inline]
    pub fn new() -> UrgentTimer {
        UrgentTimer::default()
    }

    pub fn swapped(&self) -> bool {
        self.swapped
    }

    pub fn toggle(&mut self, on: bool) {
        match on {
            true => {
                if self.started.is_none() {
                    self.reset_timer();
                    self.swapped = false;
                }
            }
            false => {
                self.started = None;
                self.swapped = false;
            }
        }
    }

    pub fn reset(&mut self) {
        if self.started.is_some() {
            self.reset_timer();
            self.swapped = !self.swapped;
        }
    }

    pub async fn wait(&self) {
        match self.started {
            Some(started) => {
                if let Some(time_left) = Duration::from_secs(1).checked_sub(started.elapsed()) {
                    tokio::time::sleep(time_left).await
                }
            }
            None => futures::future::pending::<()>().await,
        }
    }

    fn reset_timer(&mut self) {
        self.started = Some(Instant::now());
    }
}
