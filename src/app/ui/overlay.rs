use crate::ui::StateRenderer;
use tui::{layout::*, widgets::*};

#[derive(Debug)]
pub enum Overlay {
    Message(String),
}

impl Overlay {
    pub fn render<C>(&self, mut ctx: C)
    where
        C: StateRenderer,
    {
        match self {
            Self::Message(msg) => {
                let size = ctx.rect();
                let block = Block::default().title("Popup").borders(Borders::ALL);
                let area = Self::centered_rect(size);
                ctx.render_child(Clear, area); //this clears out the background
                ctx.render_child(block, area);
            }
        }
    }

    fn centered_rect(size: Rect) -> Rect {
        size
    }
}

#[derive(Default, Debug)]
pub struct Overlays {
    overlays: Vec<Overlay>,
}

impl Overlays {
    // Render the top most overlay, if any.
    pub fn render<C>(&self, ctx: C)
    where
        C: StateRenderer,
    {
        if let Some(overlay) = self.overlays.last() {
            overlay.render(ctx);
        }
    }

    pub fn push(&mut self, overlay: Overlay) -> OverlayHandle {
        let idx = self.overlays.len();
        self.overlays.push(overlay);
        OverlayHandle { idx }
    }
}

pub struct OverlayHandle {
    idx: usize,
}

impl Drop for OverlayHandle {
    fn drop(&mut self) {
        todo!()
    }
}
