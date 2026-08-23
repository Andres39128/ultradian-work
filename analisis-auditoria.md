# analisis-auditoria.md

Fecha: 2026-08-23 · Reemplaza a la auditoria de 2026-08-18 (que a su vez reemplazaba a 2026-05-11).

## Calificaciones (resumen ejecutivo)

| Dimension | Nota | Justificacion |
|---|---|---|
| Arquitectura | 7/10 | Separacion por modulos clara (main/tracker/models/i18n/screen), maquina de estados explicita para el timer, persistencia con `schema_version`. Debe principal: `tracker.rs` es un objeto-dios (UI + logica + persistencia + tests en 1168 lineas); la logica no es testeable sin egui. |
| Redundancia | 5/10 | 7 keys i18n muertas, wrappers `#[allow(dead_code)]` en `screen.rs`, `has_xset` sin uso productivo, patron sentinel `"handled"`, `#[allow(dead_code)]` sobre el struct completo que enmascara uso real. |
| Duplicidad | 5/10 | Bloque `export_message` x2 identico, expresion `total_secs` x2 (aunque `Project::total_duration` existe), tripleto `Fullscreen(false)+WindowLevel(Normal)` x3, dos windows de confirmacion casi identicas (proyecto/sesion). |
| Coherencia | 6/10 | i18n completada el 2026-08-23 (antes quedaban 12 strings hardcodeados: Excel, errores, "Ninguno" — ver C1); valores canonicos de Excel en espanol (intencional, ver C1); `Default` (90/15 min) vs `serde(default)` (0) con semantica contradictoria; manejo de errores consistente (`eprintln!`, degradacion elegante). |
| Verbosidad | 6/10 | 10 clones de String por tarea por frame en el loop de tareas, `load()` con inicializacion explicita de 24 campos (acotable), match i18n de ~180 brazos (inherente al enfoque elegido). |
| Tamano | 7/10 | 2333 lineas para el feature set es razonable; el desbalance es `tracker.rs` (50% del codigo). Binario release 20 MB (tipico de eframe/wgpu; `strip` podria recortarlo). |
| Optimizacion | 5/10 | Scan O(n²) por frame en tareas, clone profundo de todo el data en Dashboard por frame, sin repaint periodico mientras se trackea (display se congela), save atomico OK, repaint a 1s OK. |
| **Global** | **6.5/10** | v0.1 solida: los 6 criticos/altos de la auditoria anterior quedaron arreglados; B1 (persistencia de sesion en curso) resuelto el 2026-08-23. Lo que queda es deuda mediana/baja estructural. |

## Metodologia

Lectura completa de las 5 fuentes (2333/2333 lineas), `Cargo.toml`/`Cargo.lock` (dependencias), `install.sh`, `README.md`, `Makefile`, `assets/`. Cada hallazgo se verifico contra el codigo actual con su ubicacion exacta. La auditoria previa (2026-08-18) se re-evaluó item por item contra el working tree actual, que tenia cambios sin commit posteriores a esa fecha (main.rs, i18n.rs, tracker.rs, screen.rs nuevo).

## Verificacion automatica (ejecutada 2026-08-23)

| Check | Resultado |
|---|---|
| `cargo check --all-targets` | OK, sin errores |
| `cargo test` | **18/18 pasaron en 0.01s** (el test de pantalla ya es seguro gracias a `cfg!(test)`; el test de datos usa `ULTRADIANT_DATA_PATH`) |
| `cargo clippy --all-targets` | 6 warnings, todos en tests (`bool_assert_comparison` x5: tracker.rs:1005,1139,1140,1145,1146; `field_reassign_with_default` x1: tracker.rs:1013). Produccion limpia |
| Binario release | 20 MB |
| Duplicados en dep tree | Una sola copia de `egui 0.33.3` (eframe 0.33.3 + egui_plot 0.34.1 compatibles) |

**Nota:** `cargo test` ya es seguro de correr (arreglos A3/A4 verificados).

---

## Hallazgos nuevos

### Altos

#### N1 · Dialogo de borrado de proyecto pregunta por "sesion"
- **Ubicacion:** `tracker.rs:271-298` (bloque `deleting_project_id`), label en `:278`
- **Evidencia:** el confirm de borrado de **proyecto** usa la key `delete_session_confirm` ("¿Estás seguro de que deseas eliminar esta sesión?"). Copy-paste del bloque de sesiones.
- **Impacto:** en una operacion destructiva el texto no coincide con lo que se va a borrar.
- **Remediacion:** key nueva `delete_project_confirm` en i18n (2 lineas).

### Medios

#### N2 · Dashboard clona todo el data cada frame
- **Ubicacion:** `main.rs:309` (`self.tracker.data.projects.clone()`), `tracker.rs:817` (`ui_dashboard(&mut self, ...)`)
- **Evidencia:** `ui_dashboard` no muta nada (solo lee), pero la firma `&mut self` fuerza clonar todos los proyectos + sesiones **cada frame** en la vista Dashboard.
- **Remediacion:** cambiar a `ui_dashboard(&self, ...)` y pasar `&self.tracker.data.projects` sin clonar.

#### N3 · El timer manual del tracker se congela cuando la app esta quieta
- **Ubicacion:** `main.rs:264-275` (repaint solo si ultradian esta en Work/Rest o es Rest), `tracker.rs:144-156` (`toggle_tracking` minimiza la ventana)
- **Evidencia:** con una sesion manual trackeando y el ultradiano Idle/Pausado, no hay `request_repaint_after` periodico: el display HH:MM:SS solo se actualiza con eventos de puntero. Como la app se minimiza al arrancar tracking, al volver el display salta en lugar de avanzar.
- **Impacto:** UX enganosa (el usuario ve un tiempo "atrás"). Los datos guardados son correctos (se calculan desde `Instant`).
- **Remediacion:** `ctx.request_repaint_after(1s)` mientras `is_tracking`.

#### N4 · Reiniciar una fase de Rest activa corrompe el brillo guardado
- **Ubicacion:** `main.rs:135-143` (`ultradian_restart_phase`, rama Rest llama a `dim_screen()`), `screen.rs:93-101`
- **Evidencia:** si ya esta a 5%, `dim_screen()` vuelve a leer `brightnessctl g` (5%), guarda "5%" como nivel original y apaga a 5%. Al terminar, `restore_screen()` restaura 5% en vez del 80% real.
- **Remediacion:** hacer `dim_screen()` idempotente (si existe el archivo de save, no re-leer) o no re-dim en restart si ya esta dimmed.

### Bajos

| ID | Tipo | Ubicacion | Detalle |
|---|---|---|---|
| N5 | estado | `main.rs:94-100` (pausa Rest) y `:111-123` (resume) | Pausar un Rest bloqueado no desbloquea; reanudar no re-bloquea. Tras pausa+resume el descanso queda desbloqueado a pesar del setting activo |
| N6 | semantica | `main.rs:167-170` | `log_ultradian_session` registra siempre `work_duration_mins` como duracion, aunque la fase se haya pausado: la sesion logueada es mas larga que el trabajo real |
| N7 | edge case | `main.rs:463` | `ui.add_space(ui.available_height() / 2.0 - 160.0)` queda negativo con ventana minima (300px) → layout roto |
| N8 | coherencia | `models.rs:20-23` vs `:38-52`; test `tracker.rs:1067` | `serde(default)` de campos faltantes = 0, pero `Default` = 90/15. `AppState::new` parchea 0→CLI. Dos "defaults" distintos; el test fija el comportamiento 0 |
| N9 | import | `tracker.rs:752-756` | Prioridad case-sensitive: "High"/"Medium"/"Low" (capitalizadas) caen al `_ => Media` y se importan como Media sin aviso; el catch-all traga typos |
| N10 | UI | `tracker.rs:271-298` vs `:462-491` | Pueden abrirse los dos dialogs de confirmacion a la vez, ambos con titulo `delete_confirm_title` → colision de IDs en egui |
| N11 | smell | `screen.rs:7,14` | `#[allow(dead_code)]` sobre el struct e impl completa; solo `has_xset` y los wrappers de brightness son muertos de verdad |
| N12 | i18n | `i18n.rs:25` | "Cero ingresos cognitivos" — "ingresos" = income; "entradas de informacion" seria correcto (key muerta de todos modos) |
| N13 | tests | `tracker.rs:973-975` | `std::env::set_var` (global de proceso, unsafe) en un test que corre en paralelo con los demas; hoy inofensivo (es el unico que usa el data path) pero fragil si se agregan tests que llamen `load()` |

---

## Estado de hallazgos de la auditoria 2026-08-18

| ID | Anterior | Estado actual | Evidencia |
|---|---|---|---|
| A1 | Panic por underflow al pausar | **ARREGLADO** | `main.rs:90-99`: pausa ya no resta; resume usa clamp + `saturating_sub` (`:103-108`, `:113-118`) |
| A2 | Fechas en UTC | **ARREGLADO** | `chrono::Local` en `main.rs:168`, `models.rs:104`, `tracker.rs:427` (`with_timezone(&Local)`) |
| A3 | Test apaga pantalla/bloquea sesion | **ARRELLADO** | Early-return `cfg!(test)` en `dim/restore/lock/unlock` (`screen.rs:89,132,173,200`); el test pasa seguro |
| A4 | Tests destruyen datos reales | **ARREGLADO** | `ULTRADIANT_DATA_PATH` respetada (`tracker.rs:41-44`); test apunta a temp (`:971-975`) |
| A5 | Sin unlock al terminar descanso | **ARREGLADO** | `unlock_screen()` existe (`screen.rs:199-220`) y se invoca en skip (`main.rs:152-154`), fin de rest (`:245-247`), reset de settings (`:320-322`) y Ctrl+C (`screen.rs:76`) |
| A6 | Roundtrip Excel roto | **PARCIAL** | Errores de import ahora visibles (`tracker.rs:707-716`); completed acepta `true/si/1/yes` (`:750`); roundtrip Alta/Media/Baja consistente. Queda N9 (case-sensitivity) y headers en espanol |
| B1 | Sesion en curso no persiste | **ARREGLADO** | `ActiveSession` (proyecto, nombre, parent, tracking, `start_unix`, acumulados) serializada en `TrackerData` (`models.rs`); `save()` sincroniza el estado en memoria antes de escribir y `load()` restaura plegando el tiempo transcurrido desde `start_unix`; se guarda en start/pause/finish/continue/work-on-task; refs colgantes se limpian al borrar proyecto/padre |
| B2 | Midnight en today_duration | **ARREGLADO** | Clipping de solapamiento para sesion padre y subs (`models.rs:110-116`) |
| B3 | Sin eliminar proyectos | **ARREGLADO** | `deleting_project_id` + confirm window (`tracker.rs:265-298`) — pero con el bug N1 |
| B4 | Doble trigger de hotkeys | **ARREGLADO** | `has_focus` guard en `main.rs:340-351` |
| B5 | Ultradian siempre al 1er proyecto | **ARRELLADO** | Usa `active_project_id` primero (`main.rs:173-177`). Quedan: nombre "Ultradian" hardcodeado (`:181`) y sesion descartada en silencio si el proyecto desaparece (`:187`, `if let` sin else) |
| B6 | Scan O(n²) en tareas | **ABIERTO** | `tracker.rs:601-602` (`find` por frame por id) + 2do `find` en `:649-650`; agravado por 10 clones de String por tarea por frame (`:603-610`) |
| C1 | Strings hardcodeados ES | **ARREGLADO** | 12 strings → 16 keys i18n (`date_unix`, `exported_to`, `error_export`, `error_open_file`, `sheet_not_found`, `imported_tasks`, 7x `excel_header_*`, `none`, `chart_projects`, `ultradian_project`). Quedan canónicos (no i18n) el nombre de sheet "Pendientes" y los valores "Alta/Media/Baja": el import los busca por literal, localizarlos rompería el roundtrip entre idiomas |
| C2 | 7 keys i18n muertas | **ARREGLADO** | Borradas `project`, `select_project`, `search_placeholder`, `screen_dim_available`, `screen_lock_available`, `ultradian_rest_title`, `ultradian_rest_desc`; el test de keys se amplió a cubrir las 16 nuevas |
| C3 | Key desconocida → "" | **ABIERTO** | `i18n.rs:183` (`_ => ""`) sin log en debug |
| C4 | Expresion total_secs x3 | **ABIERTO** | Duplicada en `tracker.rs:216-217` y `:422-423` aunque `Project::total_duration` existe (`models.rs:93-101`) |
| C5 | Bloque export_message x2 | **ABIERTO** | Identico en `tracker.rs:321-328` y `:523-530` |
| C6 | Tripleto Fullscreen/WindowLevel x3 | **ABIERTO** | `main.rs:158-159`, `:240-241`, `:323-324` → helper `exit_rest_viewport(ctx)` |
| C7 | Dead code en screen.rs | **PARCIAL** | Wrappers `get_saved_brightness`/`save_brightness` siguen muertos (`screen.rs:224-232`); los tests usan las funciones privadas directamente |
| C8 | Sentinel "handled" | **ABIERTO** | `tracker.rs:486-490` |
| C9 | Tests de validacion falsos | **ABIERTO** | `test_create_task_validates_name` / `test_create_project_validates_name` prueban copias locales, no `create_task()`/`add_project()` |
| C10 | Nombre fijo test_export.xlsx | **ABIERTO** | `tracker.rs:970` (riesgo de colision entre runs) |
| C11 | Clippy en tests | **ABIERTO** | 6 warnings confirmados hoy (ver arriba) |
| C12 | Ruta fija /tmp para brillo | **ABIERTO** | `screen.rs:5` (symlink-able; archivo stale si crash con `kill -9` impide restore en el siguiente arranque, ver N13-contexto) |
| C13 | Multi-monitor brightnessctl | **ABIERTO** | `screen.rs:98` toma el stdout completo (2 lineas con 2 monitores) → restore falla |
| C14 | README con placeholder | **ABIERTO** | `README.md:22` sigue con `tu-usuario`; tampoco documenta dim/lock ni shortcuts |
| C15 | install.sh sin prerequisitos | **ARREGLADO** | `check_prerequisites` verifica cargo/rustc + avisa de deps opcionales (`install.sh:26-49`) |

**Resumen: 14 arreglados, 3 parciales, 8 abiertos** (de 25).

## Estado de hallazgos de la auditoria 2026-05-11 (PROD)

| ID | Estado |
|---|---|
| PROD-001 (JSON atomico) | ARREGLADO (sostiene: `tracker.rs:104-125` temp+rename) |
| PROD-002 (tracker monolitico) | **REGRESADO**: crecio de 1112 a 1168 lineas; la extraccion de screen.rs no llego a tracker |
| PROD-003 (unwrap_or_default silencioso) | PARCIAL (distingue read/parse, loguea; sigue sin feedback en UI) |
| PROD-004 (scripts muertos) | ARREGLADO |
| PROD-005 (strings hardcodeados) | ARREGLADO (ver C1, resuelto 2026-08-23) |
| PROD-006 (schema version) | PARCIAL (campo + tests; sin migracion) |
| PROD-007 (pocos tests) | MEJORADO: 18 tests, todos seguros de correr; quedan 2 falsos (C9) |
| PROD-008 (.gitignore) | ARREGLADO |
| PROD-009 (install.sh prereqs) | ARREGLADO (ver C15) |
| PROD-010 (sin logging estructurado) | ABIERTO (solo `eprintln!`) |
| PROD-011 (repaint continuo) | ARREGLADO |
| PROD-012 (paplay sin validacion) | ABIERTO (`main.rs:75` ruta hardcodeada, error ignorado) |
| PROD-013 (i18n no escalable) | ABIERTO (match de ~180 brazos; 2 idiomas hoy, no escala a un 3ro) |
| PROD-014 (.desktop duplicado) | ARREGLADO |

## Dependencias (Cargo.toml / Cargo.lock)

Actualizado desde la auditoria previa: `calamine` 0.34→**0.36.1**, `rust_xlsxwriter` 0.95→**0.98.2**, patches de chrono/clap/serde/serde_json/uuid/notify-rust, `ctrlc` agregado.

| Crates | Actual | Disponible | Delta |
|---|---|---|---|
| `eframe` | 0.33.3 | 0.36.1 | **3 minors atras** (el unico salto con riesgo breaking, viewport APIs) |
| `egui_plot` | 0.34.1 | 0.37.0 | 3 minors (acompana a eframe; hoy comparten egui 0.33.3 sin duplicados) |
| resto | — | — | al dia o solo patches |

`cargo audit` no se pudo ejecutar (no instalado): recomendable antes de release.

## Metricas de codigo

| Archivo | Lineas | % | Nota |
|---|---|---|---|
| `src/tracker.rs` | 1168 | 50% | ~920 de produccion (UI+logica+persistence) + 245 de tests. El objeto-dios del repo |
| `src/main.rs` | 523 | 22% | AppState + eframe App + UI ultradiana |
| `src/screen.rs` | 308 | 13% | Bien aislado; wrappers muertos (C7) |
| `src/i18n.rs` | 213 | 9% | ~180 brazos de match; 7 muertas |
| `src/models.rs` | 121 | 5% | Limpio, con metodos de dominio (`total_duration`, `today_duration_secs`) |

Binario release: 20 MB (eframe/wgpu; `RUSTFLAGS="-C strip=symbols"` lo recorta ~20-30%).

## Priorizacion sugerida

**Rapidos (cada uno < 30 min de trabajo):**
1. **N1** — key i18n `delete_project_confirm` (1 linea + 2 traducciones)
2. **N3** — `request_repaint_after` mientras `is_tracking` (2 lineas)
3. **N4** — `dim_screen()` idempotente (check del archivo de save)
4. **N2** — `ui_dashboard(&self)` y borrar el clone (3 lineas)

**Corto plazo (riesgo de datos / calidad):**
- **C4/C5/C6** — 3 deduplicaciones con helper (una tarde)
- ~~**B1**~~ — persistir sesion en curso — **resuelto** (ver tabla de estado)
- ~~**C1 + C2**~~ — i18n de los 12 strings restantes y borrar 7 keys muertas — **resuelto** en `576ae29` (mismo commit)
- **N5/N6/N7/N8/N9** — paquete de fixes pequenos
- **C9** — reescribir los 2 tests falsos para llamar a `create_task()`/`add_project()` reales (inyectando el data path, que ya existe)
- Actualizar `eframe`+`egui_plot` a 0.36/0.37 en commit dedicado; `cargo audit`

**Mediano plazo (deuda estructural):**
- **PROD-002** — partir `tracker.rs`: extraer logica de sesion/tareas a `session_logic.rs`/`task_logic.rs` (puras, sin egui) y dejar solo render en `tracker.rs`; habilita testear logica sin UI
- **PROD-013** — i18n con tablas `&'static [(Key, &str, &str)]` o const arrays en lugar de match gigante
- **PROD-010** — logging con `tracing` (hoy `eprintln!`), especialmente errores de save
- **C12/C13** — brightness en data dir del proyecto + parse por linea de `brightnessctl g`
- **C14** — README: URL real + seccion de shortcuts/dim/lock
