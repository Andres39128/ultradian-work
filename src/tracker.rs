use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use eframe::egui;
use crate::screen::ScreenTools;

pub use crate::models::*;

/// Rebuilds in-memory state from a persisted in-progress session.
/// If it was tracking, folds the time elapsed since the persisted start
/// into the accumulated seconds, so the session keeps counting while the
/// app is closed.
fn restored_session_state(active: &ActiveSession, now_unix: u64) -> (bool, u64) {
    if active.is_tracking && active.start_unix > 0 {
        let gap = now_unix.saturating_sub(active.start_unix);
        (true, active.elapsed_secs + gap)
    } else {
        (false, active.elapsed_secs)
    }
}

#[derive(Default)]
pub struct TimeTrackerState {
    pub data: TrackerData,
    pub active_project_id: Option<String>,
    pub active_session_name: String,
    pub active_parent_session_id: Option<String>,

    pub is_tracking: bool,
    pub current_session_start: Option<Instant>,
    pub current_session_elapsed: u64,

    pub new_project_name: String,
    pub new_project_error: Option<String>,

    pub export_message: Option<String>,
    pub export_message_time: Option<Instant>,
    pub editing_session_id: Option<(String, String)>,
    pub editing_session_name: String,
    pub deleting_session_id: Option<(String, String)>,
    pub deleting_project_id: Option<String>,
    pub session_search_query: String,

    pub new_task_project_input: String,
    pub new_task_name: String,
    pub new_task_description: String,
    pub new_task_priority: Priority,
    pub new_task_tags: String,
    pub new_task_deadline: String,
    pub new_task_error: Option<String>,
    pub hide_completed_tasks: bool,
}

impl TimeTrackerState {
    fn get_data_path() -> PathBuf {
        if let Ok(path) = std::env::var("ULTRADIANT_DATA_PATH") {
            return PathBuf::from(path);
        }
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "DevPersonal", "UltradianTimer") {
            let dir = proj_dirs.data_dir();
            fs::create_dir_all(dir).unwrap_or_default();
            dir.join("tracker_data.json")
        } else {
            PathBuf::from("tracker_data.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::get_data_path();
        let data = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(data) => data,
                        Err(e) => {
                            eprintln!("Error parseando datos del tracker (archivo preservado): {}", e);
                            TrackerData::default()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error leyendo archivo de datos: {}", e);
                    TrackerData::default()
                }
            }
        } else {
            TrackerData::default()
        };

        let active = data.active_session.as_ref().filter(|a| {
            data.projects.iter().any(|p| p.id == a.project_id)
        });
        let (active_project_id, active_session_name, active_parent_session_id, is_tracking, current_session_start, current_session_elapsed) = match active {
            Some(a) => {
                let (is_tracking, current_session_elapsed) = restored_session_state(a, chrono::Local::now().timestamp() as u64);
                let current_session_start = if is_tracking { Some(Instant::now()) } else { None };
                (
                    Some(a.project_id.clone()),
                    a.session_name.clone(),
                    a.parent_session_id.clone(),
                    is_tracking,
                    current_session_start,
                    current_session_elapsed,
                )
            }
            None => (None, String::new(), None, false, None, 0),
        };

        Self {
            data,
            active_project_id,
            active_session_name,
            active_parent_session_id,
            is_tracking,
            current_session_start,
            current_session_elapsed,
            ..Default::default()
        }
    }

    /// Copies the in-memory session state into `data.active_session` so the
    /// next `save()` (and any future `load()`) sees it.
    fn sync_active_session(&mut self) {
        let has_state = self.active_project_id.is_some()
            || !self.active_session_name.is_empty()
            || self.active_parent_session_id.is_some()
            || self.is_tracking
            || self.current_session_elapsed > 0;
        self.data.active_session = if has_state {
            Some(ActiveSession {
                project_id: self.active_project_id.clone().unwrap_or_default(),
                session_name: self.active_session_name.clone(),
                parent_session_id: self.active_parent_session_id.clone(),
                is_tracking: self.is_tracking,
                start_unix: if self.is_tracking { chrono::Local::now().timestamp() as u64 } else { 0 },
                elapsed_secs: self.current_session_elapsed,
            })
        } else {
            None
        };
    }

    fn session_display_secs(&self) -> u64 {
        let mut secs = self.current_session_elapsed;
        if let Some(start) = self.current_session_start {
            secs += start.elapsed().as_secs();
        }
        secs
    }

    pub fn save(&mut self) {
        self.sync_active_session();
        let path = Self::get_data_path();
        match serde_json::to_string_pretty(&self.data) {
            Ok(content) => {
                let tmp_path = path.with_extension("json.tmp");
                match fs::write(&tmp_path, &content) {
                    Ok(()) => {
                        if let Err(e) = fs::rename(&tmp_path, &path) {
                            eprintln!("Error renombrando archivo temporal: {}", e);
                            let _ = fs::remove_file(&tmp_path);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error escribiendo archivo temporal: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error serializando datos del tracker: {}", e);
            }
        }
    }

    fn add_project(&mut self) {
        self.new_project_error = None;
        if self.new_project_name.trim().is_empty() {
            self.new_project_error = Some(crate::i18n::t(&self.data.language, "error_empty_name").to_string());
            return;
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.data.projects.push(Project {
            id: id.clone(),
            name: self.new_project_name.clone(),
            sessions: Vec::new(),
        });
        self.active_project_id = Some(id);
        self.new_project_name.clear();
        self.save();
    }

    pub fn toggle_tracking(&mut self, ctx: &egui::Context) {
        if self.is_tracking {
            if let Some(start) = self.current_session_start {
                self.current_session_elapsed += start.elapsed().as_secs();
            }
            self.current_session_start = None;
            self.is_tracking = false;
            self.save();
        } else {
            self.current_session_start = Some(Instant::now());
            self.is_tracking = true;
            self.save();
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn finish_session(&mut self) {
        let Some(proj_id) = &self.active_project_id else { return };
        let Some(proj) = self.data.projects.iter_mut().find(|p| &p.id == proj_id) else { return };

        if self.is_tracking
            && let Some(start) = self.current_session_start {
            self.current_session_elapsed += start.elapsed().as_secs();
        }

        let end = chrono::Local::now().timestamp() as u64;
        let start = if self.current_session_elapsed > 0 {
            end.saturating_sub(self.current_session_elapsed)
        } else {
            end
        };

        let name = if self.active_session_name.trim().is_empty() {
            let lang = self.data.language;
            format!("{} {}", crate::i18n::t(&lang, "session_label"), proj.sessions.len() + 1)
        } else {
            self.active_session_name.clone()
        };

        if let Some(parent_id) = &self.active_parent_session_id {
            if let Some(parent_session) = proj.sessions.iter_mut().find(|s| &s.id == parent_id) {
                parent_session.sub_sessions.push(SubSession {
                    start_time: start,
                    end_time: end,
                });
            }
        } else {
            proj.sessions.push(Session {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                start_time: start,
                end_time: end,
                sub_sessions: Vec::new(),
            });
        }

        self.is_tracking = false;
        self.current_session_start = None;
        self.current_session_elapsed = 0;
        self.active_session_name.clear();
        self.active_parent_session_id = None;
        self.save();
    }

    fn export_project_to_file(&mut self, proj: &Project, file_path: &std::path::Path) {
        let lang = self.data.language;
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        {
            let _ = worksheet.write_string(0, 0, crate::i18n::t(&lang, "session_history"));
            let _ = worksheet.write_string(0, 1, crate::i18n::t(&lang, "total_duration"));
            let _ = worksheet.write_string(0, 2, crate::i18n::t(&lang, "date_unix"));

            for (row, session) in (1..).zip(proj.sessions.iter()) {
                let total_secs = session.total_duration();

                let _ = worksheet.write_string(row, 0, &session.name);
                let _ = worksheet.write_number(row, 1, total_secs as f64 / 60.0);
                let _ = worksheet.write_number(row, 2, session.start_time as f64);
            }

            if workbook.save(file_path).is_ok() {
                self.export_message = Some(format!("{} {}", crate::i18n::t(&lang, "exported_to"), file_path.display()));
                self.export_message_time = Some(Instant::now());
            } else {
                self.export_message = Some(crate::i18n::t(&lang, "error_export").to_string());
                self.export_message_time = Some(Instant::now());
            }
        }
    }

    fn show_export_message(&mut self, ui: &mut egui::Ui) {
        if let Some(msg) = &self.export_message
            && let Some(time) = self.export_message_time {
            if time.elapsed().as_secs() < 5 {
                ui.label(egui::RichText::new(msg).color(egui::Color32::GREEN));
            } else {
                self.export_message = None;
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.data.language;
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(crate::i18n::t(&lang, "new_project"));
            ui.text_edit_singleline(&mut self.new_project_name);
            if ui.button(crate::i18n::t(&lang, "create")).clicked() {
                self.add_project();
            }
        });

        if let Some(ref err) = self.new_project_error {
            ui.label(egui::RichText::new(err).color(egui::Color32::RED).size(12.0));
        }

        ui.separator();

        let available_height = ui.available_height();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_min_height(available_height);
                ui.set_width(200.0);
                ui.heading(crate::i18n::t(&lang, "your_projects"));
                for proj in &self.data.projects {
                    ui.horizontal(|ui| {
                        let is_selected = self.active_project_id.as_deref() == Some(&proj.id);
                        if ui.selectable_label(is_selected, &proj.name).clicked()
                            && !self.is_tracking {
                            self.active_project_id = Some(proj.id.clone());
                        }
                        if ui.button("🗑").on_hover_text(crate::i18n::t(&lang, "delete_tooltip")).clicked()
                            && !self.is_tracking {
                            self.deleting_project_id = Some(proj.id.clone());
                        }
                    });
                }
                if let Some(proj_id) = &self.deleting_project_id {
                    let mut confirm_delete = false;
                    let mut cancel_delete = false;
                    egui::Window::new(crate::i18n::t(&lang, "delete_confirm_title"))
                        .collapsible(false)
                        .resizable(false)
                        .show(ui.ctx(), |ui| {
                            ui.label(crate::i18n::t(&lang, "delete_project_confirm"));
                            ui.horizontal(|ui| {
                                if ui.button(crate::i18n::t(&lang, "yes_delete")).clicked() {
                                    confirm_delete = true;
                                }
                                if ui.button(crate::i18n::t(&lang, "cancel")).clicked() {
                                    cancel_delete = true;
                                }
                            });
                        });
                    if confirm_delete {
                        self.data.projects.retain(|p| p.id != *proj_id);
                        if self.active_project_id.as_deref() == Some(proj_id) {
                            self.active_project_id = None;
                            self.active_session_name.clear();
                            self.active_parent_session_id = None;
                            self.current_session_start = None;
                            self.current_session_elapsed = 0;
                            self.is_tracking = false;
                        }
                        self.save();
                    }
                    if confirm_delete || cancel_delete {
                        self.deleting_project_id = None;
                    }
                }
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.set_min_height(available_height);
                ui.set_min_width(ui.available_width());
                if let Some(proj_id) = self.active_project_id.clone() {
                    let proj_name = self.data.projects.iter().find(|p| p.id == proj_id).map(|p| p.name.clone()).unwrap_or_default();

                    ui.heading(format!("{} {}", crate::i18n::t(&lang, "project_label"), proj_name));
                    ui.add_space(10.0);

                    if ui.button(crate::i18n::t(&lang, "export_excel")).clicked()
                        && let Some(proj) = self.data.projects.iter().find(|p| p.id == proj_id).cloned()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("Excel Workbook", &["xlsx"])
                            .set_file_name(format!("{}_export.xlsx", proj.name))
                            .save_file() {
                        self.export_project_to_file(&proj, &path);
                    }

                    self.show_export_message(ui);

                    ui.add_space(20.0);

                    ui.group(|ui| {
                        if self.active_parent_session_id.is_some() {
                            ui.heading(format!("{} {}", crate::i18n::t(&lang, "continuing_session"), self.active_session_name));
                        } else {
                            ui.heading(crate::i18n::t(&lang, "new_work_session"));
                            ui.horizontal(|ui| {
                                ui.label(crate::i18n::t(&lang, "name"));
                                ui.add_enabled(
                                    !self.is_tracking,
                                    egui::TextEdit::singleline(&mut self.active_session_name),
                                );
                            });
                        }

                        let current_secs = self.session_display_secs();
                        let display = format!("{:02}:{:02}:{:02}", current_secs / 3600, (current_secs % 3600) / 60, current_secs % 60);

                        ui.label(egui::RichText::new(display).size(30.0).strong().color(if self.is_tracking { egui::Color32::GREEN } else { egui::Color32::WHITE }));

                        ui.horizontal(|ui| {
                            let btn_text = if self.is_tracking { crate::i18n::t(&lang, "pause") } else { crate::i18n::t(&lang, "start") };
                            if ui.button(btn_text).clicked() {
                                self.toggle_tracking(ctx);
                            }

                            if (current_secs > 0 || self.active_parent_session_id.is_some())
                                && ui.button(crate::i18n::t(&lang, "finish_save")).clicked() {
                                self.finish_session();
                            }
                        });
                    });

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.heading(crate::i18n::t(&lang, "session_history"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.text_edit_singleline(&mut self.session_search_query);
                            if !self.session_search_query.is_empty()
                                && ui.button(crate::i18n::t(&lang, "clear_search")).clicked() {
                                self.session_search_query.clear();
                            }
                            ui.label("🔍");
                        });
                    });

                    let mut session_to_delete = None;
                    let mut session_to_save = None;
                    let mut continue_session = None;

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                        if let Some(proj) = self.data.projects.iter().find(|p| p.id == proj_id) {
                            for session in &proj.sessions {
                                if !self.session_search_query.is_empty() && !session.name.to_lowercase().contains(&self.session_search_query.to_lowercase()) {
                                    continue;
                                }
                                ui.push_id(&session.id, |ui| {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            if self.editing_session_id.as_ref() == Some(&(proj_id.clone(), session.id.clone())) {
                                                ui.text_edit_singleline(&mut self.editing_session_name);
                                                if ui.button(crate::i18n::t(&lang, "save")).clicked() {
                                                    session_to_save = Some((session.id.clone(), self.editing_session_name.clone()));
                                                    self.editing_session_id = None;
                                                }
                                                if ui.button(crate::i18n::t(&lang, "cancel")).clicked() {
                                                    self.editing_session_id = None;
                                                }
                                            } else {
                                                ui.label(egui::RichText::new(&session.name).strong());

                                                if ui.button(crate::i18n::t(&lang, "continue")).clicked() {
                                                    continue_session = Some((proj_id.clone(), session.id.clone(), session.name.clone()));
                                                }

                                                if ui.button("✏").on_hover_text(crate::i18n::t(&lang, "edit_tooltip")).clicked() {
                                                    self.editing_session_id = Some((proj_id.clone(), session.id.clone()));
                                                    self.editing_session_name = session.name.clone();
                                                }
                                                if ui.button("🗑").on_hover_text(crate::i18n::t(&lang, "delete_tooltip")).clicked() {
                                                    self.deleting_session_id = Some((proj_id.clone(), session.id.clone()));
                                                }
                                            }
                                        });

                                        let total_secs = session.total_duration();

                                        let time_str = format!("{:02}:{:02}:{:02}", total_secs / 3600, (total_secs % 3600) / 60, total_secs % 60);
                                        let date_str = chrono::DateTime::from_timestamp(session.start_time as i64, 0)
                                            .map(|dt| dt.with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M").to_string())
                                            .unwrap_or_default();
                                        ui.label(format!("{} {}", crate::i18n::t(&lang, "total_duration"), time_str));
                                        if !date_str.is_empty() {
                                            ui.label(egui::RichText::new(date_str).size(12.0).color(egui::Color32::GRAY));
                                        }

                                        if !session.sub_sessions.is_empty() {
                                            ui.collapsing(format!("{} {}", session.sub_sessions.len(), crate::i18n::t(&lang, "sub_sessions")), |ui| {
                                                for sub in &session.sub_sessions {
                                                    let sub_secs = sub.end_time.saturating_sub(sub.start_time);
                                                    let sub_time_str = format!("{:02}:{:02}:{:02}", sub_secs / 3600, (sub_secs % 3600) / 60, sub_secs % 60);
                                                    ui.label(format!("- {}", sub_time_str));
                                                }
                                            });
                                        }
                                    });
                                });
                            }
                        }
                    });

                    if let Some((sid, new_name)) = session_to_save
                        && let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == proj_id)
                        && let Some(sess) = proj.sessions.iter_mut().find(|s| s.id == sid) {
                        sess.name = new_name;
                        self.save();
                    }

                    if let Some((_pid, sid, sname)) = continue_session
                        && !self.is_tracking {
                        self.active_parent_session_id = Some(sid);
                        self.active_session_name = sname;
                        self.save();
                    }

                    if let Some((del_pid, del_sid)) = &self.deleting_session_id {
                        let mut confirm_delete = false;
                        let mut cancel_delete = false;

                        egui::Window::new(crate::i18n::t(&lang, "delete_confirm_title"))
                            .collapsible(false)
                            .resizable(false)
                            .show(ui.ctx(), |ui| {
                                ui.label(crate::i18n::t(&lang, "delete_session_confirm"));
                                ui.horizontal(|ui| {
                                    if ui.button(crate::i18n::t(&lang, "yes_delete")).clicked() {
                                        confirm_delete = true;
                                    }
                                    if ui.button(crate::i18n::t(&lang, "cancel")).clicked() {
                                        cancel_delete = true;
                                    }
                                });
                            });

                        if confirm_delete {
                            let deleted_sid = del_sid.clone();
                            if let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == *del_pid) {
                                proj.sessions.retain(|s| s.id != *del_sid);
                            }
                            if self.active_parent_session_id.as_deref() == Some(del_sid) {
                                self.active_parent_session_id = None;
                                self.active_session_name.clear();
                            }
                            self.save();
                            session_to_delete = Some(deleted_sid);
                        }
                        if confirm_delete || cancel_delete {
                            session_to_delete = Some("handled".to_string());
                        }
                    }

                    if session_to_delete.is_some() {
                        self.deleting_session_id = None;
                    }
                } else {
                    ui.label(crate::i18n::t(&lang, "empty_state_tracker"));
                }
            });
        });
    }

    pub fn ui_tasks(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
        let lang = self.data.language;
        let mut redirect_to_tracker = false;

        ui.horizontal(|ui| {
            if ui.button(crate::i18n::t(&lang, "export_tasks")).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Excel Workbook", &["xlsx"])
                    .set_file_name("pendientes_export.xlsx")
                    .save_file() {
                self.export_tasks_to_file(&path);
            }
            if ui.button(crate::i18n::t(&lang, "import_tasks")).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Excel Workbook", &["xlsx"])
                    .pick_file() {
                self.import_tasks_from_file(&path);
            }
        });

        self.show_export_message(ui);

        ui.separator();

        ui.group(|ui| {
            ui.heading(crate::i18n::t(&lang, "new_task"));
            ui.horizontal(|ui| {
                ui.label(crate::i18n::t(&lang, "name"));
                ui.text_edit_singleline(&mut self.new_task_name);
            });
            ui.horizontal(|ui| {
                ui.label(crate::i18n::t(&lang, "project_optional"));
                egui::ComboBox::from_id_salt("proj_select")
                    .selected_text(if self.new_task_project_input.is_empty() { crate::i18n::t(&lang, "none") } else { &self.new_task_project_input })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.new_task_project_input, String::new(), crate::i18n::t(&lang, "none"));
                        for proj in &self.data.projects {
                            ui.selectable_value(&mut self.new_task_project_input, proj.id.clone(), &proj.name);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(crate::i18n::t(&lang, "priority"));
                ui.radio_value(&mut self.new_task_priority, Priority::Alta, crate::i18n::t(&lang, "high"));
                ui.radio_value(&mut self.new_task_priority, Priority::Media, crate::i18n::t(&lang, "medium"));
                ui.radio_value(&mut self.new_task_priority, Priority::Baja, crate::i18n::t(&lang, "low"));
            });
            ui.horizontal(|ui| {
                ui.label(crate::i18n::t(&lang, "tags_comma"));
                ui.text_edit_singleline(&mut self.new_task_tags);
            });
            ui.horizontal(|ui| {
                ui.label(crate::i18n::t(&lang, "deadline"));
                ui.text_edit_singleline(&mut self.new_task_deadline);
            });
            ui.horizontal(|ui| {
                ui.label(crate::i18n::t(&lang, "description"));
                ui.text_edit_multiline(&mut self.new_task_description);
            });
            if ui.button(crate::i18n::t(&lang, "create_task")).clicked() {
                self.create_task();
            }
            if let Some(ref err) = self.new_task_error {
                ui.label(egui::RichText::new(err).color(egui::Color32::RED).size(12.0));
            }
        });

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.heading(crate::i18n::t(&lang, "todo_list"));
            ui.checkbox(&mut self.hide_completed_tasks, crate::i18n::t(&lang, "hide_completed"));
        });

        let task_ids: Vec<String> = self.data.tasks.iter()
            .filter(|t| !self.hide_completed_tasks || !t.completed)
            .map(|t| t.id.clone())
            .collect();

        if task_ids.is_empty() {
            ui.label(crate::i18n::t(&lang, "empty_state_tasks"));
            return redirect_to_tracker;
        }

        let mut to_delete: Option<String> = None;
        let mut save_needed = false;
        let mut start_task_session: Option<(String, String)> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
            for task_id in &task_ids {
                let Some(task) = self.data.tasks.iter_mut().find(|t| &t.id == task_id) else { continue };
                let task_id_clone = task.id.clone();
                let task_name_clone = task.name.clone();
                let task_proj_clone = task.project.clone();
                let task_completed = task.completed;
                let task_description = task.description.clone();
                let task_priority = task.priority.clone();
                let task_tags = task.tags.clone();
                let task_deadline = task.deadline.clone();

                let mut new_completed = task_completed;
                let mut clicked_delete = false;
                let mut clicked_work = false;

                ui.push_id(&task_id_clone, |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut new_completed, &task_name_clone).changed() {
                                save_needed = true;
                            }

                            if ui.button("🗑").on_hover_text(crate::i18n::t(&lang, "delete_tooltip")).clicked() {
                                clicked_delete = true;
                            }

                            if !new_completed
                                && task_proj_clone.is_some()
                                && ui.button(crate::i18n::t(&lang, "work_on_this")).clicked() {
                                clicked_work = true;
                            }
                        });

                        if !task_description.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(25.0);
                                ui.label(&task_description);
                            });
                        }
                        ui.horizontal(|ui| {
                            let color = match task_priority { Priority::Alta => egui::Color32::RED, Priority::Media => egui::Color32::YELLOW, Priority::Baja => egui::Color32::GREEN };
                            ui.label(egui::RichText::new(match task_priority { Priority::Alta => crate::i18n::t(&lang, "high"), Priority::Media => crate::i18n::t(&lang, "medium"), Priority::Baja => crate::i18n::t(&lang, "low") }).color(color));
                            if !task_tags.is_empty() { ui.label(format!("🏷 {}", task_tags)); }
                            if !task_deadline.is_empty() { ui.label(format!("📅 {}", task_deadline)); }
                        });
                    });
                });

                if new_completed != task_completed
                    && let Some(t) = self.data.tasks.iter_mut().find(|t| t.id == task_id_clone) {
                    t.completed = new_completed;
                }
                if clicked_delete {
                    to_delete = Some(task_id_clone);
                }
                if clicked_work && !self.is_tracking
                    && let Some(proj_id) = task_proj_clone {
                    start_task_session = Some((proj_id, task_name_clone));
                }
            }
        });

        if let Some(id) = to_delete {
            self.data.tasks.retain(|t| t.id != id);
            self.save();
        } else if save_needed {
            self.save();
        }

        if let Some((proj_id, task_name)) = start_task_session {
            self.active_project_id = Some(proj_id);
            self.active_session_name = format!("{} {}", crate::i18n::t(&lang, "task_session_prefix"), task_name);
            self.active_parent_session_id = None;
            self.current_session_start = Some(Instant::now());
            self.is_tracking = true;
            self.save();
            redirect_to_tracker = true;
        }

        redirect_to_tracker
    }

    fn create_task(&mut self) {
        self.new_task_error = None;
        if self.new_task_name.trim().is_empty() {
            self.new_task_error = Some(crate::i18n::t(&self.data.language, "error_empty_name").to_string());
            return;
        }
        self.data.tasks.push(Task {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.new_task_name.clone(),
            description: self.new_task_description.clone(),
            completed: false,
            project: if self.new_task_project_input.is_empty() { None } else { Some(self.new_task_project_input.clone()) },
            priority: self.new_task_priority.clone(),
            tags: self.new_task_tags.clone(),
            deadline: self.new_task_deadline.clone(),
        });
        self.new_task_name.clear();
        self.new_task_description.clear();
        self.new_task_tags.clear();
        self.new_task_deadline.clear();
        self.save();
    }

    fn import_tasks_from_file(&mut self, path: &std::path::Path) {
        use calamine::{Reader, open_workbook_auto};
        let lang = self.data.language;
        let Ok(mut workbook) = open_workbook_auto(path) else { 
            self.export_message = Some(crate::i18n::t(&lang, "error_open_file").to_string());
            self.export_message_time = Some(Instant::now());
            return;
        };
        let Ok(range) = workbook.worksheet_range("Pendientes") else { 
            self.export_message = Some(crate::i18n::t(&lang, "sheet_not_found").to_string());
            self.export_message_time = Some(Instant::now());
            return;
        };

        let mut imported = 0;
        for (i, row) in range.rows().enumerate() {
            if i == 0 { continue; }
            if row.is_empty() || row.iter().all(|c| c.to_string().trim().is_empty()) { continue; }

            let name = row.first().map(|c| c.to_string().trim().to_string()).unwrap_or_default();
            if name.is_empty() { continue; }

            let description = row.get(1).map(|c| c.to_string().trim().to_string()).unwrap_or_default();

            let proj_name = row.get(2).map(|c| c.to_string().trim().to_string()).unwrap_or_default();
            let project = if proj_name.is_empty() {
                None
            } else {
                if let Some(p) = self.data.projects.iter().find(|p| p.name.trim().to_lowercase() == proj_name.to_lowercase()) {
                    Some(p.id.clone())
                } else {
                    let id = uuid::Uuid::new_v4().to_string();
                    self.data.projects.push(Project {
                        id: id.clone(),
                        name: proj_name.clone(),
                        sessions: Vec::new()
                    });
                    Some(id)
                }
            };

            if self.data.tasks.iter().any(|t| t.name.trim().to_lowercase() == name.to_lowercase() && t.project == project) {
                continue;
            }

            let completed_str = row.get(3).map(|c| c.to_string().trim().to_lowercase()).unwrap_or_default();
            let completed = matches!(completed_str.as_str(), "true" | "si" | "1" | "yes");
            let priority_str = row.get(4).map(|c| c.to_string().trim().to_lowercase()).unwrap_or_default();
            let priority = match priority_str.as_str() {
                "alta" | "high" => Priority::Alta,
                "media" | "medium" => Priority::Media,
                "baja" | "low" => Priority::Baja,
                _ => Priority::Media,
            };
            let tags = row.get(5).map(|c| c.to_string().trim().to_string()).unwrap_or_default();
            let deadline = row.get(6).map(|c| c.to_string().trim().to_string()).unwrap_or_default();

            self.data.tasks.push(Task {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                description,
                project,
                completed,
                priority,
                tags,
                deadline,
            });
            imported += 1;
        }
        self.save();
        self.export_message = Some(format!("{} {}", imported, crate::i18n::t(&lang, "imported_tasks")));
        self.export_message_time = Some(Instant::now());
    }

    fn export_tasks_to_file(&mut self, path: &std::path::Path) {
        let lang = self.data.language;
        let mut workbook = rust_xlsxwriter::Workbook::new();
        if let Ok(worksheet) = workbook.add_worksheet().set_name("Pendientes") {
            let _ = worksheet.write_string(0, 0, crate::i18n::t(&lang, "excel_header_name"));
            let _ = worksheet.write_string(0, 1, crate::i18n::t(&lang, "excel_header_description"));
            let _ = worksheet.write_string(0, 2, crate::i18n::t(&lang, "excel_header_project"));
            let _ = worksheet.write_string(0, 3, crate::i18n::t(&lang, "excel_header_completed"));
            let _ = worksheet.write_string(0, 4, crate::i18n::t(&lang, "excel_header_priority"));
            let _ = worksheet.write_string(0, 5, crate::i18n::t(&lang, "excel_header_tags"));
            let _ = worksheet.write_string(0, 6, crate::i18n::t(&lang, "excel_header_deadline"));

            for (i, task) in self.data.tasks.iter().enumerate() {
                let row = (i + 1) as u32;
                let _ = worksheet.write_string(row, 0, &task.name);
                let _ = worksheet.write_string(row, 1, &task.description);

                let proj_name = if let Some(pid) = &task.project {
                    self.data.projects.iter().find(|p| p.id == *pid).map(|p| p.name.clone()).unwrap_or_default()
                } else {
                    String::new()
                };
                let _ = worksheet.write_string(row, 2, &proj_name);

                let _ = worksheet.write_string(row, 3, if task.completed { "true" } else { "false" });
                let prio_str = match task.priority { Priority::Alta => "Alta", Priority::Media => "Media", Priority::Baja => "Baja" };
                let _ = worksheet.write_string(row, 4, prio_str);
                let _ = worksheet.write_string(row, 5, &task.tags);
                let _ = worksheet.write_string(row, 6, &task.deadline);
            }

            if workbook.save(path).is_ok() {
                self.export_message = Some(format!("{} {}", crate::i18n::t(&lang, "exported_to"), path.display()));
                self.export_message_time = Some(Instant::now());
            } else {
                self.export_message = Some(crate::i18n::t(&lang, "error_export").to_string());
                self.export_message_time = Some(Instant::now());
            }
        }
    }

    pub fn ui_dashboard(&self, _ctx: &egui::Context, ui: &mut egui::Ui, projects: &[Project]) {
        let lang = self.data.language;
        ui.heading(crate::i18n::t(&lang, "dashboard_hours_project"));
        ui.add_space(20.0);

        let today_secs: u64 = projects.iter().map(|p| p.today_duration_secs()).sum();
        let total_secs: u64 = projects.iter().map(|p| p.total_duration()).sum();
        let today_hours = today_secs / 3600;
        let today_mins = (today_secs % 3600) / 60;
        let total_hours = total_secs / 3600;
        let total_mins = (total_secs % 3600) / 60;
        let total_sessions: usize = projects.iter().map(|p| p.sessions.len()).sum();

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new(crate::i18n::t(&lang, "today_total")).size(14.0).color(egui::Color32::GRAY));
                ui.label(egui::RichText::new(format!("{:02}h {:02}m", today_hours, today_mins)).size(24.0).strong().color(egui::Color32::GREEN));
            });
            ui.group(|ui| {
                ui.label(egui::RichText::new(crate::i18n::t(&lang, "total_all_time")).size(14.0).color(egui::Color32::GRAY));
                ui.label(egui::RichText::new(format!("{:02}h {:02}m", total_hours, total_mins)).size(24.0).strong().color(egui::Color32::BLUE));
            });
            ui.group(|ui| {
                ui.label(egui::RichText::new(crate::i18n::t(&lang, "total_sessions")).size(14.0).color(egui::Color32::GRAY));
                ui.label(egui::RichText::new(format!("{}", total_sessions)).size(24.0).strong().color(egui::Color32::YELLOW));
            });
        });
        ui.add_space(20.0);

        if projects.is_empty() {
            ui.label(crate::i18n::t(&lang, "empty_state_dashboard"));
            return;
        }

        let mut bars = Vec::new();
        for (i, proj) in projects.iter().enumerate() {
            let hours = proj.total_duration() as f64 / 3600.0;
            bars.push(egui_plot::Bar::new(i as f64, hours).name(proj.name.clone()));
        }

        let chart = egui_plot::BarChart::new(crate::i18n::t(&lang, "chart_projects"), bars);
        egui_plot::Plot::new("dashboard_plot")
            .allow_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .y_axis_label(crate::i18n::t(&lang, "hours_label"))
            .show(ui, |plot_ui| plot_ui.bar_chart(chart));
    }

    pub fn ui_settings(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, screen_tools: &ScreenTools, wayland: bool) -> bool {
        let lang = self.data.language;
        ui.heading(crate::i18n::t(&lang, "settings"));
        ui.add_space(20.0);

        let mut changed = false;
        let mut durations_changed = false;
        ui.horizontal(|ui| {
            ui.label(crate::i18n::t(&lang, "deep_work_minutes"));
            if ui.add(egui::Slider::new(&mut self.data.work_duration_mins, 1..=120)).changed() { changed = true; durations_changed = true; }
            ui.label(format!("{} min", self.data.work_duration_mins));
        });
        ui.horizontal(|ui| {
            ui.label(crate::i18n::t(&lang, "rest_minutes"));
            if ui.add(egui::Slider::new(&mut self.data.rest_duration_mins, 1..=60)).changed() { changed = true; durations_changed = true; }
            ui.label(format!("{} min", self.data.rest_duration_mins));
        });

        ui.add_space(20.0);
        ui.heading(crate::i18n::t(&lang, "settings_screen"));

        let has_dim = screen_tools.has_dim_support();
        let dim_response = ui.horizontal(|ui| {
            ui.add_enabled(has_dim, egui::Checkbox::new(&mut self.data.screen_dim_during_rest, crate::i18n::t(&lang, "screen_dim_during_rest")))
        });
        if dim_response.inner.changed() && has_dim { changed = true; }
        if !has_dim {
            ui.label(egui::RichText::new(format!("  ⚠ {}", crate::i18n::t(&lang, "screen_not_available"))).color(egui::Color32::from_rgb(200, 150, 50)).size(12.0));
        } else {
            let dim_tool = if screen_tools.has_brightnessctl() { "brightnessctl" } else { "xset" };
            ui.label(egui::RichText::new(format!("  {}  ({})", crate::i18n::t(&lang, "screen_available"), dim_tool)).color(egui::Color32::from_rgb(100, 180, 100)).size(12.0));
        }
        if wayland && !screen_tools.has_brightnessctl() {
            ui.label(egui::RichText::new(crate::i18n::t(&lang, "wayland_warning")).color(egui::Color32::from_rgb(220, 130, 50)).size(12.0));
        }

        let has_lock = screen_tools.has_lock();
        let lock_response = ui.horizontal(|ui| {
            ui.add_enabled(has_lock, egui::Checkbox::new(&mut self.data.screen_lock_during_rest, crate::i18n::t(&lang, "screen_lock_during_rest")))
        });
        if lock_response.inner.changed() && has_lock { changed = true; }
        if !has_lock {
            ui.label(egui::RichText::new(format!("  ⚠ {}", crate::i18n::t(&lang, "screen_not_available"))).color(egui::Color32::from_rgb(200, 150, 50)).size(12.0));
        } else {
            let lock_tool = if screen_tools.has_lock() { "loginctl" } else { "" };
            if !lock_tool.is_empty() {
                ui.label(egui::RichText::new(format!("  {}  ({})", crate::i18n::t(&lang, "screen_available"), lock_tool)).color(egui::Color32::from_rgb(100, 180, 100)).size(12.0));
            }
        }

        if changed {
            self.save();
        }

        durations_changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language;

    // Tests that mutate ULTRADIANT_DATA_PATH (a process-global env var) must
    // run one at a time so parallel tests don't overwrite each other's data path.
    static DATA_PATH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_project_total_duration_empty() {
        let proj = Project {
            id: "1".into(),
            name: "P1".into(),
            sessions: vec![],
        };
        assert_eq!(proj.total_duration(), 0);
    }

    #[test]
    fn test_project_total_duration_with_sessions_and_subs() {
        let proj = Project {
            id: "1".into(),
            name: "P1".into(),
            sessions: vec![
                Session {
                    id: "s1".into(),
                    name: "S1".into(),
                    start_time: 100,
                    end_time: 200,
                    sub_sessions: vec![
                        SubSession { start_time: 250, end_time: 300 },
                        SubSession { start_time: 350, end_time: 300 },
                    ],
                },
                Session {
                    id: "s2".into(),
                    name: "S2".into(),
                    start_time: 500,
                    end_time: 400,
                    sub_sessions: vec![],
                },
            ],
        };
        assert_eq!(proj.total_duration(), 150);
    }

    #[test]
    fn test_export_import_tasks() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let export_path = temp_dir.join("test_export.xlsx");
        let data_path = temp_dir.join("test_tracker_data.json");

        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&export_path);
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();
        state.data.tasks.clear();

        state.data.tasks.push(Task {
            id: "test-id-1".into(),
            name: "Test Task".into(),
            description: "Description".into(),
            project: None,
            completed: false,
            priority: Priority::Alta,
            tags: "tag1".into(),
            deadline: "2023-12-31".into(),
        });

        state.export_tasks_to_file(&export_path);
        assert!(export_path.exists());

        let mut new_state = TimeTrackerState::load();
        new_state.data.tasks.clear();

        new_state.import_tasks_from_file(&export_path);

        assert_eq!(new_state.data.tasks.len(), 1);
        let imported = &new_state.data.tasks[0];
        assert_eq!(imported.name, "Test Task");
        assert_eq!(imported.description, "Description");
        assert_eq!(imported.completed, false);

        let _ = std::fs::remove_file(&export_path);
    }

    #[test]
    fn test_import_priority_is_case_insensitive() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let xlsx_path = temp_dir.join("test_import_priority.xlsx");
        let data_path = temp_dir.join("test_tracker_data_priority.json");
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&xlsx_path);
        let _ = std::fs::remove_file(&data_path);

        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet().set_name("Pendientes").expect("create Pendientes worksheet");
        for (i, header) in ["Nombre", "Descripcion", "Proyecto", "Completado", "Prioridad", "Tags", "Fecha limite"].into_iter().enumerate() {
            let _ = worksheet.write_string(0, i as u16, header);
        }
        for (r, (name, priority)) in [("T Alta", "High"), ("T Media", "medium"), ("T Baja", "Baja"), ("T Typo", "Urgente")].into_iter().enumerate() {
            let row = (r + 1) as u32;
            let _ = worksheet.write_string(row, 0, name);
            let _ = worksheet.write_string(row, 3, "false");
            let _ = worksheet.write_string(row, 4, priority);
        }
        assert!(workbook.save(&xlsx_path).is_ok());

        let mut state = TimeTrackerState::load();
        state.data.tasks.clear();
        state.import_tasks_from_file(&xlsx_path);

        assert_eq!(state.data.tasks.len(), 4);
        let priority_of = |name: &str| state.data.tasks.iter().find(|t| t.name == name).unwrap().priority.clone();
        assert_eq!(priority_of("T Alta"), Priority::Alta);
        assert_eq!(priority_of("T Media"), Priority::Media);
        assert_eq!(priority_of("T Baja"), Priority::Baja);
        assert_eq!(priority_of("T Typo"), Priority::Media);

        let _ = std::fs::remove_file(&xlsx_path);
        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let mut data = TrackerData::default();
        data.schema_version = 1;
        data.projects.push(Project {
            id: "test-proj".into(),
            name: "Test Project".into(),
            sessions: vec![Session {
                id: "test-sess".into(),
                name: "Test Session".into(),
                start_time: 1000,
                end_time: 2000,
                sub_sessions: vec![],
            }],
        });
        data.tasks.push(Task {
            id: "test-task".into(),
            name: "Test Task".into(),
            description: "Test".into(),
            completed: false,
            project: Some("test-proj".into()),
            priority: Priority::Alta,
            tags: "test".into(),
            deadline: "2024-01-01".into(),
        });

        let content = serde_json::to_string_pretty(&data).unwrap();
        let loaded = serde_json::from_str::<TrackerData>(&content).unwrap();
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "Test Project");
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].name, "Test Task");
        assert_eq!(loaded.schema_version, 1);
    }

    #[test]
    fn test_corrupt_json_falls_back_to_default() {
        let result = serde_json::from_str::<TrackerData>("{ invalid json }}}");
        assert!(result.is_err());

        let fallback = result.unwrap_or_default();
        assert!(fallback.projects.is_empty());
        assert!(fallback.tasks.is_empty());
        assert_eq!(fallback.schema_version, 1);
    }

    #[test]
    fn test_schema_version_defaults_to_one() {
        let data = TrackerData::default();
        assert_eq!(data.schema_version, 1);
    }

    #[test]
    fn test_missing_fields_use_defaults() {
        let json = r#"{"language": "Es"}"#;
        let data: TrackerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.language, Language::Es);
        // Missing durations must match the Default impl (90/15), not 0.
        assert_eq!(data.work_duration_mins, 90);
        assert_eq!(data.rest_duration_mins, 15);
        assert!(data.projects.is_empty());
        assert!(data.tasks.is_empty());
        assert_eq!(data.schema_version, 0);

        // Explicit values (including 0) must be preserved.
        let json_explicit = r#"{"work_duration_mins": 0, "rest_duration_mins": 25}"#;
        let data: TrackerData = serde_json::from_str(json_explicit).unwrap();
        assert_eq!(data.work_duration_mins, 0);
        assert_eq!(data.rest_duration_mins, 25);
    }

    #[test]
    fn test_create_task_validates_name() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let data_path = std::env::temp_dir().join("test_tracker_data_create_task.json");
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();
        assert!(state.data.tasks.is_empty());

        // Empty name must be rejected by the real create_task().
        state.new_task_name.clear();
        state.create_task();
        assert!(state.new_task_error.is_some());
        assert!(state.data.tasks.is_empty());

        // Valid name creates the task, clears the form, and persists it.
        state.new_task_name = "Valid Task".into();
        state.new_task_priority = Priority::Alta;
        state.new_task_tags = "tag1".into();
        state.create_task();
        assert!(state.new_task_error.is_none());
        assert_eq!(state.data.tasks.len(), 1);
        assert_eq!(state.data.tasks[0].name, "Valid Task");
        assert_eq!(state.data.tasks[0].priority, Priority::Alta);
        assert_eq!(state.data.tasks[0].tags, "tag1");
        assert!(state.new_task_name.is_empty());

        // The real save() wrote the task to the injected data path.
        let reloaded = TimeTrackerState::load();
        assert_eq!(reloaded.data.tasks.len(), 1);
        assert_eq!(reloaded.data.tasks[0].name, "Valid Task");

        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn test_create_project_validates_name() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let data_path = std::env::temp_dir().join("test_tracker_data_create_project.json");
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();
        assert!(state.data.projects.is_empty());

        // Empty name must be rejected by the real add_project().
        state.new_project_name.clear();
        state.add_project();
        assert!(state.new_project_error.is_some());
        assert!(state.data.projects.is_empty());

        // Valid name creates the project, activates it, clears the form, and persists it.
        state.new_project_name = "Valid Project".into();
        state.add_project();
        assert!(state.new_project_error.is_none());
        assert_eq!(state.data.projects.len(), 1);
        assert_eq!(state.data.projects[0].name, "Valid Project");
        let project_id = state.data.projects[0].id.clone();
        assert_eq!(state.active_project_id, Some(project_id));
        assert!(state.new_project_name.is_empty());

        // The real save() wrote the project to the injected data path.
        let reloaded = TimeTrackerState::load();
        assert_eq!(reloaded.data.projects.len(), 1);
        assert_eq!(reloaded.data.projects[0].name, "Valid Project");

        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn test_screen_settings_backward_compatibility() {
        // Old JSON without screen fields must deserialize using serde(default) = false.
        let json = r#"{"language": "Es", "work_duration_mins": 90, "rest_duration_mins": 15}"#;
        let data: TrackerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.screen_dim_during_rest, false);
        assert_eq!(data.screen_lock_during_rest, false);

        // Explicit values must be preserved.
        let json_explicit = r#"{"screen_dim_during_rest": true, "screen_lock_during_rest": true}"#;
        let data_explicit: TrackerData = serde_json::from_str(json_explicit).unwrap();
        assert_eq!(data_explicit.screen_dim_during_rest, true);
        assert_eq!(data_explicit.screen_lock_during_rest, true);
    }

    #[test]
    fn test_atomic_write_pattern() {
        let temp_dir = std::env::temp_dir();
        let target = temp_dir.join(format!("atomic_test_{}.json", uuid::Uuid::new_v4()));
        let tmp = target.with_extension("json.tmp");

        let data = TrackerData::default();
        let content = serde_json::to_string_pretty(&data).unwrap();
        std::fs::write(&tmp, &content).unwrap();
        std::fs::rename(&tmp, &target).unwrap();

        assert!(target.exists());
        assert!(!tmp.exists());

        let loaded = serde_json::from_str::<TrackerData>(&std::fs::read_to_string(&target).unwrap()).unwrap();
        assert_eq!(loaded.schema_version, 1);

        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn test_sync_active_session_populates_data() {
        let mut state = TimeTrackerState {
            active_project_id: Some("p1".into()),
            active_session_name: "Sesión".into(),
            active_parent_session_id: Some("s1".into()),
            is_tracking: true,
            current_session_elapsed: 120,
            ..Default::default()
        };

        state.sync_active_session();

        let active = state.data.active_session.as_ref().expect("active_session should be persisted");
        assert_eq!(active.project_id, "p1");
        assert_eq!(active.session_name, "Sesión");
        assert_eq!(active.parent_session_id.as_deref(), Some("s1"));
        assert!(active.is_tracking);
        assert_eq!(active.elapsed_secs, 120);
        assert!(active.start_unix > 0);
    }

    #[test]
    fn test_sync_active_session_none_when_empty() {
        let mut state = TimeTrackerState::default();
        state.sync_active_session();
        assert_eq!(state.data.active_session, None);
    }

    #[test]
    fn test_restored_session_state_tracking_folds_gap() {
        let active = ActiveSession {
            project_id: "p1".into(),
            session_name: "S".into(),
            parent_session_id: None,
            is_tracking: true,
            start_unix: 1_000,
            elapsed_secs: 50,
        };
        // 300s passed since start -> 50 + 300 accumulated, still tracking.
        let (is_tracking, elapsed) = restored_session_state(&active, 1_300);
        assert!(is_tracking);
        assert_eq!(elapsed, 350);
    }

    #[test]
    fn test_restored_session_state_paused_keeps_accumulated() {
        let active = ActiveSession {
            project_id: "p1".into(),
            session_name: "S".into(),
            parent_session_id: None,
            is_tracking: false,
            start_unix: 0,
            elapsed_secs: 42,
        };
        let (is_tracking, elapsed) = restored_session_state(&active, 9_999_999);
        assert!(!is_tracking);
        assert_eq!(elapsed, 42);
    }

    #[test]
    fn test_active_session_serde_roundtrip() {
        let mut data = TrackerData::default();
        data.projects.push(Project { id: "p1".into(), name: "P".into(), sessions: vec![] });
        data.active_session = Some(ActiveSession {
            project_id: "p1".into(),
            session_name: "S".into(),
            parent_session_id: Some("s1".into()),
            is_tracking: true,
            start_unix: 1_700_000_000,
            elapsed_secs: 300,
        });

        let json = serde_json::to_string(&data).unwrap();
        let loaded: TrackerData = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.active_session, data.active_session);
    }
}
