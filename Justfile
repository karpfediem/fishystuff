deploy-bot:
  skopeo --insecure-policy --debug copy docker-archive:"$(nix build .?submodules=1#bot-container --no-link --print-out-paths)" docker://registry.fly.io/criobot:latest --dest-creds x:"$(flyctl auth token)" --format v2s2
  flyctl deploy --remote-only -c bot/fly.toml