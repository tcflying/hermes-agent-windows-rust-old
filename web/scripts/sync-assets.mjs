import { cpSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));

for (const name of ["fonts", "ds-assets"]) {
  rmSync(join(root, "public", name), { force: true, recursive: true });
}

cpSync(
  join(root, "node_modules", "@nous-research", "ui", "dist", "fonts"),
  join(root, "public", "fonts"),
  { recursive: true },
);
cpSync(
  join(root, "node_modules", "@nous-research", "ui", "dist", "assets"),
  join(root, "public", "ds-assets"),
  { recursive: true },
);
