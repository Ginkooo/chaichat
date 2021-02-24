use tui::style::Color;
use tui::Terminal;
use std::io;
use tui::backend::CrosstermBackend;
use tui::widgets::canvas::{Canvas, Rectangle};
use tui::symbols::Marker;
use types::Buffer;

type TerminalWithBackend = Terminal<CrosstermBackend<io::Stdout>>;

pub fn draw_buffer_on_screen(terminal: &mut TerminalWithBackend, buffer: &mut Buffer) {
    terminal.draw(|f| {
        let size = f.size();
        let width = size.width as f64;
        let height = size.height as f64;
        let canvas = Canvas::default()
            .marker(Marker::Block)
            .x_bounds([0.0, width])
            .y_bounds([0.0, height])
            .paint(|ctx| {
                for ((x, y), pixel) in buffer.clone() {
                    ctx.draw(&Rectangle{
                        x: width -x as f64,
                        y: height - y as f64,
                        width: 1.0,
                        height: 1.0,
                        color: Color::Rgb(pixel[0], pixel[1], pixel[2]),
                    });
                }
            });
        f.render_widget(canvas, size);
    }).unwrap();
}
