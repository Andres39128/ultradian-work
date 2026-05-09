use serde::{Deserialize, Serialize};
use crate::i18n::Language;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Priority {
    Alta,
    Media,
    Baja,
}
impl Default for Priority {
    fn default() -> Self { Priority::Media }
}

pub fn default_language() -> Language { Language::Es }

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
