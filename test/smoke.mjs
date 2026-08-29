import { readFileSync } from "node:fs";
for (const file of [
  "grammar.js",
  "queries/highlights.scm",
  "test/corpus/basic.txt",
  "test/corpus/top-level-and-boolean.txt",
  "src/parser.c",
  "src/node-types.json",
]) {
  const text = readFileSync(file, "utf8");
  if (text.length < 10) throw new Error(`${file} is unexpectedly empty`);
}
console.log("tree-sitter-sley smoke ok");
