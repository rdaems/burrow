// LICENSES.txt: the licence of every package bundled into ../editor.js (all of node_modules but esbuild and types)
//   node licenses.mjs <node_modules> > LICENSES.txt   (build.sh runs it)
import { readdirSync, readFileSync, existsSync } from "node:fs";
const nm = process.argv[2], out = [];
const pkgs = readdirSync(nm).filter((n) => !n.startsWith(".")).flatMap((n) => (n.startsWith("@") ? readdirSync(`${nm}/${n}`).map((m) => `${n}/${m}`) : [n]));
for (const p of pkgs.sort()) {
  if (/^(esbuild|@esbuild\/|@types\/)/.test(p)) continue;
  const dir = `${nm}/${p}`, meta = JSON.parse(readFileSync(`${dir}/package.json`, "utf8"));
  const file = readdirSync(dir).find((f) => /^licen[cs]e/i.test(f));
  out.push(`${p} ${meta.version} (${meta.license || "see below"})\n${"-".repeat(60)}\n${file ? readFileSync(`${dir}/${file}`, "utf8").trim() : "(no licence file in the package)"}\n`);
}
console.log("The packages bundled into web/editor.js, and their licences.\n\n" + out.join("\n"));
