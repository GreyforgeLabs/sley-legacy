import { spawnSync } from 'node:child_process';
import { readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const grammarRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = path.resolve(grammarRoot, '..');

const roots = [
  path.join(repoRoot, 'examples'),
  path.join(repoRoot, 'fixtures', 'corpus', 'accepted'),
];

const files = roots.flatMap(collectSleyFiles).sort();

if (files.length === 0) {
  console.error('no .sley fixtures found');
  process.exit(1);
}

let failed = 0;
for (const file of files) {
  const result = spawnSync(
    path.join(grammarRoot, 'node_modules', '.bin', process.platform === 'win32' ? 'tree-sitter.cmd' : 'tree-sitter'),
    ['parse', '--quiet', '--grammar-path', grammarRoot, file],
    { cwd: grammarRoot, encoding: 'utf8' },
  );
  if (result.status !== 0) {
    failed += 1;
    console.error(`tree-sitter parse failed for ${path.relative(repoRoot, file)}`);
    if (result.stderr) {
      console.error(result.stderr.trim());
    }
  }
}

if (failed > 0) {
  console.error(`${failed}/${files.length} Sley fixtures failed Tree-sitter parsing`);
  process.exit(1);
}

console.log(`parsed ${files.length} Sley fixtures with tree-sitter-sley`);

function collectSleyFiles(root) {
  const files = [];
  walk(root, files);
  return files;
}

function walk(current, files) {
  const stat = statSync(current);
  if (stat.isDirectory()) {
    for (const entry of readdirSync(current)) {
      if (entry === 'node_modules' || entry === 'target') {
        continue;
      }
      walk(path.join(current, entry), files);
    }
    return;
  }
  if (current.endsWith('.sley')) {
    files.push(current);
  }
}
