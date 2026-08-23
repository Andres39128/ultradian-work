use clap::Parser;
use eframe::egui;
use std::time::{Duration, Instant};
use notify_rust::Notification;
use std::process::Command;

mod i18n;
mod models;
mod screen;
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

fn exit_rest_viewport(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
}

struct AppState {
    view: AppView,
    ultradian_state: TimerState,
    ultradian_start: Instant,
    ultradian_remaining: Duration,
    cycle_count: u32,
    tracker: tracker::TimeTrackerState,
    breath_start: Instant,
    screen_tools: screen::ScreenTools,
}

impl AppState {
    fn new(work_mins: u64, rest_mins: u64) -> Self {
        let mut tracker = TimeTrackerState::load();
        if tracker.data.work_duration_mins == 0 { tracker.data.work_duration_mins = work_mins; }
        if tracker.data.rest_duration_mins == 0 { tracker.data.rest_duration_mins = rest_mins; }
        let work_duration = Duration::from_secs(tracker.data.work_duration_mins * 60);
        let cycle_count = tracker.data.ultradian_cycles_completed;

        Self {
            view: AppView::Ultradian,
            ultradian_state: TimerState::Idle,
            ultradian_start: Instant::now(),
            ultradian_remaining: work_duration,
            cycle_count,
            tracker,
            breath_start: Instant::now(),
            screen_tools: screen::ScreenTools::detect(),
        }
    }

    fn work_dur(&self) -> Duration { Duration::from_secs(self.tracker.data.work_duration_mins * 60) }
    fn rest_dur(&self) -> Duration { Duration::from_secs(self.tracker.data.rest_duration_mins * 60) }
    fn notify(&self, title: &str, body: &str) {
        let _ = Notification::new().summary(title).body(body).show();
        let _ = Command::new("paplay").arg("/usr/share/sounds/freedesktop/stereo/complete.oga").spawn();
    }

    fn today_total_secs(&self) -> u64 {
        self.tracker.data.projects.iter().map(|p| p.today_duration_secs()).sum()
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
                // remaining is already up-to-date from tick()
            }
            TimerState::Rest => {
                self.ultradian_state = TimerState::PausedRest;
                // remaining is already up-to-date from tick()
                if self.tracker.data.screen_dim_during_rest {
                    crate::screen::restore_screen();
                }
            }
            TimerState::PausedWork => {
                self.ultradian_state = TimerState::Work;
                let dur = self.work_dur();
                if self.ultradian_remaining > dur {
                    self.ultradian_remaining = dur;
                }
                let elapsed = dur.saturating_sub(self.ultradian_remaining);
                self.ultradian_start = Instant::now() - elapsed;
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            TimerState::PausedRest => {
                self.ultradian_state = TimerState::Rest;
                let dur = self.rest_dur();
                if self.ultradian_remaining > dur {
                    self.ultradian_remaining = dur;
                }
                let elapsed = dur.saturating_sub(self.ultradian_remaining);
                self.ultradian_start = Instant::now() - elapsed;
                self.breath_start = Instant::now();
                if self.tracker.data.screen_dim_during_rest {
                    crate::screen::dim_screen();
                }
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
                self.breath_start = Instant::now();
                if self.tracker.data.screen_dim_during_rest {
                    crate::screen::dim_screen();
                }
            }
        }
    }

    fn skip_rest(&mut self, ctx: &egui::Context) {
        let lang = self.tracker.data.language;
        if self.tracker.data.screen_dim_during_rest {
            crate::screen::restore_screen();
        }
        if self.tracker.data.screen_lock_during_rest {
            crate::screen::unlock_screen();
        }
        self.ultradian_state = TimerState::Work;
        self.ultradian_remaining = self.work_dur();
        self.ultradian_start = Instant::now();
        exit_rest_viewport(ctx);
        self.notify(
            crate::i18n::t(&lang, "notification_work_title"),
            crate::i18n::t(&lang, "notification_work_body"),
        );
    }

    fn log_ultradian_session(&mut self) {
        let work_mins = self.tracker.data.work_duration_mins;
        let now = chrono::Local::now();
        let end = now.timestamp() as u64;
        let start = end.saturating_sub(work_mins * 60);
        let today = now.format("%Y-%m-%d").to_string();

        let project_id = if let Some(active_id) = &self.tracker.active_project_id {
            active_id.clone()
        } else if let Some(proj) = self.tracker.data.projects.first() {
            proj.id.clone()
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            self.tracker.data.projects.push(models::Project {
                id: id.clone(),
                name: crate::i18n::t(&self.tracker.data.language, "ultradian_project").to_string(),
                sessions: Vec::new(),
            });
            id
        };

        if let Some(proj) = self.tracker.data.projects.iter_mut().find(|p| p.id == project_id) {
            let lang = self.tracker.data.language;
            proj.sessions.push(models::Session {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("{} {}", crate::i18n::t(&lang, "cycle_label"), self.cycle_count + 1),
                start_time: start,
                end_time: end,
                sub_sessions: Vec::new(),
            });
        }

        self.cycle_count += 1;
        self.tracker.data.ultradian_cycles_completed = self.cycle_count;
        self.tracker.data.last_session_date = today;
        self.tracker.save();
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
                let lang = self.tracker.data.language;
                match self.ultradian_state {
                    TimerState::Work => {
                        self.log_ultradian_session();
                        self.ultradian_state = TimerState::Rest;
                        self.ultradian_remaining = self.rest_dur();
                        self.ultradian_start = Instant::now();
                        self.breath_start = Instant::now();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        self.notify(
                            crate::i18n::t(&lang, "notification_rest_title"),
                            crate::i18n::t(&lang, "notification_rest_body"),
                        );
                        if self.tracker.data.screen_dim_during_rest {
                            crate::screen::dim_screen();
                        }
                        if self.tracker.data.screen_lock_during_rest {
                            crate::screen::lock_screen();
                        }
                    }
                    TimerState::Rest => {
                        self.ultradian_state = TimerState::Work;
                        self.ultradian_remaining = self.work_dur();
                        self.ultradian_start = Instant::now();
                        exit_rest_viewport(ctx);
                        if self.tracker.data.screen_dim_during_rest {
                            crate::screen::restore_screen();
                        }
                        if self.tracker.data.screen_lock_during_rest {
                            crate::screen::unlock_screen();
                        }
                        self.notify(
                            crate::i18n::t(&lang, "notification_work_title"),
                            crate::i18n::t(&lang, "notification_work_body"),
                        );
                    }
                    _ => unreachable!(),
                }
            } else {
                self.ultradian_remaining = total - elapsed;
            }
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);

        let is_rest = self.ultradian_state == TimerState::Rest || self.ultradian_state == TimerState::PausedRest;

        if self.ultradian_state == TimerState::Idle {
            self.ultradian_remaining = self.work_dur();
        }

        if self.tracker.is_tracking {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
        if is_rest {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let frame_style = if is_rest {
            egui::Frame::default().fill(egui::Color32::BLACK)
        } else {
            egui::Frame::default().fill(ctx.style().visuals.panel_fill).inner_margin(16.0)
        };

        if !is_rest {
            egui::TopBottomPanel::top("top_panel").frame(egui::Frame::default().fill(egui::Color32::from_rgb(25, 30, 35)).inner_margin(8.0)).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let lang = self.tracker.data.language;
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
                AppView::Dashboard => {
                    self.tracker.ui_dashboard(ctx, ui, &self.tracker.data.projects);
                }
                AppView::Settings => {
                    if self.tracker.ui_settings(ctx, ui, &self.screen_tools, screen::is_wayland()) {
                        self.ultradian_state = TimerState::Idle;
                        self.ultradian_remaining = self.work_dur();
                        self.ultradian_start = Instant::now();
                        if self.tracker.data.screen_dim_during_rest {
                            crate::screen::restore_screen();
                        }
                        if self.tracker.data.screen_lock_during_rest {
                            crate::screen::unlock_screen();
                        }
                        exit_rest_viewport(ctx);
                    }
                }
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
        let has_focus = ctx.memory(|mem| mem.focused().is_some());
        if !has_focus {
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.ultradian_toggle_pause(ctx);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::R)) {
                self.ultradian_restart_phase();
            }
            if is_rest && ctx.input(|i| i.key_pressed(egui::Key::S)) {
                self.skip_rest(ctx);
            }
        }

        let secs = self.ultradian_remaining.as_secs();
        let display = format!("{:02}:{:02}", secs / 60, secs % 60);

        let total_secs = match self.ultradian_state {
            TimerState::Work | TimerState::PausedWork => self.work_dur().as_secs(),
            TimerState::Rest | TimerState::PausedRest => self.rest_dur().as_secs(),
            TimerState::Idle => self.work_dur().as_secs(),
        };
        let elapsed_secs = total_secs.saturating_sub(secs);
        let progress = if total_secs > 0 { elapsed_secs as f32 / total_secs as f32 } else { 0.0 };

        let today_secs = self.today_total_secs();
        let today_hours = today_secs / 3600;
        let today_mins = (today_secs % 3600) / 60;

        ui.centered_and_justified(|ui| {
            if is_rest {
                let breath_elapsed = self.breath_start.elapsed().as_secs_f32();
                let breath_phase = (breath_elapsed * 0.15 * std::f32::consts::PI).sin();
                let viewport_min = ui.available_width().min(ui.available_height());
                let base_radius = (viewport_min * 0.10).max(40.0);
                let breath_radius = base_radius + breath_phase * (base_radius * 0.5);

                let breathe_text = if breath_phase >= 0.0 {
                    crate::i18n::t(&lang, "breathe_in")
                } else {
                    crate::i18n::t(&lang, "breathe_out")
                };

                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.12);

                    // Breathing circle
                    let circle_size = breath_radius * 2.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(circle_size, circle_size),
                        egui::Sense::hover(),
                    );
                    let center = rect.center();
                    ui.painter().circle(
                        center,
                        breath_radius,
                        egui::Color32::from_rgba_premultiplied(0, 180, 180, 80),
                        egui::Stroke::new(2.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 60)),
                    );

                    ui.add_space(30.0);

                    // Breathing text
                    ui.label(
                        egui::RichText::new(breathe_text)
                            .color(egui::Color32::from_rgba_premultiplied(255, 255, 255, 220))
                            .size(24.0),
                    );

                    ui.add_space(20.0);

                    // Countdown timer
                    ui.label(
                        egui::RichText::new(display)
                            .color(egui::Color32::from_rgba_premultiplied(255, 255, 255, 230))
                            .size(60.0)
                            .strong(),
                    );

                    ui.add_space(15.0);

                    // Rest message
                    ui.label(
                        egui::RichText::new(crate::i18n::t(&lang, "rest_message"))
                            .color(egui::Color32::from_rgba_premultiplied(255, 255, 255, 180))
                            .size(18.0)
                            .italics(),
                    );

                    if self.ultradian_state == TimerState::PausedRest {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(crate::i18n::t(&lang, "ultradian_paused"))
                                .color(egui::Color32::from_rgba_premultiplied(255, 255, 255, 200))
                                .size(20.0),
                        );
                    }

                    ui.add_space(30.0);

                    // Skip rest button
                    if ui.add_sized([220.0, 44.0], egui::Button::new(
                        egui::RichText::new(crate::i18n::t(&lang, "skip_rest"))
                            .color(egui::Color32::LIGHT_GRAY)
                            .size(18.0),
                    )).clicked() {
                        self.skip_rest(ctx);
                    }
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("[S] {}", crate::i18n::t(&lang, "skip_rest_shortcut")))
                            .color(egui::Color32::from_gray(140))
                            .size(14.0),
                    );
                });
            } else {
                let (status, color) = match self.ultradian_state {
                    TimerState::Idle => (crate::i18n::t(&lang, "ultradian_idle"), egui::Color32::CYAN),
                    TimerState::Work => (crate::i18n::t(&lang, "ultradian_work"), egui::Color32::GREEN),
                    TimerState::PausedWork => (crate::i18n::t(&lang, "ultradian_paused"), egui::Color32::YELLOW),
                    _ => ("", egui::Color32::WHITE),
                };

                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 160.0);

                    ui.label(egui::RichText::new(status).color(color).size(30.0));
                    ui.add_space(10.0);

                    ui.label(egui::RichText::new(format!("{} {}", crate::i18n::t(&lang, "cycle_label"), self.cycle_count + 1)).color(egui::Color32::GRAY).size(16.0));
                    ui.add_space(10.0);

                    ui.label(egui::RichText::new(display).size(120.0).strong());
                    ui.add_space(20.0);

                    let bar_width = ui.available_width() * 0.6;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(bar_width, 8.0), egui::Sense::hover());
                    let bg_color = egui::Color32::from_gray(40);
                    let fg_color = if self.ultradian_state == TimerState::Work { egui::Color32::GREEN } else { egui::Color32::BLUE };
                    ui.painter().rect_filled(rect, 4.0, bg_color);
                    let fill_width = rect.width() * progress;
                    if fill_width > 0.0 {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(rect.min, egui::vec2(fill_width, rect.height())),
                            4.0,
                            fg_color,
                        );
                    }

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(format!("{}: {:02}h {:02}m", crate::i18n::t(&lang, "today_total"), today_hours, today_mins)).color(egui::Color32::GRAY).size(16.0));
                    ui.add_space(20.0);

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
    crate::screen::install_signal_handlers();

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
