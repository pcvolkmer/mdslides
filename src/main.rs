/*
 * This file is part of mdslides.
 *
 * Copyright (C) 2023 Paul-Christian Volkmer
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fs;
use std::io::stdout;
use std::path::Path;

use clap::Parser;
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use figlet_rs::FIGfont;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::{
    prelude::{CrosstermBackend, Stylize, Terminal},
    widgets::Paragraph,
};
use regex::Regex;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true, arg_required_else_help(true))]
pub struct Cli {
    #[arg(help = "Markdown-Datei")]
    file: String,
}

#[derive(Debug)]
pub struct Presentation {
    title: String,
    author: String,
    date: String,
    slides: Vec<Slide>,
}

impl Presentation {
    fn read(path: &Path) -> Result<Presentation, ()> {
        match fs::read_to_string(path) {
            Ok(file_content) => {
                let mut title = String::new();
                let mut author = String::new();
                let mut date = String::new();
                let mut slides = vec![];
                let mut slide_title = String::new();
                let mut slide_content = vec![];

                for (line, content) in file_content.lines().enumerate() {
                    if line == 0 && content.starts_with('%') {
                        title = content.replace("% ", "");
                    } else if line == 1 && content.starts_with('%') {
                        author = content.replace("% ", "");
                    } else if line == 2 && content.starts_with('%') {
                        date = content.replace("% ", "");
                    } else if content.starts_with("# ") {
                        if slide_title.is_empty() {
                            // Start first slide
                            slide_title = content.to_string();
                        } else {
                            slides.push(Slide {
                                title: slide_title,
                                content: slide_content.to_owned(),
                            });
                            // Start next slide
                            slide_title = content.to_string();
                            slide_content.clear();
                        }
                    } else {
                        slide_content.push(content.to_string())
                    }
                }

                slides.push(Slide {
                    title: slide_title,
                    content: slide_content.to_owned(),
                });

                Ok(Presentation {
                    title,
                    author,
                    date,
                    slides,
                })
            }
            Err(_) => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Slide {
    title: String,
    content: Vec<String>,
}

impl Slide {
    fn formatted_content(&self) -> Vec<Line> {
        let result = self.content.join("\n");
        let result = Regex::new("^\n*").unwrap().replace(&result, "");
        let result = Regex::new("\n*$").unwrap().replace(&result, "").to_string();

        let result = Regex::new("\n\n+")
            .unwrap()
            .replace(&result, "\n\n")
            .to_string();

        let mut source = false;

        result
            .trim()
            .lines()
            .map(|line| {
                if line.starts_with("##") {
                    return Some(Line::from(Span::from(line.to_string()).yellow()));
                }
                if line.trim().starts_with("* ") {
                    return Some(Line::from(vec![
                        Span::from("* ").yellow(),
                        Span::from(line.to_string().trim().replace("* ", "")),
                    ]));
                }
                if line.trim_end().starts_with("```") {
                    source = !source;
                    return None;
                }
                if source {
                    return Some(Line::from(
                        Span::from(format!(" {} ", line)).light_cyan().on_black(),
                    ));
                }
                return Some(Line::raw(line.to_string()));
            })
            .filter(Option::is_some)
            .map(|line| line.unwrap_or_default())
            .collect::<Vec<_>>()
    }
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let p = Presentation::read(Path::new(&cli.file)).unwrap();

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut slide = 0;

    loop {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Min(5), Constraint::Max(1)])
                .split(frame.size());

            if slide == 0 {
                let figlet = FIGfont::standard().unwrap();
                let figlet_output = figlet.convert(&p.title).unwrap();

                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Min((figlet_output.height + 1) as u16),
                        Constraint::Min(2),
                        Constraint::Min(2),
                    ])
                    .vertical_margin(3)
                    .horizontal_margin(3)
                    .split(layout[0]);

                frame.render_widget(
                    Paragraph::new(figlet_output.to_string())
                        .alignment(Alignment::Center)
                        .light_yellow()
                        .bold(),
                    inner_layout[0],
                );
                frame.render_widget(
                    Paragraph::new(p.author.to_string())
                        .alignment(Alignment::Center)
                        .bold(),
                    inner_layout[1],
                );
                frame.render_widget(
                    Paragraph::new(p.date.to_string())
                        .alignment(Alignment::Center)
                        .bold(),
                    inner_layout[2],
                );
            } else {
                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![Constraint::Max(2), Constraint::Min(5)])
                    .vertical_margin(2)
                    .horizontal_margin(2)
                    .split(layout[0]);

                frame.render_widget(
                    Paragraph::new(p.slides[slide - 1].title.to_string())
                        .light_yellow()
                        .bold(),
                    inner_layout[0],
                );

                frame.render_widget(
                    Paragraph::new(p.slides[slide - 1].formatted_content()),
                    inner_layout[1],
                );
            }

            let footer = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

            frame.render_widget(
                Paragraph::new(if slide == 0 {
                    p.title.to_string()
                } else {
                    format!(
                        "{} -- {}",
                        p.title,
                        p.slides[slide - 1].title.replace("# ", "")
                    )
                })
                .white()
                .bold()
                .on_blue(),
                footer[0],
            );

            frame.render_widget(
                Paragraph::new(format!("[{}/{}]", slide + 1, p.slides.len() + 1))
                    .alignment(Alignment::Right)
                    .white()
                    .on_blue(),
                footer[1],
            );
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && (key.code == KeyCode::Esc || key.code == KeyCode::Char('q'))
                {
                    break;
                }
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Left {
                    slide = slide.checked_sub(1).unwrap_or_else(|| 0);
                }
                if key.kind == KeyEventKind::Press
                    && key.code == KeyCode::Right
                    && slide < p.slides.len()
                {
                    slide += 1;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
