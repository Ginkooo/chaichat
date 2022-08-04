use std::io::{stdout, Stdout};

use crate::camera_frame::CameraFrame;
use crate::commands::handle_command;
use crate::types::Message;
use async_std::channel::{Receiver, Sender};
use crossbeam::channel::Receiver as SyncReceiver;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use tui::layout::Rect;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::Color;
use tui::symbols::Marker;
use tui::widgets::canvas::{Canvas, Rectangle};
use tui::widgets::Block;
use tui::widgets::Borders;
use tui::widgets::Paragraph;
use tui::{backend::CrosstermBackend, Terminal};

use crate::types::Res;

pub struct ChaiTerminal {
    prev_camera_frame: Option<CameraFrame>,
    inner_terminal: Terminal<CrosstermBackend<Stdout>>,
    text_area_content: Vec<String>,
}

impl ChaiTerminal {
    fn prepare_terminal_for_drawing() -> Res<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode().unwrap();
        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend).expect("failed to create terminal instance");
        terminal.clear().expect("failed to clear terminal screen");
        Ok(terminal)
    }
    pub fn init() -> Res<ChaiTerminal> {
        let terminal = ChaiTerminal::prepare_terminal_for_drawing()?;
        Ok(ChaiTerminal {
            prev_camera_frame: None,
            inner_terminal: terminal,
            text_area_content: vec![String::from("")],
        })
    }

    pub fn uninit(self: Self) {
        disable_raw_mode().expect("Cannot disable terminal raw mode");
    }

    pub fn draw_in_terminal(
        self: &mut Self,
        camera_frames: SyncReceiver<CameraFrame>,
        input_events: SyncReceiver<Event>,
        in_p2p_receiver: Receiver<Message>,
        out_p2p_sender: Sender<Message>,
    ) -> Res<()> {
        let size = self
            .inner_terminal
            .size()
            .expect("Terminal should be working by now");
        let height = size.height;
        let width = size.width;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(size);

        let video_block = Block::default().borders(Borders::all()).title("video");
        let input_block = Block::default().borders(Borders::all()).title("input");
        let input_paragraph = Paragraph::new(self.text_area_content.join("\n"));
        let input_paragraph_rect = Rect {
            x: chunks[1].x + 1,
            y: chunks[1].y + 1,
            width: chunks[1].width - 1,
            height: chunks[1].height - 1,
        };

        let text_area_content = &mut self.text_area_content;
        match input_events.try_recv() {
            Ok(event) => match event {
                Event::Key(key) => match key.code {
                    KeyCode::Char(chr) => {
                        text_area_content.last_mut().unwrap().push(chr);
                    }
                    KeyCode::Backspace => {
                        text_area_content.last_mut().unwrap().pop();
                    }
                    KeyCode::Enter => {
                        let response = handle_command(
                            text_area_content.last().unwrap(),
                            out_p2p_sender.clone(),
                        );
                        text_area_content.push(response);
                        text_area_content.push(String::from(""));
                    }
                    KeyCode::Esc => {
                        panic!();
                    }
                    _ => {}
                },
                Event::Mouse(mouse) => {}
                Event::Resize(x, y) => {}
            },
            Err(_) => {}
        }

        let camera_frame = camera_frames.try_recv();
        if camera_frame.is_err() {
            return Ok(());
        }
        self.prev_camera_frame = Some(camera_frame.expect("Camera frame should exist by now"));
        let mut camera_frame = self
            .prev_camera_frame
            .clone()
            .expect("Prev camera frame should exist by now");
        camera_frame = camera_frame
            .clone()
            .resize((width as f64 * 0.3) as u16, (height as f64 * 0.3) as u16);
        self.prev_camera_frame = Some(camera_frame.clone());

        let cam_feedback_rect =
            Rect::new(1, 1, camera_frame.resolution.0, camera_frame.resolution.1);

        self.inner_terminal.draw(|frame| {
            let camera_feedback = Canvas::default()
                .marker(Marker::Braille)
                .x_bounds([0., camera_frame.resolution.0 as f64])
                .y_bounds([0., camera_frame.resolution.1 as f64])
                .paint(|ctx| {
                    for ((x, y), color) in camera_frame.clone().get_pixels().iter() {
                        let rect = &Rectangle {
                            x: (camera_frame.resolution.0 - x.to_owned()) as f64,
                            y: (camera_frame.resolution.1 - y) as f64,
                            width: 1.,
                            height: 1.,
                            color: Color::Rgb(color[0], color[1], color[2]),
                        };
                        ctx.draw(rect);
                    }
                });

            frame.render_widget(video_block, chunks[0]);
            frame.render_widget(input_block, chunks[1]);
            frame.render_widget(input_paragraph, input_paragraph_rect);
            frame.render_widget(camera_feedback, cam_feedback_rect);
        })?;

        Ok(())
    }
}
