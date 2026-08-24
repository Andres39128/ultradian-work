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
| `cargo test` | **30/30 pasaron en 0.01s** (el test de pantalla ya es seguro gracias a `cfg!(test)`; los tests de datos/logica usan `ULTRADIANT_DATA_PATH`; tras el split de PROD-002, 6 tests nuevos corren la logica sin UI) |
| `cargo clippy --all-targets` | 6 warnings, todos en tests (`bool_assert_comparison` x5: task_logic.rs:223, tracker.rs:789,790,795,796; `field_reassign_with_default` x1: tracker.rs:716). Produccion limpia |
| Binario release | 29 MB (eframe 0.36, unstripped) |
| Duplicados en dep tree | Una sola copia de `egui 0.36.1` (eframe 0.36.1 + egui_plot 0.37.0 compatibles, verificado en `e533b4b`) |

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

**Resueltos el 2026-08-23:** N5, N6, N7, N8, N9 (ver "Priorizacion sugerida" para el detalle de cada fix).

---

## Estado de hallazgos de la auditoria 2026-08-18

| ID | Anterior | Estado actual | Evidencia |
|---|---|---|---|
| A1 | Panic por underflow al pausar | **ARREGLADO** | `main.rs:90-99`: pausa ya no resta; resume usa clamp + `saturating_sub` (`:103-108`, `:113-118`) |
| A2 | Fechas en UTC | **ARREGLADO** | `chrono::Local` en `main.rs:168`, `models.rs:104`, `tracker.rs:427` (`with_timezone(&Local)`) |
| A3 | Test apaga pantalla/bloquea sesion | **ARRELLADO** | Early-return `cfg!(test)` en `dim/restore/lock/unlock` (`screen.rs:89,132,173,200`); el test pasa seguro |
| A4 | Tests destruyen datos reales | **ARREGLADO** | `ULTRADIANT_DATA_PATH` respetada (`tracker.rs:41-44`); test apunta a temp (`:971-975`) |
| A5 | Sin unlock al terminar descanso | **ARREGLADO** | `unlock_screen()` existe (`screen.rs:199-220`) y se invoca en skip (`main.rs:152-154`), fin de rest (`:245-247`), reset de settings (`:320-322`) y Ctrl+C (`screen.rs:76`) |
| A6 | Roundtrip Excel roto | **ARREGLADO** (2026-08-23) | Errores de import visibles; completed acepta `true/si/1/yes`; prioridad case-insensitive (N9 resuelto); el parse es por posicion (los headers i18n de C1 no afectan el roundtrip; sheet "Pendientes" y valores Alta/Media/Baja canonicos por decision). Test nuevo `test_export_import_roundtrip_preserves_all_fields` verifica export→import con estado fresco: nombre, descripcion, proyecto (recreado por nombre), completed, prioridad, tags y deadline |
| B1 | Sesion en curso no persiste | **ARREGLADO** | `ActiveSession` (proyecto, nombre, parent, tracking, `start_unix`, acumulados) serializada en `TrackerData` (`models.rs`); `save()` sincroniza el estado en memoria antes de escribir y `load()` restaura plegando el tiempo transcurrido desde `start_unix`; se guarda en start/pause/finish/continue/work-on-task; refs colgantes se limpian al borrar proyecto/padre |
| B2 | Midnight en today_duration | **ARREGLADO** | Clipping de solapamiento para sesion padre y subs (`models.rs:110-116`) |
| B3 | Sin eliminar proyectos | **ARREGLADO** | `deleting_project_id` + confirm window (`tracker.rs:265-298`) — pero con el bug N1 |
| B4 | Doble trigger de hotkeys | **ARREGLADO** | `has_focus` guard en `main.rs:340-351` |
| B5 | Ultradian siempre al 1er proyecto | **ARRELLADO** | Usa `active_project_id` primero (`main.rs:173-177`). Quedan: nombre "Ultradian" hardcodeado (`:181`) y sesion descartada en silencio si el proyecto desaparece (`:187`, `if let` sin else) |
| B6 | Scan O(n²) en tareas | **ABIERTO** | `tracker.rs:601-602` (`find` por frame por id) + 2do `find` en `:649-650`; agravado por 10 clones de String por tarea por frame (`:603-610`) |
| C1 | Strings hardcodeados ES | **ARREGLADO** | 12 strings → 16 keys i18n (`date_unix`, `exported_to`, `error_export`, `error_open_file`, `sheet_not_found`, `imported_tasks`, 7x `excel_header_*`, `none`, `chart_projects`, `ultradian_project`). Quedan canónicos (no i18n) el nombre de sheet "Pendientes" y los valores "Alta/Media/Baja": el import los busca por literal, localizarlos rompería el roundtrip entre idiomas |
| C2 | 7 keys i18n muertas | **ARREGLADO** | Borradas `project`, `select_project`, `search_placeholder`, `screen_dim_available`, `screen_lock_available`, `ultradian_rest_title`, `ultradian_rest_desc`; el test de keys se amplió a cubrir las 16 nuevas |
| C3 | Key desconocida → "" | **ARREGLADO** (2026-08-23, junto a PROD-013) | sigue retornando `""` (degradacion elegante) pero loguea `[i18n] unknown key` en builds debug |
| C4 | Expresion total_secs x3 | **ABIERTO** | Duplicada en `tracker.rs:216-217` y `:422-423` aunque `Project::total_duration` existe (`models.rs:93-101`) |
| C5 | Bloque export_message x2 | **ABIERTO** | Identico en `tracker.rs:321-328` y `:523-530` |
| C6 | Tripleto Fullscreen/WindowLevel x3 | **ABIERTO** | `main.rs:158-159`, `:240-241`, `:323-324` → helper `exit_rest_viewport(ctx)` |
| C7 | Dead code en screen.rs | **ARREGLADO** (2026-08-23) | Borrados los wrappers pub muertos `get_saved_brightness`/`save_brightness` y sus `#[allow(dead_code)]`; `restore_screen` ahora usa `get_saved_brightness_from` (el helper paso a tener uso productivo); `save_brightness_to` ya era usado por `dim_screen` |
| C8 | Sentinel "handled" | **ARREGLADO** (2026-08-23, junto a PROD-002: el dance de `session_to_delete` se reemplazo por limpiar `deleting_session_id` directo al confirmar/cancelar) | `tracker.rs:486-490` |
| C9 | Tests de validacion falsos | **ARREGLADO** | Tests reescritos para llamar a `create_task()`/`add_project()` reales con `ULTRADIANT_DATA_PATH` inyectado; lock de mutex serializa los tests que mutan la env var (race detectado al agregarlos) |
| C10 | Nombre fijo test_export.xlsx | **ABIERTO** | `tracker.rs:970` (riesgo de colision entre runs) |
| C11 | Clippy en tests | **ABIERTO** | 6 warnings confirmados hoy (ver arriba) |
| C12 | Ruta fija /tmp para brillo | **ARREGLADO** (2026-08-23) | El nivel de brillo se guarda en la data dir del proyecto (`TimeTrackerState::data_dir()/brightness`, junto a `tracker_data.json`); `save_brightness_to` crea el directorio y reporta el error via tracing. Queda: archivo stale tras `kill -9` (no en el alcance de este fix) |
| C13 | Multi-monitor brightnessctl | **ARREGLADO** (2026-08-23) | `parse_brightness_output()` toma la primera linea no vacia de `brightnessctl g` y quita la anotacion de dispositivo (`74% [backlight]` → `74%`); `brightnessctl s` sin `-d` apunta al mismo dispositivo default. 5 tests nuevos |
| C14 | README con placeholder | **ARREGLADO** (2026-08-23) | URL real `https://github.com/Andres39128/ultradian-work`; secciones nuevas: atajos (Espacio/R/S), opciones CLI (`--work`/`--rest`), dim/lock con tools y deps opcionales |
| C15 | install.sh sin prerequisitos | **ARREGLADO** | `check_prerequisites` verifica cargo/rustc + avisa de deps opcionales (`install.sh:26-49`) |

**Resumen: 21 arreglados, 0 parciales, 6 abiertos** (de 27).

## Estado de hallazgos de la auditoria 2026-05-11 (PROD)

| ID | Estado |
|---|---|
| PROD-001 (JSON atomico) | ARREGLADO (sostiene: `tracker.rs:104-125` temp+rename) |
| PROD-002 (tracker monolitico) | **ARREGLADO** (2026-08-23): `session_logic.rs`/`task_logic.rs` con la logica pura (sin egui); `tracker.rs` quedo con estado + persistencia + render; 30 tests (los de logica corren sin UI) |
| PROD-003 (unwrap_or_default silencioso) | PARCIAL (distingue read/parse, loguea; sigue sin feedback en UI) |
| PROD-004 (scripts muertos) | ARREGLADO |
| PROD-005 (strings hardcodeados) | ARREGLADO (ver C1, resuelto 2026-08-23) |
| PROD-006 (schema version) | PARCIAL (campo + tests; sin migracion) |
| PROD-007 (pocos tests) | ARREGLADO: 24 tests, todos seguros de correr (C9 resuelto: los tests de validacion ahora llaman a los metodos reales) |
| PROD-008 (.gitignore) | ARREGLADO |
| PROD-009 (install.sh prereqs) | ARREGLADO (ver C15) |
| PROD-010 (sin logging estructurado) | **ARREGLADO** (2026-08-23): `tracing` + `tracing-subscriber` (feature `env-filter`); init en `main()` con default `RUST_LOG=warn`; los 11 `eprintln!` reemplazados por eventos estructurados (errores de save/load = `error!` con campos `path`+`error`, degradacion de pantalla/notificaciones = `warn!`, sonido y save exitoso = `debug!`, key i18n desconocida = `warn!` en builds debug) |
| PROD-011 (repaint continuo) | ARREGLADO |
| PROD-012 (paplay sin validacion) | ABIERTO (`main.rs:75` ruta hardcodeada, error ignorado) |
| PROD-013 (i18n no escalable) | **ARREGLADO** (2026-08-23): const `TRANSLATIONS: &[(&str, &str, &str)]` (key, en, es) + `t()` con busqueda lineal; API `t(&Language, &str)` sin cambios, agregar un idioma = ampliar el tuple |
| PROD-014 (.desktop duplicado) | ARREGLADO |

## Dependencias (Cargo.toml / Cargo.lock)

Actualizado desde la auditoria previa: `calamine` 0.34→**0.36.1**, `rust_xlsxwriter` 0.95→**0.98.2**, patches de chrono/clap/serde/serde_json/uuid/notify-rust, `ctrlc` agregado.

Actualizado 2026-08-23: `eframe` 0.33.3→**0.36.1** + `egui_plot` 0.34.1→**0.37.0** (commit dedicado `e533b4b`, con migracion de `App::update` a `logic`/`ui` de eframe 0.36).

| Crates | Actual | Disponible | Delta |
|---|---|---|---|
| `eframe` | 0.36.1 | 0.36.1 | **al dia** (migrado en `e533b4b`: `App::update` → `logic`/`ui`, `TopBottomPanel` → `Panel`, `ctx.style()` → `ctx.style_of(ctx.theme())`) |
| `egui_plot` | 0.37.0 | 0.37.0 | al dia (acompana a eframe; comparten una sola `egui 0.36.1` sin duplicados) |
| resto | — | — | al dia o solo patches |

`cargo audit` (0.22.2, 2026-08-23): **0 vulnerabilidades**, 1 warning — `ttf-parser 0.25.1` unmaintained (transitiva de egui, no accionable en este repo).

## Metricas de codigo

| Archivo | Lineas | % | Nota |
|---|---|---|---|
| `src/tracker.rs` | 836 | 29% | Estado + persistencia + solo render (UI); ~600 de produccion + 235 de tests |
| `src/session_logic.rs` | 372 | 13% | Logica pura de sesion/proyecto (sin egui) + tests |
| `src/task_logic.rs` | 386 | 13% | Logica pura de tareas/import/export (sin egui) + tests |
| `src/main.rs` | 523 | 22% | AppState + eframe App + UI ultradiana |
| `src/screen.rs` | 308 | 13% | Bien aislado; wrappers muertos (C7) |
| `src/i18n.rs` | 185 | 6% | Const tabla (key, en, es) + `t()` lineal; 3 tests (no-empty, keys unicas, unknown key) |
| `src/models.rs` | 121 | 5% | Limpio, con metodos de dominio (`total_duration`, `today_duration_secs`) |

Binario release: 31 MB con eframe 0.36 + tracing (eframe/wgpu; `RUSTFLAGS="-C strip=symbols"` lo recorta ~20-30%).

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
- ~~**N5/N6/N7/N8/N9**~~ — paquete de fixes pequenos — **resuelto** el 2026-08-23 (N5: unlock al pausar Rest / lock al reanudar y al restart; N6: la sesion logueada usa el trabajo real no pausado, no la configuracion; N7: clamp del spacer negativo; N8: defaults de serde = 90/15, consistentes con `Default`; N9: prioridad de import case-insensitive)
- ~~**C9**~~ — reescribir los 2 tests falsos para llamar a `create_task()`/`add_project()` reales — **resuelto** en `4e0498d` (data path inyectado + lock contra race de env var entre tests)
- ~~Actualizar `eframe`+`egui_plot` a 0.36/0.37 en commit dedicado; `cargo audit`~~ — **resuelto** en `e533b4b` (migracion `update`→`logic`/`ui` + `Panel` + `style_of`; `cargo audit`: 0 vulnerabilidades, 1 warning transitiva)

**Mediano plazo (deuda estructural):**
- ~~**PROD-002**~~ — partir `tracker.rs`: extraer logica de sesion/tareas a `session_logic.rs`/`task_logic.rs` (puras, sin egui) y dejar solo render en `tracker.rs`; habilita testear logica sin UI — **resuelto** el 2026-08-23 (30 tests; la persistencia `load`/`save` quedo en `tracker.rs` junto al estado; como efecto colateral se cerro C8: el sentinel `"handled"` desaparecio)
- ~~**PROD-013**~~ — i18n con const arrays en lugar de match gigante — **resuelto** el 2026-08-23: `TRANSLATIONS: &[(&str, &str, &str)]` (key, en, es); `t()` busca linealmente y mantiene la API; agregar idioma = ampliar el tuple; efecto colateral cerro C3 (debug log de key desconocida)
- ~~**PROD-010**~~ — logging con `tracing` (hoy `eprintln!`), especialmente errores de save — **resuelto** el 2026-08-23 (`tracing` + `tracing-subscriber` con env-filter; errores de save ahora `error!` con `path`+`error`; default `RUST_LOG=warn`)
- ~~**C12/C13**~~ — brightness en data dir del proyecto + parse por linea de `brightnessctl g` — **resuelto** el 2026-08-23 (`brightness` en la data dir junto a `tracker_data.json`; `parse_brightness_output()` con 5 tests)
- ~~**C14**~~ — README: URL real + seccion de shortcuts/dim/lock — **resuelto** el 2026-08-23 (URL `Andres39128/ultradian-work`; secciones de atajos, CLI y dim/lock)
