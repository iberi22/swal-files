# SWAL Files — Benchmark Tecnológico, Inspiraciones y Patrones de Arquitectura

## 1. Resumen Ejecutivo
`swal-files` es el administrador de archivos agéntico y de alto rendimiento de **SouthWest AI Labs**. Se concibe como una aplicación nativa e independiente en **Rust**, acelerada por GPU a 200Hz+, fusionando la estética moderna de **files-community/Files**, la ergonomía de navegación de **macOS Finder (QuickLook & Column View)**, el motor asíncrono no bloqueante de **Yazi & COSMIC Files**, y una pestaña especializada de **Control de Cambios Git**.

---

## 2. Benchmark de Proyectos Líderes Analizados

| Proyecto | Lenguaje / Stack | Fortalezas Extraídas | Técnicas Clave para `swal-files` |
|---|---|---|---|
| **Files (files-community/Files)** | C# / WinUI 3 (Fluent) | • Diseño Mica Alt & Acrylic translucidez.<br>• Omnibar híbrida (Ruta + Búsqueda).<br>• Sistema de pestañas y Dual-Pane. | **Estética & Layout**: Replicación exacta del sistema de pestañas flotantes, migas de pan interactivas y barra lateral agrupada. |
| **macOS Finder (Apple)** | Objective-C / Swift | • **QuickLook (`Spacebar`)** para previsualización instantánea.<br>• **Miller Columns (Column View)**.<br>• Sistema de etiquetas por puntos de color. | **Ergonomía de Archivos**: Previsualizador emergente instantáneo con soporte para Markdown, imágenes, código con resaltado de sintaxis y metadatos. |
| **Yazi** | Rust (Tokio) | • Arquitectura asíncrona 100% no bloqueante.<br>• Pre-cargador concurrente en segundo plano.<br>• Integración `inotify` en tiempo real. | **Motor VFS de Alto Rendimiento**: Escaneo asíncrono con Tokio, watcher `notify` sin polling y caché de metadatos en memoria. |
| **COSMIC Files (System76)** | Rust (Iced + libcosmic) | • Renderizado acelerado por GPU vía `wgpu`.<br>• Patrón Elm (State / Message / View).<br>• Seguridad de memoria total en Wayland. | **Renderizado GPU a 200Hz+**: Pipeline gráfico con presupuesto de frame sub-5.0ms y 0% de uso de CPU en reposo. |
| **GitKraken / Magit** | C++ / Emacs Lisp | • Vista de estado de ramas, staged/unstaged y diffs lado a lado. | **Pestaña de Git Integrada**: Panel dedicado para inspeccionar diffs, ramas, commits y conflictos sin abrir herramientas externas. |

---

## 3. Patrones de Diseño y Técnicas Avanzadas Incorporadas

### 3.1 Renderizado Virtualizado de Listas y Cuadrículas
- En lugar de instanciar miles de widgets en el DOM, el motor solo dibuja los elementos visibles en el viewport más un buffer de 10 elementos.
- **Rendimiento**: Permite abrir directorios con más de 100,000 archivos a 0.00ms de latencia percibida.

### 3.2 Previsualizador QuickLook Multimodal (`Spacebar`)
- Al presionar la barra espaciadora sobre cualquier archivo:
  - **Código/Texto**: Renderizado con resaltado de sintaxis (`syntect`).
  - **Markdown**: Formateado en tiempo real con tablas y encabezados.
  - **Imágenes/SVGs**: Decodificación acelerada por hardware (`image-rs`).
  - **Audio/Video**: Metadatos ID3 y mini reproductor de onda.
  - **Resumen Agéntico**: Tarjeta con análisis generado por Xavier Cognitive Core (`:8006`).

### 3.3 Pestaña Especializada de Control de Cambios Git
- Una pestaña nativa dedicada que detecta automáticamente si el directorio actual es un repositorio Git:
  - Árbol de archivos modificados, staged y no rastreados (`Untracked`).
  - Visor de diferencias (*Side-by-Side Diff Viewer*) con resaltado de adiciones y eliminaciones.
  - Caja de Commit rápido con atajo de teclado (`Ctrl+Enter`).
  - Selector de ramas locales y remotas.

### 3.4 Omnibar & Paleta Agéntica Híbrida (`Ctrl+L` / `Ctrl+P`)
- Soporta 4 tipos de intenciones:
  1. `Navegación`: `/home/belal/proyectosSWAL` o `~/Downloads`.
  2. `Búsqueda Rápida`: `?nombre_archivo`.
  3. `Comandos Shell`: `>cargo build`.
  4. `Prompt Agéntico`: `@explica los cambios de este commit`.
