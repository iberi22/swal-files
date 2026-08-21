# ⚡ SWAL Files: Modern Minimalist Agentic File Manager

### Native Rust + GPU-Accelerated (200Hz+) + Fluent Mica Translucency + macOS QuickLook + Dedicated Git Tab

<p align="center">
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/Rust-2021_Edition-DEA584?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/>
  </a>
  <a href="https://wgpu.rs">
    <img src="https://img.shields.io/badge/Render-GPU_wgpu_200Hz-00ff88?style=for-the-badge" alt="Render"/>
  </a>
  <a href="https://github.com/files-community/Files">
    <img src="https://img.shields.io/badge/Design-Fluent_Mica_Acrylic-06b6d4?style=for-the-badge" alt="Design"/>
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/github/license/iberi22/swal-files?style=for-the-badge&color=f97316" alt="License"/>
  </a>
</p>

---

> **"⚡ High-Refresh File Exploration Meets Autonomous AI Agents"** — SouthWest AI Labs

**SWAL Files** is a standalone, ultra-fast file manager written in pure **Rust**, designed for the **SWAL Desktop Ecosystem**. It combines the Fluent Mica/Acrylic visual aesthetics of **files-community/Files**, the instant file previewing (**QuickLook**) and **Miller Columns** of **macOS Finder**, an asynchronous non-blocking VFS with `inotify`, and a dedicated **Git Change Control & Diff Tab**.

---

## 🚀 Key Features

- **⚡ 200Hz+ GPU-Accelerated Rendering**: Sub-5.0ms frame budgeting with virtualized scroll lists capable of rendering 100,000+ files without UI lag.
- **🎨 Fluent & Mica Acrylic Glassmorphism**: Translucent frosted surfaces, glowing hover rings, and `@swal/ui` design tokens (`Hive Dark`, `Cyber Neon`, `Nord Frost`).
- **📌 Multi-Tab & Dual-Pane**: Browser-style draggable tabs, tab tear-off, and side-by-side dual pane view (`F6`).
- **🔍 QuickLook Multimodal Previews (`Spacebar`)**: Instant modal previews for Markdown, source code with syntax highlighting, images, SVGs, audio, and Xavier AI summaries.
- **🧭 Omnibar & Agent Command Palette (`Ctrl+L` / `Ctrl+P`)**: Hybrid bar supporting path breadcrumb navigation, fuzzy search, terminal shell commands (`>`), and natural language agent queries (`@`).
- **🌿 Specialized Git Change Control Tab**: Dedicated Git inspector showing branch switcher, staged/unstaged changes, side-by-side diff viewer, and quick commit box (`Ctrl+Enter`).
- **🧠 Xavier Cognitive Core Bridge (`:8006`)**: Direct semantic memory indexing, smart tag dots, and contextual AI agent actions.

---

## 🏛️ Project Architecture

```text
swal-files/
├── crates/
│   ├── swal-files-core/        # Async VFS, inotify watcher, scanner, and caching
│   ├── swal-files-git/         # Git repository inspector, diff engine, and commit builder
│   ├── swal-files-preview/     # QuickLook preview decoders (syntect, image, markdown)
│   ├── swal-files-agent/       # Xavier Cognitive Memory REST/WebSocket client
│   └── swal-files-app/         # GPU-accelerated UI application (Tabs, Omnibar, Sidebar, Views)
├── .gitcore/                   # GitCore v3.8.0 specification & autonomous wave features
├── docs/                       # Architectural blueprints & benchmarks
└── tests/                      # Visual E2E & unit test suites
```

---

## 🧪 Building & Running

```bash
# Run unit and integration test suite
cargo test --workspace

# Launch standalone GUI
cargo run --release -p swal-files-app
```

---

## 📄 License

Distributed under the **MIT License**. Created with ⚡ by **SouthWest AI Labs**.
