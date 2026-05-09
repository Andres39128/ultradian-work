use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Duración del bloque de trabajo en minutos
    #[arg(short, long, default_value_t = 90)]
    work: u64,

    /// Duración del bloque de descanso en minutos
    #[arg(short, long, default_value_t = 15)]
    rest: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum TimerState {
    Work,
    Rest,
    PausedWork,
    PausedRest,
}

struct App {
    state: TimerState,
    work_duration: Duration,
    rest_duration: Duration,
    timer_start: Instant,
    time_remaining: Duration,
    should_quit: bool,
}

impl App {
    fn new(work_mins: u64, rest_mins: u64) -> App {
        let work_duration = Duration::from_secs(work_mins * 60);
        let rest_duration = Duration::from_secs(rest_mins * 60);
        App {
            state: TimerState::Work,
            work_duration,
            rest_duration,
            timer_start: Instant::now(),
            time_remaining: work_duration,
            should_quit: false,
        }
    }

    fn tick(&mut self) {
        if self.state == TimerState::Work || self.state == TimerState::Rest {
            let elapsed = self.timer_start.elapsed();
            let total = match self.state {
                TimerState::Work => self.work_duration,
                TimerState::Rest => self.rest_duration,
                _ => unreachable!(),
            };

            if elapsed >= total {
                // Transition state
                match self.state {
                    TimerState::Work => {
                        self.state = TimerState::Rest;
                        self.time_remaining = self.rest_duration;
                        self.timer_start = Instant::now();
                    }
                    TimerState::Rest => {
                        self.state = TimerState::Work;
                        self.time_remaining = self.work_duration;
                        self.timer_start = Instant::now();
                    }
                    _ => unreachable!(),
                }
            } else {
                self.time_remaining = total - elapsed;
            }
        }
    }

    fn toggle_pause(&mut self) {
        match self.state {
            TimerState::Work => {
                self.state = TimerState::PausedWork;
            }
            TimerState::Rest => {
                self.state = TimerState::PausedRest;
            }
            TimerState::PausedWork => {
                self.state = TimerState::Work;
                // Adjust start time to account for pause
                self.timer_start = Instant::now() - (self.work_duration - self.time_remaining);
            }
            TimerState::PausedRest => {
                self.state = TimerState::Rest;
                self.timer_start = Instant::now() - (self.rest_duration - self.time_remaining);
            }
        }
    }

    fn reset(&mut self) {
        match self.state {
            TimerState::Work | TimerState::PausedWork => {
                self.state = TimerState::Work;
                self.time_remaining = self.work_duration;
                self.timer_start = Instant::now();
            }
            TimerState::Rest | TimerState::PausedRest => {
                self.state = TimerState::Rest;
                self.time_remaining = self.rest_duration;
                self.timer_start = Instant::now();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(args.work, args.rest);

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn Error>> 
where 
    <B as ratatui::backend::Backend>::Error: 'static,
{
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    while !app.should_quit {
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                        KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Char('r') => app.reset(),
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    if app.state == TimerState::Rest || app.state == TimerState::PausedRest {
        // En descanso, pantalla completamente en negro con un contador mínimo
        let secs = app.time_remaining.as_secs();
        let display = format!("{:02}:{:02}", secs / 60, secs % 60);
        
        let mut status = "Descanso Neurologico";
        if app.state == TimerState::PausedRest {
            status = "Descanso (Pausado)";
        }

        let block = Block::default().style(Style::default().bg(Color::Black).fg(Color::DarkGray));
        f.render_widget(block, f.area());

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(3),
                Constraint::Percentage(45),
            ])
            .split(f.area());

        let text = vec![
            Line::from(Span::styled(status, Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled(display, Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("No pantallas. Cero ingresos cognitivos.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))),
        ];

        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        f.render_widget(paragraph, layout[1]);

        return;
    }

    // Modo trabajo
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(2),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let title = Paragraph::new(" Ritmos Ultradianos - Trabajo Profundo ")
        .style(title_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).style(title_style));
    f.render_widget(title, chunks[0]);

    let secs = app.time_remaining.as_secs();
    let display = format!("{:02}:{:02}", secs / 60, secs % 60);

    let (status_text, color) = match app.state {
        TimerState::Work => ("TRABAJO PROFUNDO", Color::Green),
        TimerState::PausedWork => ("PAUSADO", Color::Yellow),
        _ => ("", Color::White),
    };

    let timer_text = vec![
        Line::from(Span::styled(status_text, Style::default().fg(color).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(display, Style::default().fg(color).add_modifier(Modifier::BOLD))),
    ];

    let timer_para = Paragraph::new(timer_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    
    // Centrar verticalmente el temporizador
    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(4),
            Constraint::Percentage(40),
        ])
        .split(chunks[1]);
        
    f.render_widget(timer_para, inner_layout[1]);

    let help_text = " [Espacio] Pausa/Reanudar | [r] Reiniciar fase | [q] Salir ";
    let help_para = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help_para, chunks[2]);
}
