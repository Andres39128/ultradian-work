#!/usr/bin/env bash
set -e

echo "Compilando ultradian-timer en modo release..."
cargo build --release

BIN_PATH="target/release/ultradian-timer"
DEST_DIR="$HOME/.local/bin"

if [ ! -d "$DEST_DIR" ]; then
    echo "Creando directorio $DEST_DIR"
    mkdir -p "$DEST_DIR"
fi

echo "Instalando binario en $DEST_DIR..."
cp "$BIN_PATH" "$DEST_DIR/"
chmod +x "$DEST_DIR/ultradian-timer"

# Asegurar que está en el PATH temporalmente si no lo está (instrucción)
if [[ ":$PATH:" != *":$DEST_DIR:"* ]]; then
    echo "¡Atención! $DEST_DIR no está en tu PATH."
    echo "Agrega 'export PATH=\"\$HOME/.local/bin:\$PATH\"' a tu ~/.bashrc o ~/.zshrc"
fi

echo "¡Instalación exitosa! Ejecuta 'ultradian-timer' para iniciar."