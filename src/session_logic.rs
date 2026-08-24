use std::path::Path;
use std::time::Instant;

use crate::models::*;
use crate::tracker::TimeTrackerState;

/// Rebuilds in-memory state from a persisted in-progress session.
/// If it was tracking, folds the time elapsed since the persisted start
/// into the accumulated seconds, so the session keeps counting while the
/// app is closed.
pub(crate) fn restored_session_state(active: &ActiveSession, now_unix: u64) -> (bool, u64) {
    if active.is_tracking && active.start_unix > 0 {
        let gap = now_unix.saturating_sub(active.start_unix);
        (true, active.elapsed_secs + gap)
    } else {
        (false, active.elapsed_secs)
    }
}

impl TimeTrackerState {
    /// Copies the in-memory session state into `data.active_session` so the
    /// next `save()` (and any future `load()`) sees it.
    pub(crate) fn sync_active_session(&mut self) {
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

    pub(crate) fn session_display_secs(&self) -> u64 {
        let mut secs = self.current_session_elapsed;
        if let Some(start) = self.current_session_start {
            secs += start.elapsed().as_secs();
        }
        secs
    }

    pub(crate) fn add_project(&mut self) {
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

    /// Starts tracking the current session.
    pub(crate) fn start_session(&mut self) {
        self.current_session_start = Some(Instant::now());
        self.is_tracking = true;
        self.save();
    }

    /// Pauses tracking, folding the live interval into the accumulated seconds.
    pub(crate) fn pause_session(&mut self) {
        if let Some(start) = self.current_session_start {
            self.current_session_elapsed += start.elapsed().as_secs();
        }
        self.current_session_start = None;
        self.is_tracking = false;
        self.save();
    }

    pub(crate) fn finish_session(&mut self) {
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

    /// Deletes a project, clearing the active session state if it belonged
    /// to the deleted project.
    pub(crate) fn delete_project(&mut self, project_id: &str) {
        self.data.projects.retain(|p| p.id != project_id);
        if self.active_project_id.as_deref() == Some(project_id) {
            self.active_project_id = None;
            self.active_session_name.clear();
            self.active_parent_session_id = None;
            self.current_session_start = None;
            self.current_session_elapsed = 0;
            self.is_tracking = false;
        }
        self.save();
    }

    /// Deletes a session from its project, clearing the "continue" state if
    /// the deleted session was the active parent.
    pub(crate) fn delete_session(&mut self, project_id: &str, session_id: &str) {
        if let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == project_id) {
            proj.sessions.retain(|s| s.id != session_id);
        }
        if self.active_parent_session_id.as_deref() == Some(session_id) {
            self.active_parent_session_id = None;
            self.active_session_name.clear();
        }
        self.save();
    }

    /// Marks a session as the active parent so `finish_session` appends a
    /// sub-session to it. Ignored while tracking.
    pub(crate) fn continue_session(&mut self, session_id: String, session_name: String) {
        if self.is_tracking {
            return;
        }
        self.active_parent_session_id = Some(session_id);
        self.active_session_name = session_name;
        self.save();
    }

    /// Renames a session. No-op if the project or session is missing.
    pub(crate) fn rename_session(&mut self, project_id: &str, session_id: &str, new_name: String) {
        if let Some(proj) = self.data.projects.iter_mut().find(|p| p.id == project_id)
            && let Some(sess) = proj.sessions.iter_mut().find(|s| s.id == session_id) {
            sess.name = new_name;
            self.save();
        }
    }

    pub(crate) fn export_project_to_file(&mut self, proj: &Project, file_path: &Path) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::DATA_PATH_LOCK;

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
    fn test_delete_project_clears_active_state() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let data_path = std::env::temp_dir().join("test_tracker_data_delete_project.json");
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();
        state.data.projects.push(Project { id: "p1".into(), name: "P".into(), sessions: vec![] });
        state.active_project_id = Some("p1".into());
        state.is_tracking = true;
        state.current_session_elapsed = 30;

        state.delete_project("p1");

        assert!(state.data.projects.is_empty());
        assert_eq!(state.active_project_id, None);
        assert!(!state.is_tracking);
        assert_eq!(state.current_session_elapsed, 0);
        assert!(state.current_session_start.is_none());

        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn test_continue_session_ignored_while_tracking() {
        let mut state = TimeTrackerState {
            is_tracking: true,
            ..Default::default()
        };

        state.continue_session("s1".into(), "S".into());

        assert_eq!(state.active_parent_session_id, None);
        assert!(state.active_session_name.is_empty());
    }

    #[test]
    fn test_pause_session_folds_elapsed() {
        let _guard = DATA_PATH_LOCK.lock().unwrap();
        let data_path = std::env::temp_dir().join("test_tracker_data_pause.json");
        unsafe {
            std::env::set_var("ULTRADIANT_DATA_PATH", &data_path);
        }
        let _ = std::fs::remove_file(&data_path);

        let mut state = TimeTrackerState::load();
        state.is_tracking = true;
        let start = Instant::now().checked_sub(std::time::Duration::from_secs(120)).unwrap();
        state.current_session_start = Some(start);
        state.current_session_elapsed = 30;

        state.pause_session();

        assert!(!state.is_tracking);
        assert!(state.current_session_start.is_none());
        assert!(state.current_session_elapsed >= 150);

        let _ = std::fs::remove_file(&data_path);
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
}
