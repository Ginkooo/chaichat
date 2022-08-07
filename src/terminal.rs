use std::io::{stdout, Stdout};

use crate::camera_frame::CameraFrame;
use crate::commands::handle_command;
use crate::types::CameraImage;
use crate::types::Message;
use async_std::channel::{Receiver, Sender};
use async_std::stream::StreamExt;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures::executor::block_on;

use async_std::io;
use std::mem;
use std::time::Duration;
use tui::layout::Rect;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::Color;
use tui::style::Style;
use tui::symbols::Marker;
use tui::text::Span;
use tui::text::Spans;
use tui::text::Text;
use tui::widgets::canvas::{Canvas, Rectangle};
use tui::widgets::Block;
use tui::widgets::Borders;
use tui::widgets::Paragraph;
use tui::{backend::CrosstermBackend, Terminal};

use crate::types::Res;

pub struct ChaiTerminal<'a> {
    prev_camera_frame: Option<CameraFrame>,
    inner_terminal: Terminal<CrosstermBackend<Stdout>>,
    text_area_content: Text<'a>,
}

impl<'a> ChaiTerminal<'a> {
    fn prepare_terminal_for_drawing() -> Res<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode().unwrap();
        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend).expect("failed to create terminal instance");
        terminal.clear().expect("failed to clear terminal screen");
        Ok(terminal)
    }
    pub fn init<'b>() -> Res<ChaiTerminal<'b>> {
        let terminal = ChaiTerminal::prepare_terminal_for_drawing()?;
        Ok(ChaiTerminal {
            prev_camera_frame: None,
            inner_terminal: terminal,
            text_area_content: Text::from("\n\n"),
        })
    }

    pub fn uninit(self: Self) {
        disable_raw_mode().expect("Cannot disable terminal raw mode");
    }

    pub fn draw_in_terminal(
        self: &mut Self,
        mut camera_frames: Receiver<CameraFrame>,
        input_events: Receiver<Event>,
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
        let mut in_camera_frame = CameraFrame::from_camera_image(CameraImage::new(640, 480));
        match in_p2p_receiver.try_recv() {
            Ok(Message::Text(msg)) => {
                self.text_area_content
                    .lines
                    .push(vec![Span::styled(msg, Style::default().fg(Color::Yellow))].into());
                self.text_area_content
                    .lines
                    .push(vec![Span::raw("")].into());
            }
            Ok(Message::RawCameraImage(raw)) => {
                in_camera_frame =
                    CameraFrame::from_camera_image(CameraImage::from_raw(640, 480, raw).unwrap());
            }
            Ok(_) => {}
            Err(_) => (),
        }
        let input_paragraph = Paragraph::new(self.text_area_content.clone())
            .scroll(((self.text_area_content.lines.len() as u16 - 3).max(0), 0));
        let input_paragraph_rect = Rect {
            x: chunks[1].x + 1,
            y: chunks[1].y + 1,
            width: chunks[1].width - 1,
            height: chunks[1].height - 1,
        };

        match input_events.try_recv() {
            Ok(event) => match event {
                Event::Key(key) => match key.code {
                    KeyCode::Char(chr) => {
                        let last_content = self.get_text_area_last_span_content();
                        let span = Span::styled(
                            last_content + &chr.to_string(),
                            Style::default().fg(Color::Blue),
                        );

                        let _ = mem::replace(
                            self.text_area_content.lines.last_mut().unwrap(),
                            Spans::from(vec![span]),
                        );
                    }
                    KeyCode::Backspace => {
                        let mut content = self.get_text_area_last_span_content();
                        content.pop();
                        let span = Span::styled(content, Style::default().fg(Color::Blue));

                        let _ = mem::replace(
                            self.text_area_content.lines.last_mut().unwrap(),
                            Spans::from(vec![span]),
                        );
                    }
                    KeyCode::Enter => {
                        let response = match handle_command(
                            &self.get_text_area_last_span_content(),
                            out_p2p_sender.clone(),
                        ) {
                            Ok(response) => response,
                            Err(_) => String::from("Error"),
                        };

                        let span = Span::styled(response.clone(), Style::default().fg(Color::Red));
                        let new_input_line_span =
                            Span::styled("", Style::default().fg(Color::Blue));

                        if !response.is_empty() {
                            self.text_area_content.lines.push(vec![span].into());
                        }
                        self.text_area_content
                            .lines
                            .push(vec![new_input_line_span].into());
                    }
                    KeyCode::Esc => {
                        panic!();
                    }
                    _ => {}
                },
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
            },
            Err(_) => {}
        }

        let mut camera_frame = block_on(io::timeout(Duration::from_secs(1), async {
            camera_frames
                .next()
                .await
                .ok_or(std::io::Error::last_os_error())
        }))
        .unwrap_or(CameraFrame::from_camera_image(CameraImage::new(640, 480)));
        let serializable_camera = camera_frame.camera_image.clone().into_raw();
        block_on(out_p2p_sender.send(Message::RawCameraImage(serializable_camera))).unwrap();

        let new_width = (width as f64 * 0.2) as u16;
        let new_height = (height as f64 * 0.2) as u16;
        if new_height != camera_frame.camera_image.height() as u16
            || new_width != camera_frame.camera_image.width() as u16
        {
            camera_frame =
                camera_frame.resize((width as f64 * 0.3) as u16, (height as f64 * 0.3) as u16);
        }
        self.prev_camera_frame = Some(camera_frame.clone());

        let cam_feedback_rect =
            Rect::new(1, 1, camera_frame.resolution.0, camera_frame.resolution.1);

        let pixels = camera_frame.get_pixels();
        let in_camera_pixels = in_camera_frame.get_pixels();

        self.inner_terminal.draw(|frame| {
            let received_camera = Canvas::default()
                .marker(Marker::Braille)
                .x_bounds([0., in_camera_frame.resolution.0 as f64])
                .y_bounds([0., in_camera_frame.resolution.1 as f64])
                .paint(|ctx| {
                    for ((x, y), color) in in_camera_pixels.iter() {
                        let rect = &Rectangle {
                            x: (in_camera_frame.resolution.0 - x.to_owned()) as f64,
                            y: (in_camera_frame.resolution.1 - y) as f64,
                            width: 1.,
                            height: 1.,
                            color: Color::Rgb(color[0], color[1], color[2]),
                        };
                        ctx.draw(rect);
                    }
                });

            let camera_feedback = Canvas::default()
                .marker(Marker::Braille)
                .x_bounds([0., camera_frame.resolution.0 as f64])
                .y_bounds([0., camera_frame.resolution.1 as f64])
                .paint(|ctx| {
                    for ((x, y), color) in pixels.iter() {
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

            frame.render_widget(received_camera, chunks[0]);
            frame.render_widget(input_block, chunks[1]);
            frame.render_widget(input_paragraph, input_paragraph_rect);
            frame.render_widget(camera_feedback, cam_feedback_rect);
        })?;

        Ok(())
    }

    fn get_text_area_last_span_content(&mut self) -> String {
        self.text_area_content
            .lines
            .last_mut()
            .unwrap()
            .0
            .iter()
            .map(|span| span.content.to_string())
            .reduce(|mut acc, it| {
                acc.push_str(&it);
                acc
            })
            .unwrap_or(String::new())
    }
}
