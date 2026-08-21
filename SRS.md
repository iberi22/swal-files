# SWAL Files — Software Requirements Specification (SRS)

## 1. Requisitos Funcionales (RF)

- **RF-01 (VFS Asíncrono)**: El sistema debe escanear directorios en hilos de Tokio sin bloquear el bucle de renderizado de la UI.
- **RF-02 (Watcher de Archivos)**: Monitoreo en tiempo real de eventos `inotify` (`create`, `modify`, `delete`, `rename`) actualizando el estado de la vista en <5ms.
- **RF-03 (Sistema Multi-Pestaña)**: Soporte para abrir, cerrar, renombrar y reordenar pestañas independientes (`Ctrl+T`, `Ctrl+W`, `Ctrl+Tab`).
- **RF-04 (Modo Dual-Pane)**: Posibilidad de dividir la ventana en dos paneles independientes (`F6`) para arrastrar y comparar carpetas.
- **RF-05 (QuickLook Finder)**: Al presionar `Spacebar`, debe abrirse una ventana modal flotante que previsualice el archivo seleccionado (Markdown renderizado, código resaltado con `syntect`, imágenes, audio o metadatos).
- **RF-06 (Miller Columns View)**: Vista de navegación jerárquica en columnas continuas idéntica a macOS Finder.
- **RF-07 (Omnibar & Command Palette)**: Barra unificada con soporte de migas de pan interactivas, búsqueda difusa (`?`), comandos de shell (`>`) y consultas a agentes (`@`).
- **RF-08 (Pestaña Especializada de Git)**: Panel dedicado para repositorios Git que muestre archivos modificados, diff viewer lado a lado, selector de ramas y botón de commit (`Ctrl+Enter`).
- **RF-09 (Integración con Xavier :8006)**: Cliente HTTP/WebSocket para consultar resúmenes de carpetas y buscar archivos por semántica.
- **RF-10 (Configuración Declarativa JSON)**: Persistencia en `~/.config/swal/files/config.json` con soporte para temas de `@swal/ui`.

## 2. Requisitos No Funcionales (RNF)

- **RNF-01 (Tasa de Refresco)**: La aplicación debe sostener **200Hz - 240Hz** con un presupuesto de frame sub-5.0ms en monitores de alta frecuencia.
- **RNF-02 (Uso de Recursos en Reposo)**: 0% de uso de CPU cuando no hay interacción del usuario o eventos de inotify.
- **RNF-03 (Escalabilidad de Archivos)**: Capacidad para abrir directorios con más de 100,000 archivos sin congelamiento mediante renderizado virtualizado.
- **RNF-04 (Compatibilidad Wayland)**: Soporte nativo para Hyprland y Niri con transparencias reales (Mica / Acrylic Glassmorphism).
