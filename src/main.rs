use eframe::egui;
use std::time::{Duration, Instant};
use clap::Parser;

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

struct UltradianApp {
    state: TimerState,
    work_duration: Duration,
    rest_duration: Duration,
    timer_start: Instant,
    time_remaining: Duration,
}

impl UltradianApp {
    fn new(work_mins: u64, rest_mins: u64) -> Self {
        let work_duration = Duration::from_secs(work_mins * 60);
        let rest_duration = Duration::from_secs(rest_mins * 60);
        Self {
            state: TimerState::Idle,
            work_duration,
            rest_duration,
            timer_start: Instant::now(),
            time_remaining: work_duration,
        }
    }

    fn toggle_pause(&mut self) {
        match self.state {
            TimerState::Idle => {
                self.state = TimerState::Work;
                self.timer_start = Instant::now();
            }
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
            TimerState::Idle => {}
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

    fn tick(&mut self, ctx: &egui::Context) {
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
                        // Al entrar en descanso, forzamos pantalla completa
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                    }
                    TimerState::Rest => {
                        self.state = TimerState::Work;
                        self.time_remaining = self.work_duration;
                        self.timer_start = Instant::now();
                        // Al terminar descanso, salimos de pantalla completa
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    }
                    _ => unreachable!(),
                }
            } else {
                self.time_remaining = total - elapsed;
            }
        }
    }
}

impl eframe::App for UltradianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);

        // Repintar frecuentemente para mantener el contador visualmente al día
        ctx.request_repaint_after(Duration::from_millis(200));

        // Entradas de teclado
        if ctx.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter)) {
            self.toggle_pause();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.reset();
        }

        let is_rest = self.state == TimerState::Rest || self.state == TimerState::PausedRest;

        let frame_style = if is_rest {
            egui::Frame::default().fill(egui::Color32::BLACK)
        } else {
            egui::Frame::default().fill(egui::Color32::from_rgb(15, 20, 25))
        };

        egui::CentralPanel::default().frame(frame_style).show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let secs = self.time_remaining.as_secs();
                let display = format!("{:02}:{:02}", secs / 60, secs % 60);

                if is_rest {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 2.0 - 100.0); // Simple center offset
                        ui.label(egui::RichText::new("Descanso Neurológico").color(egui::Color32::DARK_GRAY).size(30.0));
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(display).color(egui::Color32::DARK_GRAY).size(80.0).strong());
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("No pantallas. Cero ingresos cognitivos.").color(egui::Color32::DARK_GRAY).size(20.0).italics());
                        if self.state == TimerState::PausedRest {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new("(PAUSADO)").color(egui::Color32::DARK_GRAY).size(20.0));
                        }
                    });
                } else {
                    let (status, color) = match self.state {
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
                        
                        let help_text = if self.state == TimerState::Idle {
                            "[Enter] o [Espacio] Iniciar"
                        } else {
                            "[Espacio] Pausa/Reanudar | [R] Reiniciar fase"
                        };
                        ui.label(egui::RichText::new(help_text).color(egui::Color32::GRAY).size(20.0));
                    });
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let args = Args::parse();
    let app = UltradianApp::new(args.work, args.rest);

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
