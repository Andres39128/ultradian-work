use serde::{Deserialize, Serialize};
use crate::i18n::Language;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default, Debug)]
pub enum Priority {
    Alta,
    #[default]
    Media,
    Baja,
}

pub fn default_language() -> Language { Language::Es }

#[derive(Serialize, Deserialize, Clone)]
pub struct TrackerData {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default = "default_language")]
    pub language: Language,
    #[serde(default)]
    pub work_duration_mins: u64,
    #[serde(default)]
    pub rest_duration_mins: u64,
    #[serde(default)]
    pub ultradian_cycles_completed: u32,
    #[serde(default)]
    pub last_session_date: String,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub screen_dim_during_rest: bool,
    #[serde(default)]
    pub screen_lock_during_rest: bool,
    #[serde(default)]
    pub active_session: Option<ActiveSession>,
}

impl Default for TrackerData {
    fn default() -> Self {
        Self {
            schema_version: 1,
            language: default_language(),
            work_duration_mins: 90,
            rest_duration_mins: 15,
            ultradian_cycles_completed: 0,
            last_session_date: String::new(),
            projects: Vec::new(),
            tasks: Vec::new(),
            screen_dim_during_rest: false,
            screen_lock_during_rest: false,
            active_session: None,
        }
    }
}

/// In-progress session (started but not saved yet). Persisted so it can be
/// restored after the app is closed or crashes.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ActiveSession {
    pub project_id: String,
    pub session_name: String,
    pub parent_session_id: Option<String>,
    pub is_tracking: bool,
    /// Unix timestamp of the current interval start; only valid while `is_tracking`.
    pub start_unix: u64,
    /// Seconds accumulated from completed intervals of the session.
    pub elapsed_secs: u64,
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

impl Session {
    pub fn total_duration(&self) -> u64 {
        let mut total = self.end_time.saturating_sub(self.start_time);
        for sub in &self.sub_sessions {
            total += sub.end_time.saturating_sub(sub.start_time);
        }
        total
    }
}

impl Project {
    pub fn total_duration(&self) -> u64 {
        self.sessions.iter().map(|s| s.total_duration()).sum()
    }

    pub fn today_duration_secs(&self) -> u64 {
        let now = chrono::Local::now();
        let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap().timestamp() as u64;
        let end_of_day = start_of_day + 86400;

        self.sessions.iter().map(|s| {
            let mut total = 0u64;
            if s.end_time > start_of_day && s.start_time < end_of_day {
                total += s.end_time.min(end_of_day).saturating_sub(s.start_time.max(start_of_day));
            }
            for sub in &s.sub_sessions {
                if sub.end_time > start_of_day && sub.start_time < end_of_day {
                    total += sub.end_time.min(end_of_day).saturating_sub(sub.start_time.max(start_of_day));
                }
            }
            total
        }).sum()
    }
}
