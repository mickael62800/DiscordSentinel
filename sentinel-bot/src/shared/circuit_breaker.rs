//! Circuit breaker minimaliste pour le client gRPC partage (Phase 7A).
//!
//! Implementation a 3 etats : Closed (normal), Open (court-circuite),
//! HalfOpen (autorise un seul appel test apres cooldown).
//!
//! On garde tres simple :
//! - `failure_threshold` echecs consecutifs -> ouvre.
//! - Apres `cooldown`, passe en HalfOpen et autorise UN appel.
//! - Succes en HalfOpen -> referme. Echec -> reouvre pour un nouveau cooldown.

use std::sync::atomic::{AtomicI64, AtomicU8, AtomicU32, Ordering};
use std::time::{Duration, Instant};

const STATE_CLOSED: u8 = 0;
const STATE_OPEN: u8 = 1;
const STATE_HALF_OPEN: u8 = 2;

pub struct CircuitBreaker {
    state: AtomicU8,
    failures: AtomicU32,
    /// Instant epoch (millis depuis demarrage du process) ou le breaker s'est
    /// ouvert. -1 = jamais. On utilise un i64 pour pouvoir representer "jamais".
    opened_at_ms: AtomicI64,
    failure_threshold: u32,
    cooldown: Duration,
    /// Reference temporelle pour eviter d'allouer un Mutex<Instant>.
    epoch: Instant,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown: Duration) -> Self {
        Self {
            state: AtomicU8::new(STATE_CLOSED),
            failures: AtomicU32::new(0),
            opened_at_ms: AtomicI64::new(-1),
            failure_threshold,
            cooldown,
            epoch: Instant::now(),
        }
    }

    /// Renvoie `true` si l'appel est autorise (Closed ou HalfOpen autorisant
    /// le ticket d'essai), `false` s'il doit etre court-circuite.
    pub fn allow(&self) -> bool {
        match self.state.load(Ordering::Acquire) {
            STATE_CLOSED => true,
            STATE_OPEN => {
                let opened = self.opened_at_ms.load(Ordering::Acquire);
                if opened < 0 {
                    return true;
                }
                let now_ms = self.epoch.elapsed().as_millis() as i64;
                if now_ms.saturating_sub(opened) >= self.cooldown.as_millis() as i64 {
                    // Tentative de transition vers HalfOpen — un seul thread y arrive.
                    if self
                        .state
                        .compare_exchange(STATE_OPEN, STATE_HALF_OPEN, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        return true;
                    }
                }
                false
            }
            STATE_HALF_OPEN => false, // un seul ticket d'essai actif a la fois
            _ => true,
        }
    }

    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Release);
        self.state.store(STATE_CLOSED, Ordering::Release);
        self.opened_at_ms.store(-1, Ordering::Release);
    }

    pub fn record_failure(&self) {
        // En HalfOpen, on retombe immediatement en Open pour un nouveau cooldown.
        if self.state.load(Ordering::Acquire) == STATE_HALF_OPEN {
            self.open_now();
            return;
        }
        let n = self.failures.fetch_add(1, Ordering::AcqRel) + 1;
        if n >= self.failure_threshold {
            self.open_now();
        }
    }

    fn open_now(&self) {
        self.state.store(STATE_OPEN, Ordering::Release);
        self.opened_at_ms
            .store(self.epoch.elapsed().as_millis() as i64, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_then_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_millis(50));
        assert!(cb.allow());
        cb.record_failure();
        cb.record_failure();
        assert!(cb.allow());
        cb.record_failure();
        assert!(!cb.allow());
    }

    #[test]
    fn half_open_after_cooldown() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(20));
        cb.record_failure();
        assert!(!cb.allow());
        std::thread::sleep(Duration::from_millis(30));
        assert!(cb.allow()); // half-open
        cb.record_success();
        assert!(cb.allow()); // closed
    }
}
