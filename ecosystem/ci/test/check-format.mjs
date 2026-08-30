import { readFileSync } from "node:fs";
for (const file of ["README.md", "docs/contracts.md", "docs/architecture.md", "docs/seo.md"]) {
  const text = readFileSync(file, "utf8");
  if (text.includes("\u2014")) throw new Error(`${file} contains an em dash`);
  if (/\bAI\b/.test(text)) throw new Error(`${file} contains forbidden brand-facing term`);
}
console.log("format policy ok");
