# lib

Shared Rust crates with stable, narrow responsibilities.

Typical contents:

- domain ids and math
- canonical transforms and geometry helpers
- API DTOs and contract types
- typed API client code
- small shared support crates reused by runtime and tooling components

This directory should not become a generic dumping ground for unstable runtime logic.
