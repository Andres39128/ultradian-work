# Ultradian Timer 🍅
Sistema de cronómetro para terminal en Linux diseñado específicamente para implementar **Ritmos Ultradianos**. 

Fomenta ciclos de trabajo profundo (Deep Work) sin interrupciones, seguidos de pausas de inactividad cognitiva absoluta (sin pantallas) para consolidación de memoria y recarga.

## Filosofía
- **Trabajo Profundo (90 mins):** Restringir ingreso de datos y mantener el foco máximo en tareas complejas.
- **Descanso Neurológico (15 mins):** Inactividad absoluta, cero ingreso de información, alejarse de las pantallas. La pantalla del terminal se vuelve negra y muestra únicamente un contador sutil.

## Instalación
Debes tener instalado `rust` y `cargo`.

1. Clona el repositorio:
   ```bash
   git clone https://github.com/tu-usuario/ultradian-timer.git
   cd ultradian-timer
   ```
2. Ejecuta el script de instalación local:
   ```bash
   ./install.sh
   ```
   *(Esto compilará el código y lo moverá a `~/.local/bin/ultradian-timer`).*

Opcionalmente, usando cargo:
```bash
cargo install --path .
```

## Uso
Una vez instalado, simplemente ejecuta:
```bash
ultradian-timer
```

Puedes personalizar los tiempos pasando argumentos:
```bash
ultradian-timer --work 90 --rest 15
```

### Controles
- `[Espacio]`: Pausar / Reanudar
- `[r]`: Reiniciar el bloque actual
- `[q] / [Esc]`: Salir de la aplicación

## Open Source (GitHub)
Este proyecto es robusto y se puede alojar en GitHub bajo licencia MIT.

```bash
git init
git add .
git commit -m "Initial commit"
# Y subir a tu repositorio de preferencia
```

## Licencia
MIT
