use ratatui::{style::*, text::*, widgets::*, Frame};

pub fn draw_help(rect: &mut Frame) {
    let mut text = Text::from("\n");
    text.extend(Text::from(Line::from(vec![
        Span::styled(" Podium", Style::default().add_modifier(Modifier::BOLD)),
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
   left, right
       Cycle through views

   d   View deployments
   p   View pods

   ## Pods
   
   k   Kill selected pod
   
   ## Deployments
   
   r     Restart selected deployment
   +, -  Scale up or down

"#,
    ));
    let help = Paragraph::new(text).block(Block::default().title("Help").borders(Borders::ALL));
    rect.render_widget(help, rect.size());
}
