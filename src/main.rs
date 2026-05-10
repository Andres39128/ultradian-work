use clap::Parser;
use eframe::egui;
use std::time::{Duration, Instant};
use notify_rust::Notification;
use std::process::Command;

mod i18n;
mod models;
mod tracker;
use tracker::TimeTrackerState;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 90)]
    work: u64,
    #[arg(short, long, default_value_t = 15)]
    rest: u64,
}

#[derive(PartialEq)]
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
    Tasks,
    Dashboard,
    Settings,
}

struct AppState {
    view: AppView,
    ultradian_state: TimerState,
    ultradian_start: Instant,
    ultradian_remaining: Duration,
    tracker: tracker::TimeTrackerState,
}

impl AppState {
    fn new(work_mins: u64, rest_mins: u64) -> Self {
        let mut tracker = TimeTrackerState::load();
        if tracker.data.work_duration_mins == 0 { tracker.data.work_duration_mins = work_mins; }
        if tracker.data.rest_duration_mins == 0 { tracker.data.rest_duration_mins = rest_mins; }
        let work_duration = Duration::from_secs(tracker.data.work_duration_mins * 60);

        Self {
            view: AppView::Ultradian,
            ultradian_state: TimerState::Idle,
            ultradian_start: Instant::now(),
            ultradian_remaining: work_duration,
            tracker,
        }
    }

    fn work_dur(&self) -> Duration { Duration::from_secs(self.tracker.data.work_duration_mins * 60) }
    fn rest_dur(&self) -> Duration { Duration::from_secs(self.tracker.data.rest_duration_mins * 60) }
    fn notify(&self, title: &str, body: &str) {
        let _ = Notification::new().summary(title).body(body).show();
        let _ = Command::new("paplay").arg("/usr/share/sounds/freedesktop/stereo/complete.oga").spawn();
    }

    fn ultradian_toggle_pause(&mut self, ctx: &egui::Context) {
        match self.ultradian_state {
            TimerState::Idle => {
                self.ultradian_state = TimerState::Work;
                self.ultradian_start = Instant::now();
                self.ultradian_remaining = self.work_dur();
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            TimerState::Work => {
                self.ultradian_state = TimerState::PausedWork;
                self.ultradian_remaining -= self.ultradian_start.elapsed();
            }
            TimerState::Rest => {
                self.ultradian_state = TimerState::PausedRest;
                self.ultradian_remaining -= self.ultradian_start.elapsed();
            }
            TimerState::PausedWork => {
                self.ultradian_state = TimerState::Work;
                self.ultradian_start = Instant::now() - (self.work_dur() - self.ultradian_remaining);
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            TimerState::PausedRest => {
                self.ultradian_state = TimerState::Rest;
                self.ultradian_start = Instant::now() - (self.rest_dur() - self.ultradian_remaining);
            }
        }
    }

    fn ultradian_restart_phase(&mut self) {
        match self.ultradian_state {
            TimerState::Idle => {}
            TimerState::Work | TimerState::PausedWork => {
                self.ultradian_state = TimerState::Work;
                self.ultradian_remaining = self.work_dur();
                self.ultradian_start = Instant::now();
            }
            TimerState::Rest | TimerState::PausedRest => {
                self.ultradian_state = TimerState::Rest;
                self.ultradian_remaining = self.rest_dur();
                self.ultradian_start = Instant::now();
            }
        }
    }

    fn tick(&mut self, ctx: &egui::Context) {
        if self.ultradian_state == TimerState::Work || self.ultradian_state == TimerState::Rest {
            let elapsed = self.ultradian_start.elapsed();
            let total = match self.ultradian_state {
                TimerState::Work => self.work_dur(),
                TimerState::Rest => self.rest_dur(),
                _ => unreachable!(),
            };

            if elapsed >= total {
                match self.ultradian_state {
                    TimerState::Work => {
                        self.ultradian_state = TimerState::Rest;
                        self.ultradian_remaining = self.rest_dur();
                        self.ultradian_start = Instant::now();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                        self.notify("Tiempo de Descanso", "¡A descansar!");
                    }
                    TimerState::Rest => {
                        self.ultradian_state = TimerState::Work;
                        self.ultradian_remaining = self.work_dur();
                        self.ultradian_start = Instant::now();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        self.notify("Tiempo de Trabajo", "De vuelta al trabajo.");
                    }
                    _ => unreachable!(),
                }
            } else {
                self.ultradian_remaining = total - elapsed;
            }
            ctx.request_repaint();
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);

        let is_rest = self.ultradian_state == TimerState::Rest || self.ultradian_state == TimerState::PausedRest;
        
        let frame_style = if is_rest {
            egui::Frame::default().fill(egui::Color32::BLACK)
        } else {
            egui::Frame::default().fill(ctx.style().visuals.panel_fill).inner_margin(16.0)
        };

        if !is_rest {
            egui::TopBottomPanel::top("top_panel").frame(egui::Frame::default().fill(egui::Color32::from_rgb(25, 30, 35)).inner_margin(8.0)).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let lang = self.tracker.data.language; // Copying because it's a Copy enum
                    ui.selectable_value(&mut self.view, AppView::Ultradian, crate::i18n::t(&lang, "tab_ultradian"));
                    ui.selectable_value(&mut self.view, AppView::Tracker, crate::i18n::t(&lang, "tab_tracker"));
                    ui.selectable_value(&mut self.view, AppView::Tasks, crate::i18n::t(&lang, "tab_tasks"));
                    ui.selectable_value(&mut self.view, AppView::Dashboard, crate::i18n::t(&lang, "tab_dashboard"));
                    ui.selectable_value(&mut self.view, AppView::Settings, crate::i18n::t(&lang, "tab_settings"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut current_lang = self.tracker.data.language;
                        egui::ComboBox::from_id_salt("lang_select").selected_text(match current_lang { crate::i18n::Language::En => "EN", crate::i18n::Language::Es => "ES" }).show_ui(ui, |ui| {
                            ui.selectable_value(&mut current_lang, crate::i18n::Language::Es, "ES");
                            ui.selectable_value(&mut current_lang, crate::i18n::Language::En, "EN");
                        });
                        if current_lang != self.tracker.data.language { self.tracker.data.language = current_lang; self.tracker.save(); }
                    });
                });
            });
        }

        egui::CentralPanel::default().frame(frame_style).show(ctx, |ui| {
            match self.view {
                AppView::Ultradian => self.ui_ultradian(ctx, ui, is_rest),
                AppView::Tracker => self.tracker.ui(ctx, ui),
                AppView::Dashboard => self.tracker.ui_dashboard(ctx, ui),
                AppView::Settings => self.tracker.ui_settings(ctx, ui),
                AppView::Tasks => {
                    if self.tracker.ui_tasks(ctx, ui) {
                        self.view = AppView::Tracker;
                    }
                }
            }
        });
    }
}

impl AppState {
    fn ui_ultradian(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, is_rest: bool) {
        let lang = self.tracker.data.language;
        if ctx.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter)) {
            self.ultradian_toggle_pause(ctx);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.ultradian_restart_phase();
        }

        let secs = self.ultradian_remaining.as_secs();
        let display = format!("{:02}:{:02}", secs / 60, secs % 60);

        ui.centered_and_justified(|ui| {
            if is_rest {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 100.0);
                    ui.label(egui::RichText::new(crate::i18n::t(&lang, "ultradian_rest_title")).color(egui::Color32::DARK_GRAY).size(30.0));
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new(display).color(egui::Color32::DARK_GRAY).size(80.0).strong());
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new(crate::i18n::t(&lang, "ultradian_rest_desc")).color(egui::Color32::DARK_GRAY).size(20.0).italics());
                    if self.ultradian_state == TimerState::PausedRest {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(crate::i18n::t(&lang, "ultradian_paused")).color(egui::Color32::DARK_GRAY).size(20.0));
                    }
                });
            } else {
                let (status, color) = match self.ultradian_state {
                    TimerState::Idle => (crate::i18n::t(&lang, "ultradian_idle"), egui::Color32::CYAN),
                    TimerState::Work => (crate::i18n::t(&lang, "ultradian_work"), egui::Color32::GREEN),
                    TimerState::PausedWork => (crate::i18n::t(&lang, "ultradian_paused"), egui::Color32::YELLOW),
                    _ => ("", egui::Color32::WHITE),
                };

                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 120.0);
                    ui.label(egui::RichText::new(status).color(color).size(30.0));
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new(display).size(120.0).strong());
                    ui.add_space(60.0);
                    
                    let help_text = if self.ultradian_state == TimerState::Idle {
                        crate::i18n::t(&lang, "ultradian_help_start")
                    } else {
                        crate::i18n::t(&lang, "ultradian_help_pause")
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
            .with_title("Ultradian Work"),
        ..Default::default()
    };

    eframe::run_native(
        "Ultradian Work",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
