# Instalación de Ultradian Work en Linux

## Requisitos

- **Rust/Cargo**: https://rustup.rs
- **Opcionales** (para funcionalidad completa):
  - `brightnessctl` - Control de brillo
  - `loginctl` - Bloqueo de sesión
  - `xdg-screensaver` - Alternativa para bloqueo

## Instalación rápida

```bash
# Clonar el repositorio
git clone https://github.com/Andres39128/ultradian-work.git
cd ultradian-work

# Instalar (usuario)
./install.sh

# O instalación de sistema (requiere sudo)
sudo ./install.sh --system
```

## Uso

```bash
# Ejecutar desde terminal
ultradian-work

# O desde el menú de aplicaciones
# Busca "Ultradian Work"
```

## Desinstalación

```bash
./install.sh --uninstall
```

## Desarrollo

```bash
# Compilar en modo desarrollo
cargo run

# Compilar release
cargo build --release

# Tests
cargo test

# Lint
cargo clippy
```

## Estructura de instalación

**Usuario** (`~/.local`):
- Binario: `~/.local/bin/ultradian-work`
- Desktop: `~/.local/share/applications/ultradian-work.desktop`
- Icono: `~/.local/share/icons/hicolor/scalable/apps/ultradian-work.svg`
- Datos: `~/.local/share/com.DevPersonal/UltradianTimer/tracker_data.json`

**Sistema** (`/usr/local`):
- Binario: `/usr/local/bin/ultradian-work`
- Desktop: `/usr/share/applications/ultradian-work.desktop`
- Icono: `/usr/share/icons/hicolor/scalable/apps/ultradian-work.svg`

## Solución de problemas

### `cargo` no encontrado
Instala Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### `~/.local/bin` no está en PATH
Agrega a `~/.bashrc` o `~/.zshrc`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Icono no aparece
Ejecuta: `update-desktop-database ~/.local/share/applications`

### Pantalla no se atenúa
Instala `brightnessctl` o `xset`:
```bash
sudo apt install brightnessctl x11-xserver-utils
```

## Licencia

MIT
