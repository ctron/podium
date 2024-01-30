use ratatui::widgets::TableState;
use std::cmp::max;

pub trait Paging {
    fn next(&mut self, total: usize, increment: usize);
    fn prev(&mut self, total: usize, increment: usize);
}

impl Paging for TableState {
    fn next(&mut self, total: usize, increment: usize) {
        if total == 0 {
            return self.select(None);
        }

        let i = match self.selected() {
            Some(i) => {
                if i == total - 1 {
                    // last one, continue with first
                    0
                } else if i + increment >= total {
                    // close to last one, go to last
                    total - 1
                } else {
                    // just add increment
                    i + increment
                }
            }
            None => max(total, increment),
        };
        self.select(Some(i));
    }

    fn prev(&mut self, total: usize, increment: usize) {
        if total == 0 {
            return self.select(None);
        }

        let i = match self.selected() {
            Some(i) => {
                if i == 0 {
                    // first cone, continue with last
                    total - 1
                } else if i < increment {
                    // close to top, go with first
                    0
                } else {
                    i - increment
                }
            }
            None => 0,
        };
        self.select(Some(i));
    }
}
