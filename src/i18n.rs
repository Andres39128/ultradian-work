use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Language {
    En,
    #[default]
    Es,
}

const TRANSLATIONS: &[(&str, &str, &str)] = &[
    (
        "tab_ultradian",
        "🍅 Ultradian Timer",
        "🍅 Temporizador Ultradiano",
    ),
    ("ultradian_project", "Ultradian", "Ultradian"),
    ("tab_tracker", "⏱ Time Tracker", "⏱ Seguimiento"),
    ("tab_tasks", "📋 Tasks", "📋 Pendientes"),
    ("tab_dashboard", "📊 Dashboard", "📊 Dashboard"),
    ("tab_settings", "⚙ Settings", "⚙ Configuración"),
    ("ultradian_paused", "(PAUSED)", "(PAUSADO)"),
    ("ultradian_idle", "WAITING TO START", "ESPERANDO INICIO"),
    ("ultradian_work", "DEEP WORK", "TRABAJO PROFUNDO"),
    (
        "ultradian_help_start",
        "[Enter] or [Space] to Start",
        "[Enter] o [Espacio] para Iniciar",
    ),
    (
        "ultradian_help_pause",
        "[Space] Pause/Resume | [R] Restart phase",
        "[Espacio] Pausa/Reanudar | [R] Reiniciar fase",
    ),
    ("new_project", "New Project:", "Nuevo Proyecto:"),
    ("create", "Create", "Crear"),
    ("your_projects", "Your Projects", "Tus Proyectos"),
    (
        "export_excel",
        "📥 Export to Excel (.xlsx)",
        "📥 Exportar a Excel (.xlsx)",
    ),
    (
        "new_work_session",
        "New Work Session",
        "Nueva Sesión de Trabajo",
    ),
    ("name", "Name:", "Nombre:"),
    ("start", "▶ Start", "▶ Empezar"),
    ("pause", "⏸ Pause", "⏸ Pausar"),
    ("finish_save", "⏹ Finish & Save", "⏹ Finalizar y Guardar"),
    (
        "session_history",
        "Session History",
        "Historial de Sesiones",
    ),
    ("date_unix", "Date (Unix)", "Fecha (Unix)"),
    ("exported_to", "Exported to", "Exportado a"),
    ("error_export", "Error exporting", "Error al exportar"),
    (
        "continuing_session",
        "Continuing session:",
        "Continuando sesión:",
    ),
    ("session_label", "Session", "Sesión"),
    ("cycle_label", "Cycle", "Ciclo"),
    ("save", "Save", "Guardar"),
    ("cancel", "Cancel", "Cancelar"),
    ("continue", "Continue", "Continuar"),
    ("total_duration", "Total Duration:", "Duración Total:"),
    ("sub_sessions", "sub-sessions", "sub-sesiones"),
    (
        "delete_session_confirm",
        "Are you sure you want to delete this session?",
        "¿Estás seguro de que deseas eliminar esta sesión?",
    ),
    (
        "delete_project_confirm",
        "Are you sure you want to delete this project?",
        "¿Estás seguro de que deseas eliminar este proyecto?",
    ),
    ("yes_delete", "Yes, Delete", "Sí, Eliminar"),
    ("export_tasks", "📤 Export Tasks", "📤 Exportar Pendientes"),
    ("import_tasks", "📥 Import Tasks", "📥 Importar Pendientes"),
    (
        "error_open_file",
        "Error opening file",
        "Error al abrir archivo",
    ),
    (
        "sheet_not_found",
        "Sheet 'Pendientes' not found",
        "Hoja 'Pendientes' no encontrada",
    ),
    ("imported_tasks", "tasks imported", "tareas importadas"),
    ("excel_header_name", "Name", "Nombre"),
    ("excel_header_description", "Description", "Descripción"),
    ("excel_header_project", "Project", "Proyecto"),
    ("excel_header_completed", "Completed", "Completada"),
    ("excel_header_priority", "Priority", "Prioridad"),
    ("excel_header_tags", "Tags", "Etiquetas"),
    ("excel_header_deadline", "Deadline", "Fecha Límite"),
    ("new_task", "New Task", "Nuevo Pendiente"),
    ("priority", "Priority:", "Prioridad:"),
    (
        "tags_comma",
        "Tags (comma separated):",
        "Tags (coma separados):",
    ),
    ("deadline", "Deadline:", "Fecha límite:"),
    ("description", "Description:", "Descripción:"),
    ("create_task", "Create Task", "Crear Tarea"),
    ("todo_list", "To-Do List", "Lista de Pendientes"),
    ("high", "High", "Alta"),
    ("medium", "Medium", "Media"),
    ("low", "Low", "Baja"),
    (
        "dashboard_hours_project",
        "Dashboard - Hours per Project",
        "Dashboard - Horas por Proyecto",
    ),
    ("chart_projects", "Projects", "Proyectos"),
    ("settings", "Settings", "Configuración"),
    (
        "deep_work_minutes",
        "Deep Work Minutes:",
        "Minutos Trabajo Profundo:",
    ),
    ("rest_minutes", "Rest Minutes:", "Minutos Descanso:"),
    ("project_label", "Project:", "Proyecto:"),
    (
        "delete_confirm_title",
        "Confirm Deletion",
        "Confirmar Eliminación",
    ),
    (
        "empty_state_tracker",
        "Select a project on the left or create a new one.",
        "Selecciona un proyecto a la izquierda o crea uno nuevo.",
    ),
    (
        "project_optional",
        "Project (Optional):",
        "Proyecto (Opcional):",
    ),
    ("none", "None", "Ninguno"),
    ("work_on_this", "⏱ Work on this", "⏱ Trabajar en esto"),
    ("task_session_prefix", "Task:", "Tarea:"),
    ("notification_rest_title", "Rest Time", "Tiempo de Descanso"),
    ("notification_rest_body", "Time to rest!", "¡A descansar!"),
    ("notification_work_title", "Work Time", "Tiempo de Trabajo"),
    (
        "notification_work_body",
        "Back to work.",
        "De vuelta al trabajo.",
    ),
    (
        "empty_state_dashboard",
        "No projects yet. Go to Tracker to create one.",
        "No hay proyectos aún. Ve a Seguimiento para crear uno.",
    ),
    (
        "empty_state_tasks",
        "No tasks yet. Create one below.",
        "No hay pendientes. Crea uno nuevo abajo.",
    ),
    (
        "error_empty_name",
        "Name cannot be empty",
        "El nombre no puede estar vacío",
    ),
    ("clear_search", "✕", "✕"),
    ("edit_tooltip", "Edit", "Editar"),
    ("delete_tooltip", "Delete", "Eliminar"),
    ("hours_label", "Hours", "Horas"),
    ("today_total", "Today", "Hoy"),
    ("total_all_time", "Total", "Total"),
    ("total_sessions", "Sessions", "Sesiones"),
    ("skip_rest", "Skip Rest → Work", "Saltar Descanso → Trabajo"),
    (
        "skip_rest_shortcut",
        "Press [S] to skip",
        "Presiona [S] para saltar",
    ),
    ("hide_completed", "Hide completed", "Ocultar completadas"),
    ("settings_screen", "Screen", "Pantalla"),
    (
        "screen_dim_during_rest",
        "Dim screen during rest",
        "Reducir brillo durante descanso",
    ),
    (
        "screen_lock_during_rest",
        "Lock screen during rest",
        "Bloquear pantalla durante descanso",
    ),
    ("breathe_in", "Breathe in...", "Inhala..."),
    ("breathe_out", "Breathe out...", "Exhala..."),
    (
        "rest_message",
        "Rest your eyes. Move your body.",
        "Descansa la vista. Mueve el cuerpo.",
    ),
    ("screen_available", "Available", "Disponible"),
    ("screen_not_available", "Not available", "No disponible"),
    (
        "wayland_warning",
        "Wayland detected: install brightnessctl for dim support",
        "Wayland detectado: instala brightnessctl para atenuar pantalla",
    ),
];

pub fn t(lang: &Language, key: &str) -> &'static str {
    match TRANSLATIONS.iter().find(|(k, _, _)| *k == key) {
        Some((_, en, es)) => match lang {
            Language::En => en,
            Language::Es => es,
        },
        None => {
            #[cfg(debug_assertions)]
            eprintln!("[i18n] unknown key: {key}");
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_translations_present() {
        assert!(!TRANSLATIONS.is_empty());
        for (key, en, es) in TRANSLATIONS {
            assert!(!en.is_empty(), "Missing EN translation for {key}");
            assert!(!es.is_empty(), "Missing ES translation for {key}");
        }
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<&str> = TRANSLATIONS.iter().map(|(k, _, _)| *k).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "Duplicate keys in TRANSLATIONS");
    }

    #[test]
    fn unknown_key_returns_empty() {
        assert_eq!(t(&Language::En, "no_such_key"), "");
        assert_eq!(t(&Language::Es, "no_such_key"), "");
    }
}
