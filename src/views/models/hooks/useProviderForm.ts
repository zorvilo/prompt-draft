import { desktop, errorMessage, type Profile } from "@/api/desktop";
import { useMessage } from "naive-ui";
import { computed, ref, watch } from "vue";

import {
  allowsHttpApi,
  configuredModels,
  createProviderProfile,
  modelPresets,
  presetFor,
  providerLogoFile,
} from "@/config/modelPresets";

import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";

export function useProviderForm() {
  const { data, saveProfile } = useWorkspaceContext();
  const message = useMessage();
  const logos = import.meta.glob<string>("@/assets/providers/*.svg", {
    eager: true,
    import: "default",
    query: "?url",
  });
  function providerLogo(presetId: string | undefined) {
    const file = providerLogoFile(presetId);
    return file ? logos["/src/assets/providers/" + file] : undefined;
  }

  const key = ref("");
  const busy = ref(false);
  const fetchingModels = ref(false);
  const profile = ref<Profile>();
  const hasKey = ref(false);
  const adding = ref(false);
  const query = ref("");
  const configured = computed(() => configuredModels(data.value.profiles));
  const testingProvider = ref<string>();

  function showModelError(item: Profile, error: unknown) {
    message.error(item.name + " · " + errorMessage(error), {
      duration: 9000,
      closable: true,
    });
  }

  async function testSaved(item: Profile) {
    if (testingProvider.value) return;
    testingProvider.value = item.id;
    try {
      if (!(await desktop.hasKey(item))) {
        throw new Error("请先点「编辑」，填写API Key。");
      }
      await desktop.test(item, null);
      message.success(item.name + " 连接成功");
    } catch (error) {
      showModelError(item, error);
    } finally {
      testingProvider.value = undefined;
    }
  }

  const selectedPreset = computed(
    () => profile.value && presetFor(profile.value),
  );
  const allowManualModel = computed(
    () =>
      selectedPreset.value?.id !== "preset-deepseek" &&
      selectedPreset.value?.id !== "preset-glm",
  );
  function selectPreset(item: (typeof modelPresets)[number]) {
    profile.value = createProviderProfile(item, configured.value);
  }
  function editProfile(item: Profile) {
    const preset = presetFor(item);
    profile.value = {
      ...item,
      presetId: preset?.id,
      website: item.website ?? preset?.website ?? "",
    };
  }
  const filtered = computed(() => {
    const term = query.value.trim().toLowerCase();
    return modelPresets.filter((item) =>
      [item.name, item.model, item.endpoint].some((text) =>
        text.toLowerCase().includes(term),
      ),
    );
  });

  function closeForm() {
    profile.value = undefined;
    adding.value = false;
    query.value = "";
  }
  watch(
    [profile, () => profile.value?.endpoint.trim()],
    async ([item, endpoint], [previous, previousEndpoint], onCleanup) => {
      let current = true;
      onCleanup(() => {
        current = false;
      });
      if (item !== previous) key.value = "";
      if (item && item === previous && endpoint !== previousEndpoint)
        item.catalog = [];
      hasKey.value = false;
      if (!item) return;
      try {
        const keyAvailable = await desktop.hasKey({
          ...item,
          endpoint: item.endpoint.trim(),
        });
        if (current) hasKey.value = keyAvailable;
      } catch (error) {
        if (current) showModelError(item, error);
      }
    },
    { immediate: true, flush: "sync" },
  );

  async function refreshModels() {
    const item = profile.value;
    if (!item || busy.value || fetchingModels.value) return;
    fetchingModels.value = true;
    try {
      const catalog = await desktop.fetchModels(
        { ...item, endpoint: item.endpoint.trim() },
        key.value.trim() || null,
      );
      if (profile.value !== item) return;
      item.catalog = catalog;
      message.success(
        "已获取 " + catalog.length + " 个模型。保存配置后可在工作台选择。",
      );
    } catch (error) {
      showModelError(item, error);
    } finally {
      fetchingModels.value = false;
    }
  }

  async function run(action: "test" | "save" | "clear") {
    const current = profile.value && {
      ...profile.value,
      endpoint: profile.value.endpoint.trim(),
      website: profile.value.website?.trim(),
    };
    if (busy.value || fetchingModels.value || !current || !selectedPreset.value)
      return;
    busy.value = true;
    try {
      if (action === "clear") {
        await desktop.setKey(current, "");
        hasKey.value = false;
        key.value = "";
        message.success("已清除已保存的密钥");
        return;
      }
      let endpoint: URL;
      try {
        endpoint = new URL(current.endpoint);
      } catch {
        throw new Error("请输入有效的模型接口地址。");
      }
      const local = allowsHttpApi(endpoint.hostname);
      if (
        !(
          endpoint.protocol === "https:" ||
          (local && endpoint.protocol === "http:")
        ) ||
        endpoint.username ||
        endpoint.password ||
        endpoint.search ||
        endpoint.hash
      )
        throw new Error(
          "公网接口请使用 HTTPS，本机和内网接口可使用 HTTP；地址中请勿加入账号、查询参数或片段。",
        );
      if (current.website) {
        let website: URL;
        try {
          website = new URL(current.website);
        } catch {
          throw new Error("请输入有效的官方网站地址。");
        }
        if (
          !["https:", "http:"].includes(website.protocol) ||
          website.username ||
          website.password
        )
          throw new Error(
            "官方网站地址须使用 HTTP 或 HTTPS，且不包含账号信息。",
          );
      }
      const token = key.value.trim();
      if (!token && !hasKey.value)
        throw new Error("请输入所选服务的 API Key。");
      if (action === "test") {
        await desktop.test(current, token || null);
        message.success(current.name + " 连接成功");
        return;
      }
      if (!current.name.trim()) throw new Error("请输入配置名称。");
      if (!current.model.trim()) throw new Error("请选择或输入模型 ID。");
      const saved = await saveProfile(
        {
          ...current,
          name: current.name.trim(),
          model: current.model.trim(),
        },
        token || undefined,
      );
      if (!saved) return;
      hasKey.value = true;
      key.value = "";
      closeForm();
    } catch (error) {
      showModelError(current, error);
    } finally {
      busy.value = false;
    }
  }

  return {
    providerLogo,
    key,
    busy,
    fetchingModels,
    profile,
    hasKey,
    adding,
    query,
    configured,
    testingProvider,
    testSaved,
    selectedPreset,
    allowManualModel,
    selectPreset,
    editProfile,
    filtered,
    closeForm,
    refreshModels,
    run,
  };
}
