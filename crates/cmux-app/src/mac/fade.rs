//! Enter/leave timing for overlays (plan K5). The caller keeps a `Fade` next to the overlay,
//! draws it with `amount_at` as opacity and `slide` as offset, and asks for another frame
//! while `!done_at(now)`.

use std::time::Instant;

use super::motion;

#[derive(Debug, Clone, Copy)]
pub struct Fade {
    start: Instant,
    leaving: bool,
}

impl Fade {
    pub fn enter() -> Self {
        Fade {
            start: Instant::now(),
            leaving: false,
        }
    }

    pub fn leave() -> Self {
        Fade {
            start: Instant::now(),
            leaving: true,
        }
    }

    pub fn leaving(&self) -> bool {
        self.leaving
    }

    fn progress(&self, now: Instant) -> f32 {
        let total = if self.leaving {
            motion::LEAVE
        } else {
            motion::ENTER
        };
        now.saturating_duration_since(self.start).as_secs_f32() / total.as_secs_f32()
    }

    pub fn amount_at(&self, now: Instant) -> f32 {
        let e = motion::ease_out(self.progress(now));
        if self.leaving { 1.0 - e } else { e }
    }

    pub fn done_at(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }

    pub fn slide(&self, now: Instant) -> f32 {
        (1.0 - self.amount_at(now)) * motion::SLIDE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn enter_rises_to_one_and_finishes() {
        let t0 = Instant::now();
        let f = Fade {
            start: t0,
            leaving: false,
        };
        assert_eq!(f.amount_at(t0), 0.0);
        assert!(f.amount_at(t0 + motion::ENTER / 2) > 0.5);
        assert_eq!(f.amount_at(t0 + motion::ENTER), 1.0);
        assert!(!f.done_at(t0 + motion::ENTER / 2));
        assert!(f.done_at(t0 + motion::ENTER));
    }

    #[test]
    fn leave_falls_to_zero_and_slides_back() {
        let t0 = Instant::now();
        let f = Fade {
            start: t0,
            leaving: true,
        };
        assert_eq!(f.amount_at(t0), 1.0);
        assert_eq!(f.slide(t0), 0.0);
        assert_eq!(f.amount_at(t0 + motion::LEAVE), 0.0);
        assert_eq!(f.slide(t0 + motion::LEAVE), motion::SLIDE);
        assert!(f.done_at(t0 + motion::LEAVE + Duration::from_millis(1)));
    }
}
