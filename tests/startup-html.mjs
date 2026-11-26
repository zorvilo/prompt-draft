import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
// Inline styles acquire a Tauri CSP nonce and block Naive UI's runtime styles.
for (const file of ["../index.html", "../dist/index.html"]) {
  const html = readFileSync(new URL(file, import.meta.url), "utf8");
  assert.doesNotMatch(html, /<style(?:\s|>)/i, file);
}
console.log("PASS source and production HTML contain no inline style elements");
