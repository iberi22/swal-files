# SWAL Files — Plan Maestro de Implementación (Ola 1: 15 Micro-Tareas)

## 1. Visión General
Esta ola (Wave 1) establece la base completa de la aplicación nativa en Rust:
- **Crate 1 (`swal-files-core`)**: Sistema de tipos, escáner asíncrono, watcher inotify, configuración JSON y pestañas.
- **Crate 2 (`swal-files-git`)**: Detección de repositorios, estado del working tree, parseador de diffs y operaciones de commit.
- **Crate 3 (`swal-files-preview`)**: Previsualizador QuickLook (Syntect, Markdown, Thumbnails de imágenes).
- **Crate 4 (`swal-files-agent`)**: Cliente REST/WebSocket para Xavier Core y etiquetador semántico.
- **Crate 5 (`swal-files-app`)**: Omnibar híbrida y punto de entrada de la UI.

---

## 2. Matriz de Islas de Archivos Disjuntas (0% Intersección)

| Issue # | Feature ID | Crate / Archivos Destino | Responsabilidad |
|---|---|---|---|
| **01** | `feat-files-01` | `crates/swal-files-core/src/types.rs` | Tipos y metadatos de FileEntry |
| **02** | `feat-files-02` | `crates/swal-files-core/src/scanner.rs` | Escáner asíncrono de directorios |
| **03** | `feat-files-03` | `crates/swal-files-core/src/watcher.rs` | Monitoreo inotify con notify |
| **04** | `feat-files-04` | `crates/swal-files-core/src/config.rs` | Configuración declarativa JSON |
| **05** | `feat-files-05` | `crates/swal-files-core/src/tabs.rs` | Gestión de pestañas y dual pane |
| **06** | `feat-files-06` | `crates/swal-files-git/src/detector.rs` | Detector de raíz Git y ramas |
| **07** | `feat-files-07` | `crates/swal-files-git/src/status.rs` | Lector de estado del working tree |
| **08** | `feat-files-08` | `crates/swal-files-git/src/diff.rs` | Parseador de diffs lado a lado |
| **09** | `feat-files-09` | `crates/swal-files-git/src/commit.rs` | Operador de stage y commit |
| **10** | `feat-files-10` | `crates/swal-files-preview/src/syntax.rs` | Resaltador de código Syntect |
| **11** | `feat-files-11` | `crates/swal-files-preview/src/markdown.rs` | Formateador Markdown QuickLook |
| **12** | `feat-files-12` | `crates/swal-files-preview/src/image.rs` | Generador de miniaturas de imagen |
| **13** | `feat-files-13` | `crates/swal-files-agent/src/client.rs` | Cliente Xavier Memory Core (:8006) |
| **14** | `feat-files-14` | `crates/swal-files-agent/src/tagger.rs` | Etiquetador semántico y búsqueda NL |
| **15** | `feat-files-15` | `crates/swal-files-app/src/omnibar.rs` | Omnibar y paleta de comandos |
