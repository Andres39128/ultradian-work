#!/usr/bin/env bash
set -e

echo "Compilando ultradian-timer en modo release..."
cargo build --release

BIN_PATH="target/release/ultradian-timer"
DEST_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

echo "Creando directorios necesarios..."
mkdir -p "$DEST_DIR"
mkdir -p "$APP_DIR"
mkdir -p "$ICON_DIR"

echo "Instalando binario en $DEST_DIR..."
cp "$BIN_PATH" "$DEST_DIR/"
chmod +x "$DEST_DIR/ultradian-timer"

echo "Instalando icono y acceso directo..."
cp assets/icon.svg "$ICON_DIR/ultradian-timer.svg"
cp assets/ultradian-timer.desktop "$APP_DIR/"
update-desktop-database "$HOME/.local/share/applications" || true

# Asegurar que está en el PATH temporalmente si no lo está (instrucción)
if [[ ":$PATH:" != *":$DEST_DIR:"* ]]; then
    echo "¡Atención! $DEST_DIR no está en tu PATH."
    echo "Agrega 'export PATH=\"\$HOME/.local/bin:\$PATH\"' a tu ~/.bashrc o ~/.zshrc"
fi

echo "¡Instalación exitosa! Busca 'Ultradian Timer' en el menú de aplicaciones de tu sistema."