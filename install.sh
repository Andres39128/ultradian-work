#!/usr/bin/env bash
set -euo pipefail

# Ultradian Work - Linux Installer
# Uso: ./install.sh [--system] [--uninstall]

VERSION="0.1.0"
REPO_URL="https://github.com/Andres39128/ultradian-work.git"
APP_NAME="ultradian-work"
BINARY_NAME="ultradian-work"
DESKTOP_FILE="ultradian-work.desktop"
ICON_FILE="assets/icon.svg"

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() { echo -e "${BLUE}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }

check_prerequisites() {
    log "Verificando prerequisitos..."
    
    if ! command -v cargo &> /dev/null; then
        error "cargo no está instalado. Instala Rust desde https://rustup.rs"
    fi
    
    if ! command -v rustc &> /dev/null; then
        error "rustc no está instalado. Instala Rust desde https://rustup.rs"
    fi
    
    # Verificar dependencias opcionales
    local missing_deps=()
    command -v brightnessctl &> /dev/null || missing_deps+=("brightnessctl")
    command -v loginctl &> /dev/null || missing_deps+=("loginctl")
    command -v xdg-screensaver &> /dev/null || missing_deps+=("xdg-screensaver")
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        warn "Dependencias opcionales faltantes: ${missing_deps[*]}"
        warn "La funcionalidad de dim/ lock de pantalla puede no funcionar correctamente"
    fi
    
    success "Prerequisitos verificados"
}

get_install_dirs() {
    if [ "${SYSTEM_INSTALL:-false}" = true ]; then
        if [ "$EUID" -ne 0 ]; then
            error "Instalación de sistema requiere privilegios de root (sudo)"
        fi
        DEST_DIR="/usr/local/bin"
        APP_DIR="/usr/share/applications"
        ICON_DIR="/usr/share/icons/hicolor/scalable/apps"
        DATA_DIR="/usr/share/$APP_NAME"
    else
        DEST_DIR="$HOME/.local/bin"
        APP_DIR="$HOME/.local/share/applications"
        ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
        DATA_DIR="$HOME/.local/share/$APP_NAME"
    fi
}

build_binary() {
    log "Compilando $APP_NAME en modo release..."
    cargo build --release
    success "Compilación completada"
}

install_binary() {
    log "Instalando binario en $DEST_DIR..."
    mkdir -p "$DEST_DIR"
    cp "target/release/$BINARY_NAME" "$DEST_DIR/"
    chmod +x "$DEST_DIR/$BINARY_NAME"
    success "Binario instalado"
}

install_desktop_file() {
    log "Instalando archivo desktop..."
    mkdir -p "$APP_DIR"
    
    # Crear desktop file con ruta correcta
    cat > "$APP_DIR/$DESKTOP_FILE" << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Ultradian Work
Comment=Time tracker con ritmos ultradianos
Exec=$DEST_DIR/$BINARY_NAME
Icon=$APP_NAME
Terminal=false
Categories=Office;Productivity;
StartupNotify=true
EOF
    
    success "Archivo desktop instalado"
}

install_icon() {
    if [ -f "$ICON_FILE" ]; then
        log "Instalando icono..."
        mkdir -p "$ICON_DIR"
        cp "$ICON_FILE" "$ICON_DIR/$APP_NAME.svg"
        success "Icono instalado"
    else
        warn "Icono no encontrado en $ICON_FILE"
    fi
}

update_desktop_db() {
    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database "$APP_DIR" 2>/dev/null || true
    fi
}

check_path() {
    if [[ ":$PATH:" != *":$DEST_DIR:"* ]]; then
        warn "$DEST_DIR no está en tu PATH"
        echo "Agrega esta línea a tu ~/.bashrc o ~/.zshrc:"
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
        echo "Luego ejecuta: source ~/.bashrc"
    fi
}

uninstall() {
    log "Desinstalando $APP_NAME..."
    
    # Binario
    if [ -f "$DEST_DIR/$BINARY_NAME" ]; then
        rm -f "$DEST_DIR/$BINARY_NAME"
        success "Binario eliminado"
    fi
    
    # Desktop file
    if [ -f "$APP_DIR/$DESKTOP_FILE" ]; then
        rm -f "$APP_DIR/$DESKTOP_FILE"
        success "Archivo desktop eliminado"
    fi
    
    # Icono
    if [ -f "$ICON_DIR/$APP_NAME.svg" ]; then
        rm -f "$ICON_DIR/$APP_NAME.svg"
        success "Icono eliminado"
    fi
    
    # Datos de usuario
    if [ -d "$HOME/.local/share/com.DevPersonal/UltradianTimer" ]; then
        warn "Datos de usuario conservados en ~/.local/share/com.DevPersonal/UltradianTimer"
        warn "Elimínalos manualmente si quieres borrar todo"
    fi
    
    update_desktop_db
    success "Desinstalación completada"
}

show_help() {
    cat << EOF
Ultradian Work - Instalador Linux

Uso: $0 [OPCIONES]

Opciones:
  --system       Instalar en /usr/local (requiere sudo)
  --uninstall    Desinstalar la aplicación
  --help         Mostrar esta ayuda

Ejemplos:
  $0                    # Instalación usuario (~/.local)
  $0 --system           # Instalación sistema (/usr/local)
  $0 --uninstall        # Desinstalar

Requisitos:
  - Rust/Cargo instalados
  - Opcionales: brightnessctl, loginctl, xdg-screensaver
EOF
}

main() {
    case "${1:-}" in
        --help|-h)
            show_help
            exit 0
            ;;
        --uninstall)
            get_install_dirs
            uninstall
            exit 0
            ;;
        --system)
            SYSTEM_INSTALL=true
            shift
            ;;
        "")
            SYSTEM_INSTALL=false
            ;;
        *)
            error "Opción desconocida: $1. Usa --help para ver opciones."
            ;;
    esac
    
    get_install_dirs
    check_prerequisites
    build_binary
    install_binary
    install_desktop_file
    install_icon
    update_desktop_db
    check_path
    
    echo ""
    success "¡Instalación completada!"
    echo ""
    echo "Para ejecutar: ultradian-work"
    echo "Para abrir desde el menú: busca 'Ultradian Work'"
    echo ""
    if [ "$SYSTEM_INSTALL" = true ]; then
        echo "Instalación de sistema completada en $DEST_DIR"
    else
        echo "Instalación de usuario completada en $DEST_DIR"
    fi
}

main "$@"
