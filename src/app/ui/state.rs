use tui::widgets::TableState;

pub trait Paging {
    fn next(&mut self, total: usize);
    fn prev(&mut self, total: usize);
}

impl Paging for TableState {
    fn next(&mut self, total: usize) {
        let i = match self.selected() {
            Some(i) => {
                if i >= total - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.select(Some(i));
    }

    fn prev(&mut self, total: usize) {
        let i = match self.selected() {
            Some(i) => {
                if i == 0 {
                    total - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.select(Some(i));
    }
}
