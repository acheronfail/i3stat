use super::fraction;
use crate::context::BarEvent;
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

    pub fn set_len(&mut self, len: usize) {
        if len == 0 {
            panic!("a Paginator's length must be > 0");
        }

        self.len = len;
        if self.idx >= len {
            self.idx = 0;
        }
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
    #[should_panic]
    fn set_len0() {
        let mut p = Paginator::new();
        p.set_len(0);
    }

    #[test]
    fn forward_wrap() {
        let mut p = Paginator::new();

        p.set_len(1);
        assert_eq!(p.idx(), 0);
        p.incr();
        assert_eq!(p.idx(), 0);

        p.set_len(2);
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

        p.set_len(1);
        assert_eq!(p.idx(), 0);
        p.decr();
        assert_eq!(p.idx(), 0);

        p.set_len(2);
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
