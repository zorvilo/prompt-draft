import assert from "node:assert/strict";
import { createUpdateManifest } from "../scripts/prepare-update.mjs";
const manifest = createUpdateManifest(
  "0.1.1",
  "https://example.com/setup.exe",
  "YWJjZA==\n",
  "修复问题",
);
assert.equal(manifest.version, "0.1.1");
assert.deepEqual(Object.keys(manifest.platforms), ["windows-x86_64"]);
assert.equal(manifest.platforms["windows-x86_64"].signature, "YWJjZA==");
assert.equal(manifest.notes, "修复问题");
assert.throws(() =>
  createUpdateManifest("bad", "https://example.com/setup.exe", "YWJjZA=="),
);
assert.throws(() =>
  createUpdateManifest("0.1.1", "http://example.com/setup.exe", "YWJjZA=="),
);
assert.throws(() =>
  createUpdateManifest(
    "0.1.1",
    "https://user:password@example.com/setup.exe",
    "YWJjZA==",
  ),
);
assert.throws(() =>
  createUpdateManifest("0.1.1", "https://example.com/setup.exe", ""),
);
