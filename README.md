# Ultradian Work 🍅

Aplicación de escritorio para Linux diseñada específicamente para implementar **Ritmos Ultradianos** y maximizar tu productividad. Desarrollada en Rust garantizando alto rendimiento y un bajo consumo de recursos.

Fomenta ciclos de trabajo profundo (Deep Work) sin interrupciones, seguidos de pausas de inactividad cognitiva absoluta (sin pantallas) para consolidación de memoria y recarga de energía.

## Características

- **Interfaz Gráfica Nativa:** Construida con `egui` (GUI rápida y ligera).
- **Ritmos Ultradianos:** 
  - *Trabajo Profundo:* Restringir ingreso de datos y mantener el foco máximo en tareas complejas (ej. 90 minutos).
  - *Descanso Neurológico:* Inactividad absoluta, cero ingreso de información, alejarse de las pantallas para recargar energías (ej. 15-20 minutos).
- **Integración con Linux:** Notificaciones nativas (`notify-rust`) y acceso directo a aplicaciones integrado (`.desktop`).
- **Seguimiento y Análisis:** Exportación e importación de reportes gracias a soporte para Excel/JSON (`calamine`, `rust_xlsxwriter`).

## Instalación

Asegúrate de tener instalados `rust` y `cargo` en tu sistema.

1. Clona el repositorio:
   ```bash
   git clone https://github.com/tu-usuario/ultradian-work.git
   cd ultradian-work
   ```

2. Ejecuta el script de instalación automática:
   ```bash
   ./install.sh
   ```
   *(Este script compilará el código en modo release, instalará el binario en `~/.local/bin/`, y configurará el icono y el acceso directo de la aplicación en tu entorno de escritorio).*

**Alternativa (usando Cargo):**
Puedes instalar únicamente el binario mediante:
```bash
cargo install --path .
```
*(Nota: Si usas este método, deberás configurar el archivo `.desktop` y el icono manualmente si deseas un acceso desde el menú de aplicaciones).*

## Uso

Una vez instalado mediante el script, puedes lanzar la aplicación de las siguientes formas:

- **Interfaz Gráfica:** Busca **"Ultradian Work"** en el menú principal/lanzador de aplicaciones de tu entorno de escritorio Linux.
- **Terminal:** Ejecuta el comando:
  ```bash
  ultradian-work
  ```

## Licencia

Este proyecto se distribuye bajo la licencia **MIT**. Consulta el archivo `LICENSE` para más detalles.
