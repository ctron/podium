pub mod help;
pub mod overlay;
pub mod state;

use crate::{ui::help::draw_help, App, Args};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
    Frame,
};
use tui_logger::TuiLoggerWidget;

pub trait StateRenderer {
    fn rect(&self) -> Rect;

    fn render_child<W: Widget>(&mut self, w: W, rect: Rect);
    fn render_child_stateful<W: StatefulWidget>(&mut self, w: W, state: &mut W::State, rect: Rect);

    fn render<W: Widget>(&mut self, w: W) {
        self.render_child(w, self.rect());
    }

    fn render_stateful<W: StatefulWidget>(&mut self, w: W, state: &mut W::State) {
        self.render_child_stateful(w, state, self.rect());
    }
}

impl<'c, 'f, B> StateRenderer for RenderContext<'c, 'f, B>
where
    B: Backend,
{
    #[inline]
    fn rect(&self) -> Rect {
        self.rect
    }

    fn render_child<W: Widget>(&mut self, w: W, rect: Rect) {
        self.frame.render_widget(w, rect);
    }

    fn render_child_stateful<W: StatefulWidget>(&mut self, w: W, state: &mut W::State, rect: Rect) {
        self.frame.render_stateful_widget(w, rect, state);
    }
}

struct RenderContext<'c, 'f, B>
where
    B: Backend + 'f,
{
    frame: &'c mut Frame<'f, B>,
    rect: Rect,
}

pub fn draw<B>(rect: &mut Frame<B>, app: &App)
where
    B: Backend,
{
    if app.global.help {
        draw_help(rect)
    } else {
        draw_default(rect, app)
    }
}

pub fn draw_default<B>(rect: &mut Frame<B>, app: &App)
where
    B: Backend,
{
    let size = rect.size();
    // TODO check size

    let logs = app.global().logs;
    let mut constraints = vec![Constraint::Length(3), Constraint::Min(1)];

    if logs {
        constraints.push(Constraint::Length(6));
    }

    // Vertical layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    // Title block
    let title = draw_title(&app.args);
    rect.render_widget(title, chunks[0]);

    // Main
    app.state().render(RenderContext {
        frame: rect,
        rect: chunks[1],
    });

    // Logs
    if logs {
        let logs = draw_logs();
        rect.render_widget(logs, chunks[2]);
    }
}

fn draw_title<'a>(args: &Args) -> Paragraph<'a> {
    Paragraph::new(format!(
        "Poddy ({})",
        args.namespace.as_deref().unwrap_or("<current>")
    ))
    .style(Style::default().fg(Color::White))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .border_type(BorderType::Plain),
    )
}

fn draw_logs<'a>() -> TuiLoggerWidget<'a> {
    TuiLoggerWidget::default()
        .output_timestamp(Some("%H:%M:%S%.3f".into()))
        .style_error(Style::default().fg(Color::Red))
        .style_debug(Style::default().fg(Color::Green))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_trace(Style::default().fg(Color::Gray))
        .style_info(Style::default().fg(Color::Blue))
        .block(
            Block::default()
                .title("Logs")
                .border_style(Style::default().fg(Color::White).bg(Color::Black))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
}
