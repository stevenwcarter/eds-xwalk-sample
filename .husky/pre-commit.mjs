import { exec } from 'node:child_process';

// Resolve with the outcome rather than rejecting: lint steps are expected to
// exit non-zero, and we need their output to show the developer what failed.
const run = (cmd) => new Promise((resolve) => exec(
  cmd,
  { maxBuffer: 32 * 1024 * 1024 },
  (error, stdout, stderr) => resolve({
    ok: !error,
    code: error ? error.code ?? 1 : 0,
    stdout,
    stderr,
  }),
));

const out = async (cmd) => (await run(cmd)).stdout;

const changeset = await out('git diff --cached --name-only --diff-filter=ACMR');
const modifiedFiles = changeset.split('\n').filter(Boolean);

// check if there are any model files staged
const modifledPartials = modifiedFiles.filter((file) => file.match(/(^|\/)_.*.json/));
if (modifledPartials.length > 0) {
  console.log(await out('npm run build:json --silent'));
  await run('git add component-models.json component-definition.json component-filters.json');
}

// Content hash of each staged file, so we can tell which ones the linters
// actually rewrote. Comparing hashes (rather than `git diff --name-only`)
// avoids re-staging unrelated work that was already dirty in the worktree.
const hashes = async (files) => {
  if (files.length === 0) return {};
  const list = files.map((f) => JSON.stringify(f)).join(' ');
  const { stdout } = await run(`git hash-object ${list}`);
  const lines = stdout.split('\n').filter(Boolean);
  return Object.fromEntries(files.map((file, i) => [file, lines[i]]));
};

const before = await hashes(modifiedFiles);

// Run the two fixers independently: `npm run lint:fix` chains them with `&&`, so
// an unfixable JS error would skip the CSS pass entirely. Failures here are not
// fatal -- the verification pass below decides whether the commit proceeds.
await run('npm run lint:js --silent -- --fix');
await run('npm run lint:css --silent -- --fix');

const after = await hashes(modifiedFiles);
const fixed = modifiedFiles.filter((file) => before[file] && after[file] !== before[file]);

if (fixed.length > 0) {
  await run(`git add ${fixed.map((f) => JSON.stringify(f)).join(' ')}`);
  console.log(`Auto-formatted and re-staged:\n${fixed.map((f) => `  ${f}`).join('\n')}`);
}

// Verify with the exact command CI runs, so a passing commit means a passing build.
const lint = await run('npm run lint --silent');
if (!lint.ok) {
  console.error(lint.stdout || '');
  console.error(lint.stderr || '');
  console.error('\nCommit aborted: `npm run lint` failed and could not be auto-fixed.');
  process.exit(lint.code || 1);
}
