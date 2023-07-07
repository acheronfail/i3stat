use super::fraction;
use crate::context::BarEvent;
use crate::error::Result;
use crate::i3::I3Button::*;
use crate::theme::Theme;

pub struct Paginator {
    idx: usize,
    len: usize,
}

impl Paginator {
    pub fn new() -> Paginator {
        Paginator { idx: 0, len: 1 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Set the length of the paginator.
    /// Returns an error if `len == 0`, which is invalid.
    pub fn set_len(&mut self, len: usize) -> Result<()> {
        if len == 0 {
            bail!("a Paginator's length must be > 0");
        }

        self.len = len;
        if self.idx >= len {
            self.idx = 0;
        }

        Ok(())
    }

    fn incr(&mut self) {
        self.idx += 1;
        if self.idx >= self.len {
            self.idx = 0;
        }
    }

    fn decr(&mut self) {
        self.idx = (self.idx.wrapping_sub(1)).clamp(0, self.len.saturating_sub(1));
    }

    pub fn update(&mut self, event: &BarEvent) {
        match event {
            BarEvent::Click(c) if matches!(c.button, Left | ScrollUp) => self.incr(),
            BarEvent::Click(c) if matches!(c.button, Right | ScrollDown) => self.decr(),
            _ => {}
        }
    }

    pub fn format(&self, theme: &Theme) -> String {
        fraction(theme, self.idx + 1, self.len)
    }
}

#[cfg(test)]
mod paginator_tests {
    use super::*;

    #[test]
    #[should_panic(expected = "a Paginator's length must be > 0")]
    fn set_len0() {
        let mut p = Paginator::new();
        p.set_len(0).unwrap();
    }

    #[test]
    fn forward_wrap() {
        let mut p = Paginator::new();

        p.set_len(1).unwrap();
        assert_eq!(p.idx(), 0);
        p.incr();
        assert_eq!(p.idx(), 0);

        p.set_len(2).unwrap();
        p.incr();
        assert_eq!(p.idx(), 1);
        p.incr();
        assert_eq!(p.idx(), 0);
        p.incr();
        assert_eq!(p.idx(), 1);
        p.incr();
        assert_eq!(p.idx(), 0);
    }

    #[test]
    fn backward_wrap() {
        let mut p = Paginator::new();

        p.set_len(1).unwrap();
        assert_eq!(p.idx(), 0);
        p.decr();
        assert_eq!(p.idx(), 0);

        p.set_len(2).unwrap();
        p.decr();
        assert_eq!(p.idx(), 1);
        p.decr();
        assert_eq!(p.idx(), 0);
        p.decr();
        assert_eq!(p.idx(), 1);
        p.decr();
        assert_eq!(p.idx(), 0);
    }
}
