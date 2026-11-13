//! An overlay's value with its fade: open while live, kept after `close` until the leave fade
//! ends so it can still be drawn. Only the live value counts as "open".

use std::time::Instant;

use super::Fade;

pub struct Fading<T>(Option<(T, Fade)>);

impl<T> Default for Fading<T> {
    fn default() -> Self {
        Fading(None)
    }
}

impl<T> Fading<T> {
    /// Shows `v`, fading in; drops whatever was still fading out.
    pub fn open(&mut self, v: T) {
        self.0 = Some((v, Fade::enter()));
    }

    /// Starts the leave fade from where the enter was; the closed value, if one was live.
    pub fn close(&mut self, now: Instant) -> Option<&T> {
        let (v, f) = self.0.as_mut().filter(|(_, f)| !f.leaving())?;
        *f = Fade::leave_from(f, now);
        Some(v)
    }

    pub fn live(&self) -> Option<&T> {
        self.0
            .as_ref()
            .filter(|(_, f)| !f.leaving())
            .map(|(v, _)| v)
    }

    pub fn live_mut(&mut self) -> Option<&mut T> {
        self.0
            .as_mut()
            .filter(|(_, f)| !f.leaving())
            .map(|(v, _)| v)
    }

    pub fn is_open(&self) -> bool {
        self.live().is_some()
    }

    pub fn leaving(&self) -> Option<&T> {
        self.0.as_ref().filter(|(_, f)| f.leaving()).map(|(v, _)| v)
    }

    /// What to draw: the live or leaving value with its fade.
    pub fn shown(&self) -> Option<(&T, Fade)> {
        self.0.as_ref().map(|(v, f)| (v, *f))
    }

    /// Drops a finished leaving value; true while a fade still needs frames.
    pub fn prune(&mut self, now: Instant) -> bool {
        if self
            .0
            .as_ref()
            .is_some_and(|(_, f)| f.leaving() && f.done_at(now))
        {
            self.0 = None;
        }
        self.0.as_ref().is_some_and(|(_, f)| !f.done_at(now))
    }
}

#[cfg(test)]
mod tests {
    use super::super::motion;
    use super::*;

    #[test]
    fn open_is_live_and_settles() {
        let mut f = Fading::default();
        assert!(!f.is_open() && f.shown().is_none());
        f.open(1);
        let t0 = Instant::now();
        assert_eq!(f.live(), Some(&1));
        assert!(f.leaving().is_none());
        assert!(f.prune(t0), "entering needs frames");
        assert!(!f.prune(t0 + motion::ENTER), "settled");
        assert_eq!(f.live(), Some(&1), "a settled live value is kept");
    }

    #[test]
    fn close_leaves_then_prune_drops_it() {
        let mut f = Fading::default();
        f.open(1);
        let t = Instant::now() + motion::ENTER;
        assert_eq!(f.close(t), Some(&1));
        assert!(!f.is_open());
        assert_eq!(f.leaving(), Some(&1));
        assert!(f.shown().unwrap().1.leaving());
        assert!(f.close(t).is_none(), "nothing live to close");
        assert!(f.prune(t));
        assert!(!f.prune(t + motion::LEAVE * 2));
        assert!(f.shown().is_none());
    }

    #[test]
    fn close_mid_enter_starts_from_the_current_amount() {
        let mut f = Fading::default();
        f.open(());
        let t = Instant::now() + motion::ENTER / 4;
        let entering = f.shown().unwrap().1.amount_at(t);
        f.close(t);
        let leave = f.shown().unwrap().1;
        assert!((leave.amount_at(t) - entering).abs() < 1e-3);
    }

    #[test]
    fn reopen_while_leaving_drops_the_leaving_one() {
        let mut f = Fading::default();
        f.open(1);
        f.close(Instant::now());
        f.open(2);
        assert_eq!(f.live(), Some(&2));
        assert!(f.leaving().is_none());
        assert!(!f.shown().unwrap().1.leaving());
        *f.live_mut().unwrap() = 3;
        assert_eq!(f.live(), Some(&3));
    }
}
