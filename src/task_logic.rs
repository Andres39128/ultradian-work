use std::path::Path;
use std::time::Instant;

use crate::models::*;
use crate::tracker::TimeTrackerState;

impl TimeTrackerState {
    pub(crate) fn create_task(&mut self) {
        self.new_task_error = None;
        if self.new_task_name.trim().is_empty() {
            self.new_task_error =
                Some(crate::i18n::t(&self.data.language, "error_empty_name").to_string());
            return;
        }
        self.data.tasks.push(Task {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.new_task_name.clone(),
            description: self.new_task_description.clone(),
            completed: false,
            project: if self.new_task_project_input.is_empty() {
                None
            } else {
                Some(self.new_task_project_input.clone())
            },
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

    fn set_export_message(&mut self, message: String) {
        self.export_message = Some(message);
        self.export_message_time = Some(Instant::now());
    }

    pub(crate) fn import_tasks_from_file(&mut self, path: &Path) {
        use calamine::{Reader, open_workbook_auto};
        let lang = self.data.language;
        let Ok(mut workbook) = open_workbook_auto(path) else {
            self.set_export_message(crate::i18n::t(&lang, "error_open_file").to_string());
            return;
        };
        let Ok(range) = workbook.worksheet_range("Pendientes") else {
            self.set_export_message(crate::i18n::t(&lang, "sheet_not_found").to_string());
            return;
        };

        let mut imported = 0;
        for (i, row) in range.rows().enumerate() {
            if i == 0 {
                continue;
            }
            if row.is_empty() || row.iter().all(|c| c.to_string().trim().is_empty()) {
                continue;
            }

            let name = row
                .first()
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }

            let description = row
                .get(1)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default();

            let proj_name = row
                .get(2)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default();
            let project = if proj_name.is_empty() {
                None
            } else {
                if let Some(p) = self
                    .data
                    .projects
                    .iter()
                    .find(|p| p.name.trim().to_lowercase() == proj_name.to_lowercase())
                {
                    Some(p.id.clone())
                } else {
                    let id = uuid::Uuid::new_v4().to_string();
                    self.data.projects.push(Project {
                        id: id.clone(),
                        name: proj_name.clone(),
                        sessions: Vec::new(),
                    });
                    Some(id)
                }
            };

            if self.data.tasks.iter().any(|t| {
                t.name.trim().to_lowercase() == name.to_lowercase() && t.project == project
            }) {
                continue;
            }

            let completed_str = row
                .get(3)
                .map(|c| c.to_string().trim().to_lowercase())
                .unwrap_or_default();
            let completed = matches!(completed_str.as_str(), "true" | "si" | "1" | "yes");
            let priority_str = row
                .get(4)
                .map(|c| c.to_string().trim().to_lowercase())
                .unwrap_or_default();
            let priority = match priority_str.as_str() {
                "alta" | "high" => Priority::Alta,
                "media" | "medium" => Priority::Media,
                "baja" | "low" => Priority::Baja,
                _ => Priority::Media,
            };
            let tags = row
                .get(5)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default();
            let deadline = row
                .get(6)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default();

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
        self.set_export_message(format!(
            "{} {}",
            imported,
            crate::i18n::t(&lang, "imported_tasks")
        ));
    }

    pub(crate) fn export_tasks_to_file(&mut self, path: &Path) {
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
                    self.data
                        .projects
                        .iter()
                        .find(|p| p.id == *pid)
                        .map(|p| p.name.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let _ = worksheet.write_string(row, 2, &proj_name);

                let _ =
                    worksheet.write_string(row, 3, if task.completed { "true" } else { "false" });
                let prio_str = match task.priority {
                    Priority::Alta => "Alta",
                    Priority::Media => "Media",
                    Priority::Baja => "Baja",
                };
                let _ = worksheet.write_string(row, 4, prio_str);
                let _ = worksheet.write_string(row, 5, &task.tags);
                let _ = worksheet.write_string(row, 6, &task.deadline);
            }

            if workbook.save(path).is_ok() {
                self.set_export_message(format!(
                    "{} {}",
                    crate::i18n::t(&lang, "exported_to"),
                    path.display()
                ));
            } else {
                self.set_export_message(crate::i18n::t(&lang, "error_export").to_string());
            }
        }
    }

    pub(crate) fn delete_task(&mut self, task_id: &str) {
        self.data.tasks.retain(|t| t.id != task_id);
        self.save();
    }

    /// Sets a task's completed flag without persisting; the caller decides
    /// when to save (batching).
    pub(crate) fn set_task_completed(&mut self, task_id: &str, completed: bool) {
        if let Some(t) = self.data.tasks.iter_mut().find(|t| t.id == task_id) {
            t.completed = completed;
        }
    }

    /// Starts tracking a session for a task. Returns false (and does nothing)
    /// if a session is already tracking.
    pub(crate) fn start_task_session(&mut self, project_id: String, task_name: String) -> bool {
        if self.is_tracking {
            return false;
        }
        self.active_project_id = Some(project_id);
        self.active_session_name = format!(
            "{} {}",
            crate::i18n::t(&self.data.language, "task_session_prefix"),
            task_name
        );
        self.active_parent_session_id = None;
        self.current_session_start = Some(Instant::now());
        self.is_tracking = true;
        self.save();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::DATA_PATH_LOCK;

    #[test]
    fn test_export_import_tasks() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let export_path = temp_dir.join(format!("test_export_{}.xlsx", std::process::id()));
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
        assert!(!imported.completed);

        let _ = std::fs::remove_file(&export_path);
    }

    #[test]
    fn test_export_import_roundtrip_preserves_all_fields() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let export_path = temp_dir.join(format!("test_roundtrip_{}.xlsx", std::process::id()));
        let data_path = temp_dir.join(format!(
            "test_tracker_data_roundtrip_{}.json",
            std::process::id()
        ));

        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&export_path);
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();
        state.data.projects.push(Project {
            id: "proj-1".into(),
            name: "Mi Proyecto".into(),
            sessions: Vec::new(),
        });
        state.data.tasks.push(Task {
            id: "t1".into(),
            name: "Con proyecto".into(),
            description: "Desc con espacios".into(),
            project: Some("proj-1".into()),
            completed: true,
            priority: Priority::Alta,
            tags: "tag1, tag2".into(),
            deadline: "2026-12-31".into(),
        });
        state.data.tasks.push(Task {
            id: "t2".into(),
            name: "Sin proyecto".into(),
            description: String::new(),
            project: None,
            completed: false,
            priority: Priority::Baja,
            tags: String::new(),
            deadline: String::new(),
        });
        state.export_tasks_to_file(&export_path);

        // Fresh state without projects or tasks: import must rebuild everything from the sheet.
        let mut fresh = TimeTrackerState::load();
        fresh.data.projects.clear();
        fresh.data.tasks.clear();
        fresh.import_tasks_from_file(&export_path);

        assert_eq!(fresh.data.tasks.len(), 2);
        let by_name = |n: &str| {
            fresh
                .data
                .tasks
                .iter()
                .find(|t| t.name == n)
                .unwrap_or_else(|| panic!("task {n} missing"))
        };
        let t1 = by_name("Con proyecto");
        assert_eq!(t1.description, "Desc con espacios");
        assert!(t1.completed);
        assert_eq!(t1.priority, Priority::Alta);
        assert_eq!(t1.tags, "tag1, tag2");
        assert_eq!(t1.deadline, "2026-12-31");
        let proj = fresh
            .data
            .projects
            .iter()
            .find(|p| p.id == t1.project.as_deref().expect("project id"))
            .expect("project recreated");
        assert_eq!(proj.name, "Mi Proyecto");

        let t2 = by_name("Sin proyecto");
        assert!(!t2.completed);
        assert_eq!(t2.priority, Priority::Baja);
        assert_eq!(t2.project, None);

        let _ = std::fs::remove_file(&export_path);
        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn test_import_priority_is_case_insensitive() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let xlsx_path = temp_dir.join(format!("test_import_priority_{}.xlsx", std::process::id()));
        let data_path = temp_dir.join(format!("test_tracker_data_priority_{}.json", std::process::id()));
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&xlsx_path);
        let _ = std::fs::remove_file(&data_path);

        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook
            .add_worksheet()
            .set_name("Pendientes")
            .expect("create Pendientes worksheet");
        for (i, header) in [
            "Nombre",
            "Descripcion",
            "Proyecto",
            "Completado",
            "Prioridad",
            "Tags",
            "Fecha limite",
        ]
        .into_iter()
        .enumerate()
        {
            let _ = worksheet.write_string(0, i as u16, header);
        }
        for (r, (name, priority)) in [
            ("T Alta", "High"),
            ("T Media", "medium"),
            ("T Baja", "Baja"),
            ("T Typo", "Urgente"),
        ]
        .into_iter()
        .enumerate()
        {
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
        let priority_of = |name: &str| {
            state
                .data
                .tasks
                .iter()
                .find(|t| t.name == name)
                .unwrap()
                .priority
                .clone()
        };
        assert_eq!(priority_of("T Alta"), Priority::Alta);
        assert_eq!(priority_of("T Media"), Priority::Media);
        assert_eq!(priority_of("T Baja"), Priority::Baja);
        assert_eq!(priority_of("T Typo"), Priority::Media);

        let _ = std::fs::remove_file(&xlsx_path);
        let _ = std::fs::remove_file(&data_path);
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
    fn test_delete_task_removes_it() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let data_path = std::env::temp_dir().join("test_tracker_data_delete_task.json");
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();
        state.data.tasks.push(Task {
            id: "t1".into(),
            name: "A".into(),
            description: String::new(),
            completed: false,
            project: None,
            priority: Priority::Media,
            tags: String::new(),
            deadline: String::new(),
        });
        state.data.tasks.push(Task {
            id: "t2".into(),
            name: "B".into(),
            description: String::new(),
            completed: false,
            project: None,
            priority: Priority::Media,
            tags: String::new(),
            deadline: String::new(),
        });

        state.delete_task("t1");

        assert_eq!(state.data.tasks.len(), 1);
        assert_eq!(state.data.tasks[0].id, "t2");

        // The real save() persisted the deletion to the injected data path.
        let reloaded = TimeTrackerState::load();
        assert_eq!(reloaded.data.tasks.len(), 1);
        assert_eq!(reloaded.data.tasks[0].id, "t2");

        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn test_start_task_session_ignored_while_tracking() {
        let mut state = TimeTrackerState {
            is_tracking: true,
            ..Default::default()
        };

        let started = state.start_task_session("p1".into(), "T".into());

        assert!(!started);
        assert_eq!(state.active_project_id, None);
        assert!(state.active_session_name.is_empty());
    }

    #[test]
    fn test_start_task_session_activates_project_and_tracking() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let data_path = std::env::temp_dir().join("test_tracker_data_start_task.json");
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();

        let started = state.start_task_session("p1".into(), "Mi tarea".into());

        assert!(started);
        assert_eq!(state.active_project_id, Some("p1".into()));
        assert!(state.is_tracking);
        assert!(state.current_session_start.is_some());
        assert!(state.active_session_name.ends_with("Mi tarea"));
        assert_eq!(state.active_parent_session_id, None);

        let _ = std::fs::remove_file(&data_path);
    }
}
