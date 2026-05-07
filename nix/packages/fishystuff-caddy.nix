{ caddy }:

# Stock Caddy can serve precompressed .br sidecars, but live Brotli encoding is
# provided by an external module.
caddy.withPlugins {
  plugins = [
    "github.com/ueffel/caddy-brotli@v1.6.0"
  ];
  hash = "sha256-KvE1ZsUic6VDw/HiXtSkJ4kdrp5ehA86ovF9xxWQ71g=";
}
