use std::time::{Duration, Instant};

const ALPHA: f64 = 0.125; // EWMA Smoothing Factor

pub struct NetworkOracle {
    pub smoothed_rtt: Duration, // FIXED: Made Public
    pub rtt_var: Duration,      // FIXED: Made Public
    pub loss_rate: f64,         // FIXED: Made Public
    last_sent: Instant,
}

impl NetworkOracle {
    pub fn new() -> Self {
        Self {
            smoothed_rtt: Duration::from_millis(100), // Default start
            rtt_var: Duration::from_millis(0),
            loss_rate: 0.0,
            last_sent: Instant::now(),
        }
    }

    /// Update the model with a new RTT measurement
    pub fn update_rtt(&mut self, rtt: Duration) {
        // Standard TCP RTT Estimator (Jacobson's Algorithm)
        let rtt_float = rtt.as_secs_f64();
        let srtt_float = self.smoothed_rtt.as_secs_f64();
        
        // EWMA Calculation
        let new_srtt = (1.0 - ALPHA) * srtt_float + ALPHA * rtt_float;
        self.smoothed_rtt = Duration::from_secs_f64(new_srtt);

        // Simple Heuristic: If RTT spikes > 2x average, assume congestion/loss
        if rtt > self.smoothed_rtt * 2 {
            self.loss_rate = (self.loss_rate + 0.1).min(1.0);
        } else {
            self.loss_rate = (self.loss_rate - 0.01).max(0.0);
        }
    }

    /// Calculate how long to sleep between packets (Congestion Control)
    pub fn get_pacing_interval(&self) -> Duration {
        // Base pacing on RTT. Slower RTT = Slower Pacing.
        // This prevents flooding a slow network.
        self.smoothed_rtt.div_f64(10.0) // Send at 10% of RTT interval
    }
}