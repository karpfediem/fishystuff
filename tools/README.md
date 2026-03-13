# tools

Offline and administrative tooling.

This component should contain:

- purpose-built Rust tooling crates
- lightweight orchestration scripts under `tools/scripts/`
- documentation for local bake, import, and maintenance workflows

Scripts should stay thin. If a script accumulates business logic, move that logic into a Rust crate and keep the script as a small wrapper.
