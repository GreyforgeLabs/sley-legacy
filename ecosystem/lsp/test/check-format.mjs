import { readFileSync } from "node:fs";
import { globSync } from "node:fs";

const files = ["README.md", "docs/contracts.md", "docs/architecture.md", "docs/seo.md"];
for (const file of files) {
  const text = readFileSync(file, "utf8");
  if (text.includes("\u2014")) {
    throw new Error(`${file} contains an em dash`);
  }
  if (/\bAI\b/.test(text)) {
    throw new Error(`${file} contains forbidden brand-facing term`);
  }
}
console.log("format policy ok");
