use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::i18n::Language;
use eframe::egui;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Priority {
    Alta,
    Media,
    Baja,
}
impl Default for Priority {
    fn default() -> Self { Priority::Media }
}

fn default_language() -> Language { Language::Es }

#[derive(Serialize, Deserialize, Clone)]
pub struct TrackerData {
    #[serde(default = "default_language")]
    pub language: Language,
    #[serde(default)]
    pub work_duration_mins: u64,
    #[serde(default)]
    pub rest_duration_mins: u64,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
}

impl Default for TrackerData {
    fn default() -> Self {
        Self {
            language: default_language(),
            work_duration_mins: 90,
            rest_duration_mins: 15,
            projects: Vec::new(),
            tasks: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub sessions: Vec<Session>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub sub_sessions: Vec<SubSession>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SubSession {
    pub start_time: u64,
    pub end_time: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub completed: bool,
    pub project: Option<String>,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub deadline: String,
}

impl Project {
    pub fn total_duration(&self) -> u64 {
        self.sessions.iter().map(|s| {
            let mut total = if s.end_time >= s.start_time { s.end_time - s.start_time } else { 0 };
            for sub in &s.sub_sessions {
                if sub.end_time >= sub.start_time {
                    total += sub.end_time - sub.start_time;
                }
            }
            total
        }).sum()
    }
}

pub struct TimeTrackerState {
    pub data: TrackerData,
    pub active_project_id: Option<String>,
    pub active_session_name: String,
    pub active_parent_session_id: Option<String>,

    pub is_tracking: bool,
    pub current_session_start: Option<Instant>,
    pub current_session_elapsed: u64,

    pub new_project_name: String,
    
    pub export_message: Option<String>,
    pub export_message_time: Option<Instant>,
    pub editing_session_id: Option<(String, String)>,
    pub editing_session_name: String,
    pub deleting_session_id: Option<(String, String)>,
    pub session_search_query: String,

    pub new_task_project_input: String,
    pub new_task_name: String,
    pub new_task_description: String,
    pub new_task_priority: Priority,
    pub new_task_tags: String,
    pub new_task_deadline: String,
}

impl TimeTrackerState {
    fn get_data_path() -> PathBuf {
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
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            TrackerData::default()
        };

        Self {
            data,
            active_project_id: None,
            active_session_name: String::new(),
            active_parent_session_id: None,
            is_tracking: false,
            current_session_start: None,
            current_session_elapsed: 0,
            new_project_name: String::new(),
            export_message: None,
            export_message_time: None,
            editing_session_id: None,
            editing_session_name: String::new(),
            deleting_session_id: None,
            session_search_query: String::new(),
            new_task_project_input: String::new(),
            new_task_name: String::new(),
            new_task_description: String::new(),
            new_task_priority: Priority::Media,
            new_task_tags: String::new(),
            new_task_deadline: String::new(),
        }
    }

    pub fn save(&self) {
        let path = Self::get_data_path();
        match serde_json::to_string_pretty(&self.data) {
            Ok(content) => {
                if let Err(e) = fs::write(&path, content) {
                    eprintln!("Error guardando datos del tracker en {}: {}", path.display(), e);
                }
            }
            Err(e) => {
                eprintln!("Error serializando datos del tracker: {}", e);
            }
        }
    }

    fn add_project(&mut self) {
        if self.new_project_name.trim().is_empty() { return; }
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

    pub fn toggle_tracking(&mut self) {
        if self.is_tracking {
            if let Some(start) = self.current_session_start {
                self.current_session_elapsed += start.elapsed().as_secs();
            }
            self.current_session_start = None;
            self.is_tracking = false;
        } else {
            self.current_session_start = Some(Instant::now());
            self.is_tracking = true;
        }
    }

    fn finish_session(&mut self) {
        if let Some(proj_id) = &self.active_project_id {
            if let Some(proj) = self.data.projects.iter_mut().find(|p| &p.id == proj_id) {
                if self.is_tracking {
                    if let Some(start) = self.current_session_start {
                        self.current_session_elapsed += start.elapsed().as_secs();
                    }
                }

                let end = chrono::Utc::now().timestamp() as u64;
                let start = if self.current_session_elapsed > 0 {
                    end.saturating_sub(self.current_session_elapsed)
                } else {
                    end
                };

                let name = if self.active_session_name.trim().is_empty() {
                    format!("Sesión {}", proj.sessions.len() + 1)
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
            }
        }
        self.is_tracking = false;
        self.current_session_start = None;
        self.current_session_elapsed = 0;
        self.active_session_name.clear();
        self.active_parent_session_id = None;
        self.save();
    }

    fn export_project(&mut self, proj: &Project) {
        let file_path = if let Some(path) = rfd::FileDialog::new()
            .add_filter("Excel Workbook", &["xlsx"])
            .set_file_name(&format!("{}_export.xlsx", proj.name))
            .save_file() {
                path
        } else {
            return;
        };

        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        {
            let _ = worksheet.write_string(0, 0, "Sesión");
            let _ = worksheet.write_string(0, 1, "Duración (minutos)");
            let _ = worksheet.write_string(0, 2, "Fecha (Unix)");

            let mut row = 1;
            for session in &proj.sessions {
                let mut total_secs = if session.end_time >= session.start_time { session.end_time - session.start_time } else { 0 };
                for sub in &session.sub_sessions {
                    if sub.end_time >= sub.start_time {
                        total_secs += sub.end_time - sub.start_time;
                    }
                }

                let _ = worksheet.write_string(row, 0, &session.name);
                let _ = worksheet.write_number(row, 1, total_secs as f64 / 60.0);
                let _ = worksheet.write_number(row, 2, session.start_time as f64);
                row += 1;
            }

            if workbook.save(&file_path).is_ok() {
                self.export_message = Some(format!("Exportado a {:?}", file_path.display()));
                self.export_message_time = Some(Instant::now());
            } else {
                self.export_message = Some("Error al exportar".to_string());
                self.export_message_time = Some(Instant::now());
            }
        }
    }

    pub fn ui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Nuevo Proyecto:");
            ui.text_edit_singleline(&mut self.new_project_name);
            if ui.button("Crear").clicked() {
                self.add_project();
            }
        });

        ui.separator();

        let available_height = ui.available_height();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_min_height(available_height);
                ui.set_width(200.0);
                ui.heading("Tus Proyectos");
                for proj in &self.data.projects {
                    let is_selected = self.active_project_id.as_deref() == Some(&proj.id);
                    if ui.selectable_label(is_selected, &proj.name).clicked() {
                        if !self.is_tracking {
                            self.active_project_id = Some(proj.id.clone());
                        }
                    }
                }
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.set_min_height(available_height);
                ui.set_min_width(ui.available_width());
                if let Some(proj_id) = self.active_project_id.clone() {
                    let proj_name = self.data.projects.iter().find(|p| p.id == proj_id).map(|p| p.name.clone()).unwrap_or_default();
                    
                    ui.heading(format!("Proyecto: {}", proj_name));
                    ui.add_space(10.0);

                    if ui.button("📥 Exportar a Excel (.xlsx)").clicked() {
                        if let Some(proj) = self.data.projects.iter().find(|p| p.id == proj_id).cloned() {
                            self.export_project(&proj);
                        }
                    }

                    if let Some(msg) = &self.export_message {
                        if let Some(time) = self.export_message_time {
                            if time.elapsed().as_secs() < 5 {
                                ui.label(egui::RichText::new(msg).color(egui::Color32::GREEN));
                            } else {
                                self.export_message = None;
                            }
                        }
                    }

                    ui.add_space(20.0);
                    
                    ui.group(|ui| {
                        if let Some(_) = &self.active_parent_session_id {
                            ui.heading(format!("Continuando sesión: {}", self.active_session_name));
                        } else {
                            ui.heading("Nueva Sesión de Trabajo");
                            ui.horizontal(|ui| {
                                ui.label("Nombre:");
                                ui.add_enabled(
                                    !self.is_tracking,
                                    egui::TextEdit::singleline(&mut self.active_session_name),
                                );
                            });
                        }

                        let mut current_secs = self.current_session_elapsed;
                        if let Some(start) = self.current_session_start {
                            current_secs += start.elapsed().as_secs();
                        }
                        let display = format!("{:02}:{:02}:{:02}", current_secs / 3600, (current_secs % 3600) / 60, current_secs % 60);

                        ui.label(egui::RichText::new(display).size(30.0).strong().color(if self.is_tracking { egui::Color32::GREEN } else { egui::Color32::WHITE }));

                        ui.horizontal(|ui| {
                            let btn_text = if self.is_tracking { "⏸ Pausar" } else { "▶ Empezar" };
                            if ui.button(btn_text).clicked() {
                                self.toggle_tracking();
                            }

                            if current_secs > 0 || self.active_parent_session_id.is_some() {
                                if ui.button("⏹ Finalizar y Guardar").clicked() {
                                    self.finish_session();
                                }
                            }
                        });
                    });

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.heading("Historial de Sesiones");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.text_edit_singleline(&mut self.session_search_query);
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
                                                if ui.button("Guardar").clicked() {
                                                    session_to_save = Some((session.id.clone(), self.editing_session_name.clone()));
                                                    self.editing_session_id = None;
                                                }
                                                if ui.button("Cancelar").clicked() {
                                                    self.editing_session_id = None;
                                                }
                                            } else {
                                                ui.label(egui::RichText::new(&session.name).strong());
                                                
                                                if ui.button("Continuar").clicked() {
                                                    continue_session = Some((proj_id.clone(), session.id.clone(), session.name.clone()));
                                                }

                                                if ui.button("✏").clicked() {
                                                    self.editing_session_id = Some((proj_id.clone(), session.id.clone()));
                                                    self.editing_session_name = session.name.clone();
                                                }
                                                if ui.button("🗑").clicked() {
                                                    self.deleting_session_id = Some((proj_id.clone(), session.id.clone()));
                                                }
                                            }
                                        });

                                        let mut total_secs = if session.end_time >= session.start_time { session.end_time - session.start_time } else { 0 };
                                        for sub in &session.sub_sessions {
                                            if sub.end_time >= sub.start_time {
                                                total_secs += sub.end_time - sub.start_time;
                                            }
                                        }

                                        let time_str = format!("{:02}:{:02}:{:02}", total_secs / 3600, (total_secs % 3600) / 60, total_secs % 60);
                                        ui.label(format!("Duración Total: {}", time_str));
                                        
                                        if !session.sub_sessions.is_empty() {
                                            ui.collapsing(format!("{} sub-sesiones", session.sub_sessions.len()), |ui| {
                                                for (_idx, sub) in session.sub_sessions.iter().enumerate() {
                                                    let sub_secs = if sub.end_time >= sub.start_time { sub.end_time - sub.start_time } else { 0 };
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

                    if let Some((sid, new_name)) = session_to_save {
                        if let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == proj_id) {
                            if let Some(sess) = proj.sessions.iter_mut().find(|s| s.id == sid) {
                                sess.name = new_name;
                                self.save();
                            }
                        }
                    }

                    if let Some((_pid, sid, sname)) = continue_session {
                        if !self.is_tracking {
                            self.active_parent_session_id = Some(sid);
                            self.active_session_name = sname;
                        }
                    }

                    if let Some((del_pid, del_sid)) = &self.deleting_session_id {
                        let mut confirm_delete = false;
                        let mut cancel_delete = false;
                        
                        egui::Window::new("Confirmar Eliminación")
                            .collapsible(false)
                            .resizable(false)
                            .show(ui.ctx(), |ui| {
                                ui.label("¿Estás seguro de que deseas eliminar esta sesión? Esta acción no se puede deshacer.");
                                ui.horizontal(|ui| {
                                    if ui.button("Sí, Eliminar").clicked() {
                                        confirm_delete = true;
                                    }
                                    if ui.button("Cancelar").clicked() {
                                        cancel_delete = true;
                                    }
                                });
                            });

                        if confirm_delete {
                            if let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == *del_pid) {
                                proj.sessions.retain(|s| s.id != *del_sid);
                                self.save();
                            }
                            session_to_delete = Some(del_sid.clone());
                        }
                        if confirm_delete || cancel_delete {
                            session_to_delete = Some("handled".to_string());
                        }
                    }

                    if session_to_delete.is_some() {
                        self.deleting_session_id = None;
                    }
                } else {
                    ui.label("Selecciona un proyecto a la izquierda o crea uno nuevo.");
                }
            });
        });
    }

    pub fn ui_tasks(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
        let mut redirect_to_tracker = false;

        ui.horizontal(|ui| {
            if ui.button("📤 Exportar Pendientes").clicked() {
                self.export_tasks();
            }
            if ui.button("📥 Importar Pendientes").clicked() {
                self.import_tasks();
            }
        });
        
        if let Some(msg) = &self.export_message {
            if let Some(time) = self.export_message_time {
                if time.elapsed().as_secs() < 5 {
                    ui.label(egui::RichText::new(msg).color(egui::Color32::GREEN));
                } else {
                    self.export_message = None;
                }
            }
        }

        ui.separator();

        ui.group(|ui| {
            ui.heading("Nuevo Pendiente");
            ui.horizontal(|ui| {
                ui.label("Nombre:");
                ui.text_edit_singleline(&mut self.new_task_name);
            });
            ui.horizontal(|ui| {
                ui.label("Proyecto (Opcional):");
                egui::ComboBox::from_id_salt("proj_select")
                    .selected_text(if self.new_task_project_input.is_empty() { "Ninguno" } else { &self.new_task_project_input })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.new_task_project_input, String::new(), "Ninguno");
                        for proj in &self.data.projects {
                            ui.selectable_value(&mut self.new_task_project_input, proj.id.clone(), &proj.name);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Prioridad:");
                ui.radio_value(&mut self.new_task_priority, Priority::Alta, "Alta");
                ui.radio_value(&mut self.new_task_priority, Priority::Media, "Media");
                ui.radio_value(&mut self.new_task_priority, Priority::Baja, "Baja");
            });
            ui.horizontal(|ui| {
                ui.label("Tags (coma separados):");
                ui.text_edit_singleline(&mut self.new_task_tags);
            });
            ui.horizontal(|ui| {
                ui.label("Fecha límite:");
                ui.text_edit_singleline(&mut self.new_task_deadline);
            });
            ui.horizontal(|ui| {
                ui.label("Descripción:");
                ui.text_edit_multiline(&mut self.new_task_description);
            });
            if ui.button("Crear Tarea").clicked() {
                self.create_task();
            }
        });

        ui.add_space(20.0);
        ui.heading("Lista de Pendientes");

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
            let mut to_delete = None;
            let mut save_needed = false;
            let mut start_task_session = None;

            for task in &mut self.data.tasks {
                ui.push_id(&task.id, |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let mut completed = task.completed;
                            if ui.checkbox(&mut completed, &task.name).changed() {
                                task.completed = completed;
                                save_needed = true;
                            }

                            if ui.button("🗑").clicked() {
                                to_delete = Some(task.id.clone());
                            }

                            if !task.completed {
                                if let Some(proj_id) = &task.project {
                                    if ui.button("⏱ Trabajar en esto").clicked() {
                                        start_task_session = Some((proj_id.clone(), task.name.clone()));
                                    }
                                }
                            }
                        });
                        
                        if !task.description.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(25.0);
                                ui.label(&task.description);
                            });
                        }
                        ui.horizontal(|ui| {
                            let color = match task.priority { Priority::Alta => egui::Color32::RED, Priority::Media => egui::Color32::YELLOW, Priority::Baja => egui::Color32::GREEN };
                            ui.label(egui::RichText::new(match task.priority { Priority::Alta => "Alta", Priority::Media => "Media", Priority::Baja => "Baja" }).color(color));
                            if !task.tags.is_empty() { ui.label(format!("🏷 {}", task.tags)); }
                            if !task.deadline.is_empty() { ui.label(format!("📅 {}", task.deadline)); }
                        });
                    });
                });
            }

            if let Some(id) = to_delete {
                self.data.tasks.retain(|t| t.id != id);
                self.save();
            } else if save_needed {
                self.save();
            }

            if let Some((proj_id, task_name)) = start_task_session {
                if !self.is_tracking {
                    self.active_project_id = Some(proj_id);
                    self.active_session_name = format!("Tarea: {}", task_name);
                    self.active_parent_session_id = None;
                    self.current_session_start = Some(Instant::now());
                    self.is_tracking = true;
                    redirect_to_tracker = true;
                }
            }
        });

        redirect_to_tracker
    }

    fn create_task(&mut self) {
        if self.new_task_name.trim().is_empty() { return; }
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

    fn import_tasks(&mut self) {
        use calamine::{Reader, open_workbook_auto};
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Excel Workbook", &["xlsx"])
            .pick_file() {
            if let Ok(mut workbook) = open_workbook_auto(&path) {
                if let Ok(range) = workbook.worksheet_range("Pendientes") {
                    let mut imported = 0;
                    for (i, row) in range.rows().enumerate() {
                        if i == 0 { continue; } 
                        if row.is_empty() { continue; }

                        let name = row.get(0).map(|c| c.to_string()).unwrap_or_default();
                        if name.is_empty() { continue; }

                        let description = row.get(1).map(|c| c.to_string()).unwrap_or_default();
                        
                        let proj_name = row.get(2).map(|c| c.to_string()).unwrap_or_default();
                        let project = if proj_name.is_empty() {
                            None
                        } else {
                            if let Some(p) = self.data.projects.iter().find(|p| p.name == proj_name) {
                                Some(p.id.clone())
                            } else {
                                let id = uuid::Uuid::new_v4().to_string();
                                self.data.projects.push(Project {
                                    id: id.clone(),
                                    name: proj_name,
                                    sessions: Vec::new()
                                });
                                Some(id)
                            }
                        };

                        let completed = row.get(3).map(|c| c.to_string() == "Si").unwrap_or(false);
                        let priority_str = row.get(4).map(|c| c.to_string()).unwrap_or_default();
                        let priority = match priority_str.as_str() { "Alta" => Priority::Alta, "Baja" => Priority::Baja, _ => Priority::Media };
                        let tags = row.get(5).map(|c| c.to_string()).unwrap_or_default();
                        let deadline = row.get(6).map(|c| c.to_string()).unwrap_or_default();

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
                    self.export_message = Some(format!("{} importadas", imported));
                    self.export_message_time = Some(Instant::now());
                }
            }
        }
    }

    fn export_tasks(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Excel Workbook", &["xlsx"])
            .set_file_name("pendientes_export.xlsx")
            .save_file() {
            let mut workbook = rust_xlsxwriter::Workbook::new();
            if let Ok(worksheet) = workbook.add_worksheet().set_name("Pendientes") {
                let _ = worksheet.write_string(0, 0, "Nombre");
                let _ = worksheet.write_string(0, 1, "Descripción");
                let _ = worksheet.write_string(0, 2, "Proyecto");
                let _ = worksheet.write_string(0, 3, "Completada");
                let _ = worksheet.write_string(0, 4, "Prioridad");
                let _ = worksheet.write_string(0, 5, "Etiquetas");
                let _ = worksheet.write_string(0, 6, "Fecha Limite");

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
                    
                    let _ = worksheet.write_string(row, 3, if task.completed { "Si" } else { "No" });
                    let prio_str = match task.priority { Priority::Alta => "Alta", Priority::Media => "Media", Priority::Baja => "Baja" };
                    let _ = worksheet.write_string(row, 4, prio_str);
                    let _ = worksheet.write_string(row, 5, &task.tags);
                    let _ = worksheet.write_string(row, 6, &task.deadline);
                }
                
                if workbook.save(&path).is_ok() {
                    // Update state manually since we are in &self
                    // For export message, normally needs &mut self, skipped for pure dump
                }
            }
        }
    }

    pub fn ui_dashboard(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Dashboard - Horas por Proyecto");
        ui.add_space(20.0);

        let mut bars = Vec::new();
        for (i, proj) in self.data.projects.iter().enumerate() {
            let hours = proj.total_duration() as f64 / 3600.0;
            bars.push(egui_plot::Bar::new(i as f64, hours).name(proj.name.clone()));
        }
        
        let chart = egui_plot::BarChart::new("Proyectos", bars);
        egui_plot::Plot::new("dashboard_plot")
            .allow_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .show(ui, |plot_ui| plot_ui.bar_chart(chart));
    }

    pub fn ui_settings(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Configuración");
        ui.add_space(20.0);

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Minutos Trabajo Profundo:");
            if ui.add(egui::Slider::new(&mut self.data.work_duration_mins, 1..=120)).changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            ui.label("Minutos Descanso:");
            if ui.add(egui::Slider::new(&mut self.data.rest_duration_mins, 1..=60)).changed() { changed = true; }
        });
        
        if changed {
            self.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                    end_time: 200, // duration 100
                    sub_sessions: vec![
                        SubSession { start_time: 250, end_time: 300 }, // duration 50
                        SubSession { start_time: 350, end_time: 300 }, // duration 0 (invalid/reversed)
                    ],
                },
                Session {
                    id: "s2".into(),
                    name: "S2".into(),
                    start_time: 500,
                    end_time: 400, // duration 0 (invalid/reversed)
                    sub_sessions: vec![],
                },
            ],
        };
        // 100 + 50 + 0 + 0 = 150
        assert_eq!(proj.total_duration(), 150);
    }
}
