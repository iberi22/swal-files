# SWAL Files — Índice de Agentes y Roles (GitCore v3.8.0)

| Agente | Rol en el Proyecto | Contexto & Responsabilidades |
|---|---|---|
| **Hermes Orchestrator** | Arquitecto Principal & Síntesis Local | Descompone la arquitectura en olas de trabajo, valida la compatibilidad con el ecosistema SWAL y realiza la integración final. |
| **Jules (Autonomous Worker)** | Ejecutor Concurrente de Tareas | Implementa micro-tareas de código en paralelo (lotes de hasta 15 micro-issues disjuntas) entregando Pull Requests limpios. |
| **Xavier Node Core** | Motor Cognitivo & RAG | Provee búsqueda semántica sobre archivos, generación de resúmenes de código e indexación en base vectorial. |
| **Antigravity (Pair Programmer)** | Coordinación de Olas & Pruebas Visuales | Valida compilación, ejecuta suites de prueba E2E con capturas de pantalla de Wayland y reconcilia `features.json`. |
