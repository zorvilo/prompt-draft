import { existsSync } from "node:fs";
import assert from "node:assert/strict";
import {
  modelPresets,
  allowsHttpApi,
  presetFor,
  configuredModels,
  createProviderProfile,
  modelsFor,
  providerLogoFile,
} from "../src/features/settings/modelPresets.ts";
const deepseek = modelPresets.find((item) => item.id === "preset-deepseek");
const first = createProviderProfile(deepseek, []);
const second = createProviderProfile(deepseek, [first]);
assert.equal(first.name, "DeepSeek");
assert.equal(second.name, "DeepSeek 2");
assert.equal(presetFor(second).name, "DeepSeek");
assert.equal(second.website, deepseek.website);
second.name = "我的配置";
second.endpoint = "https://proxy.example/v1";
second.website = "https://example.com";
assert.equal(presetFor(second).id, deepseek.id);
assert.equal(presetFor(second).name, "DeepSeek");
assert.equal(
  createProviderProfile(deepseek, [first, second]).name,
  "DeepSeek 3",
);
assert.deepEqual(configuredModels([first, second]), [first, second]);
assert.deepEqual(modelsFor(second), [deepseek.model]);
assert.equal(deepseek.endpoint, "https://api.deepseek.com");
assert.equal(deepseek.website, "https://platform.deepseek.com");
console.log(
  "PASS independent preset identity, duplicate instance naming, editable addresses, custom endpoint availability, model defaults",
);

const customPreset = modelPresets.find((item) => item.id === "preset-custom");
const own = createProviderProfile(customPreset, []);
assert.equal(own.endpoint, "");
assert.equal(own.model, "");
assert.equal(own.website, "");
own.name = "公司 API";
own.endpoint = "https://api.example.com/v1";
own.model = "company-model";
own.catalog = ["company-model", "other-model"];
assert.equal(presetFor(own).id, "preset-custom");
assert.ok(configuredModels([own]).includes(own));
assert.deepEqual(modelsFor(own), ["company-model", "other-model"]);
assert.equal(createProviderProfile(customPreset, [own]).name, "自定义供应商 2");
assert.equal(
  presetFor({ id: "invalid", name: "invalid", endpoint: "", model: "model" }),
  undefined,
);
console.log(
  "PASS custom provider creation, identification, manual/fetched models and invalid preset handling",
);

for (const host of [
  "localhost",
  "[::1]",
  "127.0.0.1",
  "10.147.103.23",
  "172.16.0.1",
  "172.31.255.255",
  "192.168.1.1",
])
  assert.ok(allowsHttpApi(host), host);
for (const host of [
  "example.com",
  "8.8.8.8",
  "172.15.255.255",
  "172.32.0.1",
  "192.169.0.1",
  "10.1.2.999",
])
  assert.ok(!allowsHttpApi(host), host);
console.log("PASS private-network HTTP support and public-address boundaries");

for (const preset of modelPresets.filter(
  (item) => item.id !== "preset-custom",
)) {
  const file = providerLogoFile(preset.id);
  assert.ok(
    existsSync(new URL("../src/assets/providers/" + file, import.meta.url)),
    preset.id,
  );
  const profile = createProviderProfile(preset, []);
  profile.name = "改名后的配置";
  profile.endpoint = "https://proxy.example/v1";
  assert.equal(providerLogoFile(profile.presetId), file);
}
assert.equal(providerLogoFile("preset-glm"), "zhipu.svg");
assert.equal(providerLogoFile("preset-custom"), undefined);
assert.equal(providerLogoFile(undefined), undefined);
console.log(
  "PASS all brand logos resolve for presets and renamed/custom-endpoint provider instances.",
);
