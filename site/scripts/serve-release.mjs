import path from "node:path";

function parseArgs(argv) {
  const args = {
    port: 1990,
    root: ".out",
    host: "127.0.0.1",
    help: false,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (arg === "--port" && i + 1 < argv.length) {
      args.port = Number(argv[++i]);
      continue;
    }
    if (arg === "--root" && i + 1 < argv.length) {
      args.root = argv[++i];
      continue;
    }
    if (arg === "--host" && i + 1 < argv.length) {
      args.host = argv[++i];
      continue;
    }
    throw new Error(`unknown arg: ${arg}`);
  }

  if (!Number.isFinite(args.port) || args.port <= 0) {
    throw new Error(`invalid --port: ${args.port}`);
  }

  return args;
}

function printHelp() {
  console.log(`serve-release.mjs

Serve the generated Zine release output from .out for local development.

Options:
  --root <dir>   Release output directory (default: .out)
  --host <addr>  Listen host (default: 127.0.0.1)
  --port <num>   Listen port (default: 1990)
  --help         Show this message
`);
}

function withinRoot(root, candidate) {
  return candidate === root || candidate.startsWith(`${root}${path.sep}`);
}

async function resolveFile(root, pathname) {
  const cleanPath = decodeURIComponent(pathname).replace(/^\/+/, "");
  const base = path.resolve(root);
  const stem = path.resolve(base, cleanPath);
  const candidates = [];

  if (!cleanPath || pathname.endsWith("/")) {
    candidates.push(path.resolve(base, cleanPath, "index.html"));
  } else {
    candidates.push(stem);
    candidates.push(`${stem}.html`);
    candidates.push(path.resolve(stem, "index.html"));
  }

  for (const candidate of candidates) {
    if (!withinRoot(base, candidate)) {
      continue;
    }
    const file = Bun.file(candidate);
    if (await file.exists()) {
      return file;
    }
  }

  return null;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    return;
  }

  const root = args.root;
  const server = Bun.serve({
    hostname: args.host,
    port: args.port,
    async fetch(request) {
      const url = new URL(request.url);
      const file = await resolveFile(root, url.pathname);
      if (file) {
        return new Response(file);
      }
      return new Response("Not Found", { status: 404 });
    },
  });

  console.log(`Serving ${path.resolve(process.cwd(), root)} at http://${server.hostname}:${server.port}/`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
