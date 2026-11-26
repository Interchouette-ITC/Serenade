import { build, context } from "esbuild";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = path.join(root, "assets/clitorine.min.js");
const mode = process.argv[2];
if (
  process.argv.length > 3 ||
  (mode && !["--check", "--watch"].includes(mode))
) {
  throw new Error("Usage: npm run build | npm run watch | npm run check");
}
const options = {
  absWorkingDir: root,
  entryPoints: ["assets/clitorine.js"],
  outfile: output,
  bundle: true,
  minify: true,
  format: "iife",
  target: "es2020",
  legalComments: "none",
  banner: {
    js: "/*! Clitorine - MyFeed demo. Generated; edit source modules. */",
  },
};

if (mode === "--watch") {
  const watcher = await context(options);
  await watcher.watch();
  console.log(
    "Watching Clitorine modules; Cargo embeds the bundle on rebuild.",
  );
} else if (mode === "--check") {
  const result = await build({ ...options, write: false });
  const current = await readFile(output);
  if (!current.equals(Buffer.from(result.outputFiles[0].contents))) {
    throw new Error("Clitorine bundle is stale. Run npm run build.");
  }
  console.log("Clitorine bundle matches its source modules.");
} else {
  await build(options);
  console.log("Built assets/clitorine.min.js");
}
