# SWAL Files — Session Handoff & Wave Orchestration State (v3.8.0)

## 1. Project Identity & Status
- **Repository**: `/home/belal/proyectosSWAL/swal-files`
- **Specification Version**: GitCore v3.8.0
- **Total Features**: 15 (Wave 1)
- **Active Waves**: `wave-1` (Pending Dispatch)
- **Architecture**: Standalone High-Refresh (200Hz+) Native Rust File Manager inspired by `files-community/Files`, macOS Finder (QuickLook & Column View), and Yazi.

---

## 2. Workspace Structure
```text
swal-files/
├── Cargo.toml                  # Root workspace (5 member crates)
├── crates/
│   ├── swal-files-core/        # Types, Scanner, Watcher, Config, Tabs
│   ├── swal-files-git/         # Detector, Status, Diff, Commit
│   ├── swal-files-preview/     # Syntax, Markdown, Image
│   ├── swal-files-agent/       # Xavier Client, Semantic Tagger
│   └── swal-files-app/         # Omnibar, Main entrypoint
├── .gitcore/
│   ├── features.json           # 15 micro-tasks for Wave 1
│   ├── features/               # 15 atomic feature specs
│   ├── issues/                 # 15 canonical issue markdown bodies (body-01.md to body-15.md)
│   ├── planning/               # PLANNING.md & tasks.json
│   └── SESSION_HANDOFF.md      # State persistence & resumption
├── docs/                       # BENCHMARK_AND_INSPIRATION.md
├── ARCHITECTURE.md             # System module contracts
├── AGENT_INDEX.md              # Multi-agent roles
├── SRS.md                      # Functional & Non-functional specs
└── README.md                   # Full English documentation
```

---

## 3. Jules Wave 1 Dispatch Guide

To dispatch all 15 issues concurrently to Jules via GitHub CLI:

```bash
cd /home/belal/proyectosSWAL/swal-files

# Create 15 GitHub issues from prepared bodies:
for i in {01..15}; do
  gh issue create --title "feat(files): implement TASK-$i" --body-file ".gitcore/issues/body-$i.md" --label "wave-1,ola1"
done

# Dispatch in parallel to Jules:
for id in $(gh issue list --limit 15 --json number --jq '.[].number'); do
  gh issue edit $id --add-label "jules"
done
```
