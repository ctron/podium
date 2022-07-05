use tui::{backend::Backend, style::*, text::*, widgets::*, Frame};

pub fn draw_help<B>(rect: &mut Frame<B>)
where
    B: Backend,
{
    let mut text = Text::from("\n");
    text.extend(Text::from(Spans::from(vec![
        Span::styled(" Poddy", Style::default().add_modifier(Modifier::BOLD)),
        Span::from(" - "),
        Span::styled(
            "watch your pods",
            Style::default().add_modifier(Modifier::ITALIC),
        ),
    ])));

    text.extend(Text::from(
        r#"
 Keys:
   <Esc>   Exit the current view (or the application)
   q, <Ctrl> + c   Exit the application

   h   View this help   
   l   Toggle log view

   d   View deployments (not working yet)
   p   View pods

   ## Pods
   
   k   Kill selected pod

"#,
    ));
    let help = Paragraph::new(text).block(Block::default().title("Help").borders(Borders::ALL));
    rect.render_widget(help, rect.size());
}
