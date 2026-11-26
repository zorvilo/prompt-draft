import type { Profile } from "../api/desktop";

export const modelPresets = [
  {
    id: "preset-custom",
    name: "自定义供应商",
    model: "",
    website: "",
    endpoint: "",
    models: [],
  },
  {
    id: "preset-deepseek",
    name: "DeepSeek",
    model: "deepseek-flash",
    website: "https://platform.deepseek.com",
    endpoint: "https://api.deepseek.com",
    models: [],
  },
  {
    id: "preset-glm",
    name: "智谱 GLM",
    model: "glm-4.7-flash",
    website: "https://bigmodel.cn",
    endpoint: "https://open.bigmodel.cn/api/paas/v4",
    models: [],
  },
  {
    id: "preset-qwen",
    name: "阿里云百炼 · 通义千问",
    endpoint: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    website: "https://bailian.console.aliyun.com",
    model: "qwen-plus",
    models: ["qwen-plus", "qwen-turbo", "qwen-max"],
  },
  {
    id: "preset-kimi",
    name: "月之暗面 · Kimi",
    endpoint: "https://api.moonshot.cn/v1",
    website: "https://platform.moonshot.cn",
    model: "moonshot-v1-8k",
    models: ["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"],
  },
  {
    id: "preset-doubao",
    name: "火山方舟 · 豆包",
    endpoint: "https://ark.cn-beijing.volces.com/api/v3",
    website: "https://console.volcengine.com/ark",
    model: "doubao-seed-2-1-pro-260915",
    models: ["doubao-seed-2-1-pro-260915"],
  },
  {
    id: "preset-minimax",
    name: "MiniMax",
    endpoint: "https://api.minimax.cn/v1",
    website: "https://platform.minimax.cn",
    model: "MiniMax-M3",
    models: ["MiniMax-M3", "MiniMax-M3.1-Flash"],
  },
  {
    id: "preset-stepfun",
    name: "阶跃星辰",
    endpoint: "https://api.stepfun.com/v1",
    website: "https://platform.stepfun.com",
    model: "step-3.5-flash",
    models: ["step-3.5-flash"],
  },
  {
    id: "preset-mimo",
    name: "小米 MiMo",
    endpoint: "https://api.xiaomimimo.com/v1",
    website: "https://platform.xiaomimimo.com",
    model: "mimo-v2.6-pro",
    models: ["mimo-v2.6-pro"],
  },
  {
    id: "preset-longcat",
    name: "美团 LongCat",
    endpoint: "https://api.longcat.chat/openai/v1",
    website: "https://longcat.chat/platform",
    model: "LongCat-2.0",
    models: ["LongCat-2.0"],
  },
  {
    id: "preset-siliconflow",
    name: "硅基流动",
    endpoint: "https://api.siliconflow.cn/v1",
    website: "https://cloud.siliconflow.cn",
    model: "deepseek-ai/DeepSeek-V3.2",
    models: ["deepseek-ai/DeepSeek-V3.2"],
  },
  {
    id: "preset-baidu",
    name: "百度智能云千帆",
    endpoint: "https://qianfan.baidubce.com/v2",
    website: "https://cloud.baidu.com/product/qianfan_modelbuilder",
    model: "ernie-4.5-8k",
    models: ["ernie-4.5-8k"],
  },
  {
    id: "preset-tencent",
    name: "腾讯云 TokenHub",
    endpoint: "https://tokenhub.tencentmaas.com/v1",
    website: "https://cloud.tencent.com/product/tokenhub",
    model: "glm-5.3",
    models: ["glm-5.3"],
  },
  {
    id: "preset-modelscope",
    name: "魔搭社区 ModelScope",
    endpoint: "https://api-inference.modelscope.cn/v1",
    website: "https://modelscope.cn",
    model: "Qwen/Qwen3-32B",
    models: ["Qwen/Qwen3-32B"],
  },
  {
    id: "preset-spark",
    name: "讯飞星火",
    endpoint: "https://spark-api-open.xf-yun.com/v1",
    website: "https://xinghuo.xfyun.cn",
    model: "generalv3.5",
    models: ["generalv3.5", "lite", "4.0Ultra"],
  },
  {
    id: "preset-baichuan",
    name: "百川智能",
    endpoint: "https://api.baichuan-ai.com/v1",
    website: "https://platform.baichuan-ai.com",
    model: "Baichuan4-Turbo",
    models: ["Baichuan4-Turbo", "Baichuan4-Air", "Baichuan4"],
  },
] satisfies (Profile & { models: string[]; website: string })[];

export function providerLogoFile(presetId: string | undefined) {
  if (!presetId || presetId === "preset-custom") return undefined;
  const name =
    presetId === "preset-glm" ? "zhipu" : presetId.replace(/^preset-/, "");
  return name + ".svg";
}

export function presetFor(profile: Profile) {
  return modelPresets.find((item) => item.id === profile.presetId);
}

export function createProviderProfile(
  item: (typeof modelPresets)[number],
  profiles: Profile[],
): Profile {
  const copies = profiles.filter(
    (record) => presetFor(record)?.id === item.id,
  ).length;
  const names = new Set(profiles.map((record) => record.name));
  let suffix = copies + 1;
  let name = copies ? item.name + " " + suffix : item.name;
  while (names.has(name)) name = item.name + " " + ++suffix;
  return {
    id: crypto.randomUUID(),
    presetId: item.id,
    name,
    website: item.website,
    model: item.model,
    endpoint: item.endpoint,
  };
}

export function configuredModels(profiles: Profile[]): Profile[] {
  return profiles.filter(
    (profile) => presetFor(profile) && profile.model.trim(),
  );
}

export function modelsFor(profile: Profile) {
  const presets = presetFor(profile)?.models ?? [];
  return [
    ...new Set([
      profile.model,
      ...(profile.catalog?.length ? profile.catalog : presets),
    ]),
  ].filter(Boolean);
}

export function allowsHttpApi(hostname: string): boolean {
  if (hostname === "localhost" || hostname === "[::1]") return true;
  const octets = hostname.split(".").map(Number);
  if (
    octets.length !== 4 ||
    !octets.every(
      (value) => Number.isInteger(value) && value >= 0 && value <= 255,
    )
  )
    return false;
  const [first, second] = octets;
  return (
    first === 127 ||
    first === 10 ||
    (first === 172 && second >= 16 && second <= 31) ||
    (first === 192 && second === 168)
  );
}
