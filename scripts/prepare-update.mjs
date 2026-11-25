import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function createUpdateManifest(version, url, signature, notes = "") {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version))
    throw new Error("版本号格式无效");
  const address = new URL(url);
  if (address.protocol !== "https:" || address.username || address.password)
    throw new Error("安装包必须使用不含凭据的 HTTPS 地址");
  if (!signature.trim() || !/^[A-Za-z0-9+/=]+$/.test(signature.trim()))
    throw new Error("缺少有效签名，请使用安装包对应的 .sig 文件");
  return {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms: {
      "windows-x86_64": { url: address.href, signature: signature.trim() },
    },
  };
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const [url, signaturePath, notesPath, outputPath] = process.argv.slice(2);
  if (!url || !signaturePath || !notesPath || !outputPath)
    throw new Error(
      "用法：node scripts/prepare-update.mjs <安装包HTTPS地址> <签名文件> <说明文件> <输出JSON路径>",
    );
  const config = JSON.parse(
    readFileSync(
      new URL("../src-tauri/tauri.conf.json", import.meta.url),
      "utf8",
    ),
  );
  const packageJson = JSON.parse(
    readFileSync(new URL("../package.json", import.meta.url), "utf8"),
  );
  const cargo = readFileSync(
    new URL("../src-tauri/Cargo.toml", import.meta.url),
    "utf8",
  );
  const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (packageJson.version !== config.version || cargoVersion !== config.version)
    throw new Error(
      "package.json、Cargo.toml 与 tauri.conf.json 的版本号必须一致",
    );
  const manifest = createUpdateManifest(
    config.version,
    url,
    readFileSync(signaturePath, "utf8"),
    readFileSync(notesPath, "utf8"),
  );
  writeFileSync(outputPath, JSON.stringify(manifest, null, 2) + "\n");
}
