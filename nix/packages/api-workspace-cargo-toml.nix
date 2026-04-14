{ writeText }:

writeText "fishystuff-api-workspace-Cargo.toml" ''
  [workspace]
  resolver = "2"
  members = [
    "api/fishystuff_server",
    "lib/fishystuff_api",
    "lib/fishystuff_config",
    "lib/fishystuff_core",
  ]

  [workspace.package]
  edition = "2021"
  license = "MIT"

  [workspace.dependencies]
  anyhow = "1"
  chrono = { version = "0.4", default-features = false, features = ["alloc", "std"] }
  clap = { version = "4", features = ["derive"] }
  csv = "1"
  image = { version = "0.25", default-features = false, features = ["png"] }
  mysql = { version = "24", default-features = false, features = ["default-rustls"] }
  rusqlite = { version = "0.31", features = ["bundled"] }
  serde = { version = "1", features = ["derive"] }

  [profile.profiling]
  inherits = "release"
  debug = 1
  strip = "none"
''
