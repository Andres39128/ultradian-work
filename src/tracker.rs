use chrono::{DateTime, Local};
use eframe::egui;
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::Instant,
};

fn default_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    #[serde(default = "default_uuid")]
    pub id: String,
    pub name: String,
    pub date: DateTime<Local>,
    pub duration_secs: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub sessions: Vec<Session>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    #[serde(default = "default_uuid")]
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct TrackerData {
    pub projects: Vec<Project>,
    #[serde(default)]
    pub tasks: Vec<Task>,
}

pub struct TimeTrackerState {
    pub data: TrackerData,
    pub active_project_id: Option<String>,
    pub active_session_name: String,

    pub is_tracking: bool,
    pub current_session_start: Option<Instant>,
    pub current_session_elapsed: u64, // Accumulated seconds

    pub new_project_name: String,
    
    // Feedback and edit states
    pub export_message: Option<String>,
    pub export_message_time: Option<Instant>,
    pub editing_session_id: Option<(String, String)>, // (proj_id, session_id)
    pub editing_session_name: String,
    pub deleting_session_id: Option<(String, String)>, // (proj_id, session_id)

    // Task fields
    pub new_task_project_input: String,
    pub new_task_name: String,
    pub new_task_description: String,
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
            is_tracking: false,
            current_session_start: None,
            current_session_elapsed: 0,
            new_project_name: String::new(),
            export_message: None,
            export_message_time: None,
            editing_session_id: None,
            editing_session_name: String::new(),
            deleting_session_id: None,
            new_task_project_input: String::new(),
            new_task_name: String::new(),
            new_task_description: String::new(),
        }
    }

    pub fn save(&self) {
        let path = Self::get_data_path();
        if let Ok(content) = serde_json::to_string_pretty(&self.data) {
            let _ = fs::write(path, content);
        }
    }

    fn add_project(&mut self) {
        if self.new_project_name.trim().is_empty() {
            return;
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.data.projects.push(Project {
            id: id.clone(),
            name: self.new_project_name.trim().to_string(),
            sessions: Vec::new(),
        });
        self.new_project_name.clear();
        self.active_project_id = Some(id);
        self.save();
    }

    fn toggle_tracking(&mut self) {
        if self.is_tracking {
            // Stop
            if let Some(start) = self.current_session_start {
                self.current_session_elapsed += start.elapsed().as_secs();
            }
            self.is_tracking = false;
            self.current_session_start = None;
        } else {
            // Start
            if self.active_project_id.is_some() && !self.active_session_name.trim().is_empty() {
                self.is_tracking = true;
                self.current_session_start = Some(Instant::now());
            }
        }
    }

    fn finish_session(&mut self) {
        if let Some(start) = self.current_session_start {
            self.current_session_elapsed += start.elapsed().as_secs();
        }
        
        let secs = self.current_session_elapsed;
        if secs > 0 {
            if let Some(proj_id) = &self.active_project_id {
                if let Some(proj) = self.data.projects.iter_mut().find(|p| &p.id == proj_id) {
                    proj.sessions.push(Session {
                        id: default_uuid(),
                        name: self.active_session_name.trim().to_string(),
                        date: Local::now(),
                        duration_secs: secs,
                    });
                }
            }
        }

        self.is_tracking = false;
        self.current_session_start = None;
        self.current_session_elapsed = 0;
        self.active_session_name.clear();
        self.save();
    }

    fn export_project(&mut self, proj: &Project) {
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        
        // Headers
        let _ = sheet.write_string(0, 0, "Proyecto");
        let _ = sheet.write_string(0, 1, "Sesión");
        let _ = sheet.write_string(0, 2, "Fecha");
        let _ = sheet.write_string(0, 3, "Tiempo Trabajado (hrs)");
        let _ = sheet.write_string(0, 4, "Tiempo Trabajado (mins)");

        for (i, session) in proj.sessions.iter().enumerate() {
            let row = (i + 1) as u32;
            let _ = sheet.write_string(row, 0, &proj.name);
            let _ = sheet.write_string(row, 1, &session.name);
            let _ = sheet.write_string(row, 2, &session.date.format("%Y-%m-%d %H:%M:%S").to_string());
            let _ = sheet.write_number(row, 3, session.duration_secs as f64 / 3600.0);
            let _ = sheet.write_number(row, 4, session.duration_secs as f64 / 60.0);
        }

        let default_name = format!("{}_Export_{}.xlsx", proj.name.replace(" ", "_"), Local::now().format("%Y%m%d"));
        
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Guardar Excel de Proyecto")
            .add_filter("Excel Document", &["xlsx"])
            .set_file_name(&default_name)
            .save_file()
        {
            if let Err(e) = workbook.save(&path) {
                self.export_message = Some(format!("Error: {}", e));
            } else {
                self.export_message = Some(format!("Guardado: {:?}", path.file_name().unwrap_or_default()));
            }
            self.export_message_time = Some(Instant::now());
        }
    }

    pub fn ui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Time Tracker / Proyectos");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Nuevo Proyecto:");
            ui.text_edit_singleline(&mut self.new_project_name);
            if ui.button("Crear").clicked() {
                self.add_project();
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
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
                        ui.heading("Nueva Sesión de Trabajo");
                        ui.horizontal(|ui| {
                            ui.label("Nombre de sesión:");
                            ui.add_enabled(
                                !self.is_tracking,
                                egui::TextEdit::singleline(&mut self.active_session_name),
                            );
                        });

                        let mut current_secs = self.current_session_elapsed;
                        if let Some(start) = self.current_session_start {
                            current_secs += start.elapsed().as_secs();
                        }
                        let display = format!("{:02}:{:02}:{:02}", current_secs / 3600, (current_secs % 3600) / 60, current_secs % 60);

                        ui.label(egui::RichText::new(display).size(30.0).strong().color(if self.is_tracking { egui::Color32::GREEN } else { egui::Color32::WHITE }));

                        ui.horizontal(|ui| {
                            let btn_text = if self.is_tracking { "Pausar" } else { "Iniciar" };
                            if ui.button(btn_text).clicked() {
                                self.toggle_tracking();
                            }

                            if current_secs > 0 {
                                if ui.button("Finalizar y Guardar").clicked() {
                                    self.finish_session();
                                }
                            }
                        });
                    });

                    ui.add_space(20.0);
                    ui.heading("Historial de Sesiones");
                    let mut session_to_delete = None;
                    let mut session_to_save = None;

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(proj) = self.data.projects.iter().find(|p| p.id == proj_id) {
                            for session in &proj.sessions {
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
                                            if ui.button("✏").clicked() {
                                                self.editing_session_id = Some((proj_id.clone(), session.id.clone()));
                                                self.editing_session_name = session.name.clone();
                                                self.deleting_session_id = None;
                                            }

                                            if self.deleting_session_id.as_ref() == Some(&(proj_id.clone(), session.id.clone())) {
                                                ui.label(egui::RichText::new("¿Seguro?").color(egui::Color32::RED));
                                                if ui.button(egui::RichText::new("Sí, eliminar").color(egui::Color32::RED)).clicked() {
                                                    session_to_delete = Some(session.id.clone());
                                                    self.deleting_session_id = None;
                                                }
                                                if ui.button("Cancelar").clicked() {
                                                    self.deleting_session_id = None;
                                                }
                                            } else {
                                                if ui.button("🗑").clicked() {
                                                    self.deleting_session_id = Some((proj_id.clone(), session.id.clone()));
                                                    self.editing_session_id = None;
                                                }
                                            }
                                        }
                                    });
                                    ui.label(format!("Fecha: {}", session.date.format("%Y-%m-%d %H:%M")));
                                    ui.label(format!("Duración: {} min", session.duration_secs / 60));
                                });
                            }
                        }
                    });

                    // Apply modifications after UI rendering
                    if let Some((sess_id, new_name)) = session_to_save {
                        if let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == proj_id) {
                            if let Some(sess) = proj.sessions.iter_mut().find(|s| s.id == sess_id) {
                                sess.name = new_name;
                            }
                        }
                        self.save();
                    }

                    if let Some(sess_id) = session_to_delete {
                        if let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == proj_id) {
                            proj.sessions.retain(|s| s.id != sess_id);
                        }
                        self.save();
                    }
                } else {
                    ui.label("Selecciona o crea un proyecto para comenzar.");
                }
            });
        });
    }
    pub fn ui_tasks(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
        let mut switch_to_tracker = false;

        ui.heading("📋 Mis Pendientes (Tareas)");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("📥 Importar Excel").clicked() {
                self.import_tasks();
            }
            if ui.button("📤 Exportar Excel").clicked() {
                self.export_tasks();
            }
        });
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Nuevo Pendiente");
            ui.horizontal(|ui| {
                ui.label("Proyecto:");
                ui.text_edit_singleline(&mut self.new_task_project_input);
                egui::ComboBox::from_id_source("proj_select")
                    .selected_text("Seleccionar existente...")
                    .show_ui(ui, |ui| {
                        for p in &self.data.projects {
                            if ui.selectable_label(false, &p.name).clicked() {
                                self.new_task_project_input = p.name.clone();
                            }
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Tarea:");
                ui.text_edit_singleline(&mut self.new_task_name);
            });
            ui.horizontal(|ui| {
                ui.label("Descripción:");
                ui.text_edit_multiline(&mut self.new_task_description);
            });
            if ui.button("Crear Pendiente").clicked() {
                self.create_task();
            }
        });

        ui.add_space(20.0);
        ui.heading("Lista de Pendientes");

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut to_delete = None;
            let mut save_needed = false;
            let mut start_task_session = None;

            for task in &mut self.data.tasks {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let mut completed = task.completed;
                        if ui.checkbox(&mut completed, &task.name).changed() {
                            task.completed = completed;
                            save_needed = true;
                        }
                        
                        let proj_name = self.data.projects.iter().find(|p| p.id == task.project_id).map(|p| p.name.clone()).unwrap_or_else(|| "Desconocido".to_string());
                        ui.label(egui::RichText::new(format!("[{}]", proj_name)).color(egui::Color32::LIGHT_GRAY));

                        if ui.button("▶ Iniciar Sesión").clicked() {
                            start_task_session = Some((task.project_id.clone(), task.name.clone()));
                        }

                        if ui.button("🗑").clicked() {
                            to_delete = Some(task.id.clone());
                        }
                    });
                    if !task.description.is_empty() {
                        ui.label(egui::RichText::new(&task.description).italics().color(egui::Color32::DARK_GRAY));
                    }
                });
            }

            if let Some(id) = to_delete {
                self.data.tasks.retain(|t| t.id != id);
                save_needed = true;
            }

            if let Some((proj_id, task_name)) = start_task_session {
                self.active_project_id = Some(proj_id);
                self.active_session_name = task_name;
                if !self.is_tracking {
                    self.is_tracking = true;
                    self.current_session_start = Some(Instant::now());
                }
                switch_to_tracker = true;
            }

            if save_needed {
                self.save();
            }
        });

        switch_to_tracker
    }

    fn create_task(&mut self) {
        if self.new_task_name.trim().is_empty() || self.new_task_project_input.trim().is_empty() {
            return;
        }

        let proj_name = self.new_task_project_input.trim().to_string();
        let proj_id = if let Some(p) = self.data.projects.iter().find(|p| p.name.eq_ignore_ascii_case(&proj_name)) {
            p.id.clone()
        } else {
            let new_id = default_uuid();
            self.data.projects.push(Project {
                id: new_id.clone(),
                name: proj_name.clone(),
                sessions: Vec::new(),
            });
            new_id
        };

        self.data.tasks.push(Task {
            id: default_uuid(),
            project_id: proj_id,
            name: self.new_task_name.trim().to_string(),
            description: self.new_task_description.trim().to_string(),
            completed: false,
        });

        self.new_task_name.clear();
        self.new_task_description.clear();
        self.save();
    }

    fn import_tasks(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Importar Excel de Pendientes")
            .add_filter("Excel Document", &["xlsx", "xls"])
            .pick_file()
        {
            use calamine::{Reader, open_workbook_auto, DataType};
            if let Ok(mut workbook) = open_workbook_auto(&path) {
                if let Some(Ok(range)) = workbook.worksheet_range_at(0) {
                    let mut is_first = true;
                    for row in range.rows() {
                        if is_first {
                            is_first = false;
                            continue; // skip header
                        }
                        let empty_data = calamine::Data::Empty;
                        let proj_name = row.get(0).unwrap_or(&empty_data).to_string();
                        let task_name = row.get(1).unwrap_or(&empty_data).to_string();
                        let task_desc = row.get(2).unwrap_or(&empty_data).to_string();
                        let completed_str = row.get(3).unwrap_or(&empty_data).to_string().to_lowercase();
                        let completed = completed_str == "sí" || completed_str == "si" || completed_str == "true" || completed_str == "1";

                        if proj_name.is_empty() || task_name.is_empty() {
                            continue;
                        }

                        let proj_id = if let Some(p) = self.data.projects.iter().find(|p| p.name.eq_ignore_ascii_case(&proj_name)) {
                            p.id.clone()
                        } else {
                            let new_id = default_uuid();
                            self.data.projects.push(Project {
                                id: new_id.clone(),
                                name: proj_name.clone(),
                                sessions: Vec::new(),
                            });
                            new_id
                        };

                        self.data.tasks.push(Task {
                            id: default_uuid(),
                            project_id: proj_id,
                            name: task_name,
                            description: task_desc,
                            completed,
                        });
                    }
                    self.save();
                }
            }
        }
    }

    fn export_tasks(&self) {
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        
        let _ = sheet.write_string(0, 0, "Proyecto");
        let _ = sheet.write_string(0, 1, "Tarea");
        let _ = sheet.write_string(0, 2, "Descripción");
        let _ = sheet.write_string(0, 3, "Completada");

        for (i, task) in self.data.tasks.iter().enumerate() {
            let row = (i + 1) as u32;
            let proj_name = self.data.projects.iter().find(|p| p.id == task.project_id).map(|p| p.name.clone()).unwrap_or_default();
            
            let _ = sheet.write_string(row, 0, &proj_name);
            let _ = sheet.write_string(row, 1, &task.name);
            let _ = sheet.write_string(row, 2, &task.description);
            let _ = sheet.write_string(row, 3, if task.completed { "Sí" } else { "No" });
        }

        let default_name = format!("Pendientes_Export_{}.xlsx", Local::now().format("%Y%m%d"));
        
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Guardar Excel de Pendientes")
            .add_filter("Excel Document", &["xlsx"])
            .set_file_name(&default_name)
            .save_file()
        {
            let _ = workbook.save(&path);
        }
    }
}
