use clap::Parser;
use eframe::egui;
use std::time::{Duration, Instant};

mod tracker;
use tracker::TimeTrackerState;

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
    Idle,
    Work,
    Rest,
    PausedWork,
    PausedRest,
}

#[derive(PartialEq)]
enum AppView {
    Ultradian,
    Tracker,
}

struct AppState {
    view: AppView,
    
    // Ultradian fields
    ultradian_state: TimerState,
    work_duration: Duration,
    rest_duration: Duration,
    ultradian_start: Instant,
    ultradian_remaining: Duration,
    
    // Tracker fields
    tracker: TimeTrackerState,
}

impl AppState {
    fn new(work_mins: u64, rest_mins: u64) -> Self {
        let work_duration = Duration::from_secs(work_mins * 60);
        let rest_duration = Duration::from_secs(rest_mins * 60);
        Self {
            view: AppView::Ultradian,
            ultradian_state: TimerState::Idle,
            work_duration,
            rest_duration,
            ultradian_start: Instant::now(),
            ultradian_remaining: work_duration,
            tracker: TimeTrackerState::load(),
        }
    }

    fn ultradian_toggle_pause(&mut self) {
        match self.ultradian_state {
            TimerState::Idle => {
                self.ultradian_state = TimerState::Work;
                self.ultradian_start = Instant::now();
            }
            TimerState::Work => {
                self.ultradian_state = TimerState::PausedWork;
            }
            TimerState::Rest => {
                self.ultradian_state = TimerState::PausedRest;
            }
            TimerState::PausedWork => {
                self.ultradian_state = TimerState::Work;
                self.ultradian_start = Instant::now() - (self.work_duration - self.ultradian_remaining);
            }
            TimerState::PausedRest => {
                self.ultradian_state = TimerState::Rest;
                self.ultradian_start = Instant::now() - (self.rest_duration - self.ultradian_remaining);
            }
        }
    }

    fn ultradian_reset(&mut self) {
        match self.ultradian_state {
            TimerState::Idle => {}
            TimerState::Work | TimerState::PausedWork => {
                self.ultradian_state = TimerState::Work;
                self.ultradian_remaining = self.work_duration;
                self.ultradian_start = Instant::now();
            }
            TimerState::Rest | TimerState::PausedRest => {
                self.ultradian_state = TimerState::Rest;
                self.ultradian_remaining = self.rest_duration;
                self.ultradian_start = Instant::now();
            }
        }
    }

    fn tick(&mut self, ctx: &egui::Context) {
        // Ultradian tick
        if self.ultradian_state == TimerState::Work || self.ultradian_state == TimerState::Rest {
            let elapsed = self.ultradian_start.elapsed();
            let total = match self.ultradian_state {
                TimerState::Work => self.work_duration,
                TimerState::Rest => self.rest_duration,
                _ => unreachable!(),
            };

            if elapsed >= total {
                match self.ultradian_state {
                    TimerState::Work => {
                        self.ultradian_state = TimerState::Rest;
                        self.ultradian_remaining = self.rest_duration;
                        self.ultradian_start = Instant::now();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                    }
                    TimerState::Rest => {
                        self.ultradian_state = TimerState::Work;
                        self.ultradian_remaining = self.work_duration;
                        self.ultradian_start = Instant::now();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    }
                    _ => unreachable!(),
                }
            } else {
                self.ultradian_remaining = total - elapsed;
            }
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);
        ctx.request_repaint_after(Duration::from_millis(200));

        let is_rest = self.view == AppView::Ultradian 
            && (self.ultradian_state == TimerState::Rest || self.ultradian_state == TimerState::PausedRest);

        let frame_style = if is_rest {
            egui::Frame::default().fill(egui::Color32::BLACK)
        } else {
            egui::Frame::default().fill(egui::Color32::from_rgb(15, 20, 25))
        };

        if !is_rest {
            egui::TopBottomPanel::top("top_panel").frame(egui::Frame::default().fill(egui::Color32::from_rgb(25, 30, 35)).inner_margin(8.0)).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.view, AppView::Ultradian, "🍅 Ultradian Timer");
                    ui.selectable_value(&mut self.view, AppView::Tracker, "⏱ Time Tracker");
                });
            });
        }

        egui::CentralPanel::default().frame(frame_style).show(ctx, |ui| {
            match self.view {
                AppView::Ultradian => self.ui_ultradian(ctx, ui, is_rest),
                AppView::Tracker => self.tracker.ui(ctx, ui),
            }
        });
    }
}

impl AppState {
    fn ui_ultradian(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, is_rest: bool) {
        // Keyboard inputs for Ultradian
        if ctx.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter)) {
            self.ultradian_toggle_pause();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.ultradian_reset();
        }

        ui.centered_and_justified(|ui| {
            let secs = self.ultradian_remaining.as_secs();
            let display = format!("{:02}:{:02}", secs / 60, secs % 60);

            if is_rest {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 100.0);
                    ui.label(egui::RichText::new("Descanso Neurológico").color(egui::Color32::DARK_GRAY).size(30.0));
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new(display).color(egui::Color32::DARK_GRAY).size(80.0).strong());
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("No pantallas. Cero ingresos cognitivos.").color(egui::Color32::DARK_GRAY).size(20.0).italics());
                    if self.ultradian_state == TimerState::PausedRest {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("(PAUSADO)").color(egui::Color32::DARK_GRAY).size(20.0));
                    }
                });
            } else {
                let (status, color) = match self.ultradian_state {
                    TimerState::Idle => ("ESPERANDO INICIO", egui::Color32::CYAN),
                    TimerState::Work => ("TRABAJO PROFUNDO", egui::Color32::GREEN),
                    TimerState::PausedWork => ("PAUSADO", egui::Color32::YELLOW),
                    _ => ("", egui::Color32::WHITE),
                };

                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 150.0);
                    ui.label(egui::RichText::new(status).color(color).size(40.0).strong());
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new(display).color(color).size(120.0).strong());
                    ui.add_space(60.0);
                    
                    let help_text = if self.ultradian_state == TimerState::Idle {
                        "[Enter] o [Espacio] Iniciar"
                    } else {
                        "[Espacio] Pausa/Reanudar | [R] Reiniciar fase"
                    };
                    ui.label(egui::RichText::new(help_text).color(egui::Color32::GRAY).size(20.0));
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let args = Args::parse();
    let app = AppState::new(args.work, args.rest);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Ultradian Timer"),
        ..Default::default()
    };

    eframe::run_native(
        "Ultradian Timer",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
