import { desktop, type Profile } from "@/api/desktop";
import { modelsFor } from "@/config/modelPresets";
import { useWorkspace } from "@/store/modules/workspace";
import { useMessage } from "naive-ui";
import { computed, watch } from "vue";

import { useConfirmDialog } from "@/components/useConfirmDialog";
import { useTemplates } from "@/views/templates/hooks/useTemplates";
export function useProviders(
  workspace: ReturnType<typeof useWorkspace>,
  templates: ReturnType<typeof useTemplates>,
) {
  const { data, draft, profile, models, saveError, save } = workspace;
  const message = useMessage();
  const { templateList, defaultTemplateId } = templates;
  const confirm = useConfirmDialog();
  const selectedModel = computed({
    get: () => draft.value?.model ?? null,
    set: (model) => {
      if (draft.value) draft.value.model = model;
    },
  });
  const modelOptions = computed(() =>
    (profile.value ? modelsFor(profile.value) : []).map((model) => ({
      label: model,
      value: model,
    })),
  );
  watch(
    () => [
      draft.value?.id,
      draft.value?.providerId,
      draft.value?.model,
      draft.value?.templateId,
      JSON.stringify(data.value.profiles),
      JSON.stringify(templateList.value),
    ],
    () => {
      const item = draft.value;
      if (!item) return;
      if (
        !templateList.value.some((template) => template.id === item.templateId)
      )
        item.templateId = defaultTemplateId.value;
      if (!models.value.some((provider) => provider.id === item.providerId)) {
        const fallback =
          models.value.find((provider) => provider.isDefault) ??
          (models.value.length === 1 ? models.value[0] : undefined);
        item.providerId = fallback?.id ?? null;
        item.model = fallback?.model ?? null;
      }
      const provider = models.value.find(
        (provider) => provider.id === item.providerId,
      );
      if (provider && !modelsFor(provider).includes(item.model ?? ""))
        item.model = provider.model;
    },
    { immediate: true, flush: "sync" },
  );
  function selectProvider(id: string) {
    if (!draft.value) return;
    draft.value.providerId = id;
    draft.value.model =
      models.value.find((item) => item.id === id)?.model ?? null;
  }
  async function setDefaultProfile(id: string) {
    for (const item of data.value.profiles) item.isDefault = item.id === id;
    await save();
    if (saveError.value) message.error(saveError.value);
    else message.success("已设为默认供应商");
  }
  async function saveProfile(profile: Profile, key?: string) {
    if (!data.value.profiles.length) profile = { ...profile, isDefault: true };
    const previous = data.value.profiles;
    const selected = draft.value;
    const previousSelection = selected && {
      providerId: selected.providerId,
      model: selected.model,
    };
    const index = data.value.profiles.findIndex(
      (item) => item.id === profile.id,
    );
    data.value.profiles =
      index === -1
        ? [...previous, profile]
        : previous.map((item, position) =>
            position === index ? profile : item,
          );
    const attempted = data.value.profiles;
    const attemptedSelection = selected && {
      providerId: selected.providerId,
      model: selected.model,
    };
    const saved = await save(
      key === undefined
        ? undefined
        : // Keep the typed key out of Pinia action arguments and devtools records.
          (changes) =>
            desktop.saveChanges(changes, {
              providerId: profile.id,
              endpoint: profile.endpoint,
              key,
            }),
    );
    if (!saved) {
      const restoreSelection =
        selected &&
        attemptedSelection &&
        selected.providerId === attemptedSelection.providerId &&
        selected.model === attemptedSelection.model;
      if (data.value.profiles === attempted) {
        data.value.profiles = previous;
        if (restoreSelection && previousSelection) {
          selected.providerId = previousSelection.providerId;
          selected.model = previousSelection.model;
        }
      }
      message.error(saveError.value || "供应商配置保存失败，请重试。");
      return false;
    }
    message.success("供应商配置已保存");
    return true;
  }
  function removeProfile(item: Profile) {
    confirm(
      "删除供应商",
      "删除「" + item.name + "」并清除已保存的密钥？",
      async () => {
        if (data.value.activeProfile === item.id)
          data.value.activeProfile = null;
        data.value.profiles = data.value.profiles.filter(
          (entry) => entry.id !== item.id,
        );
        await save();
        if (!saveError.value) message.success("供应商已删除");
      },
      "删除",
    );
  }

  return {
    selectedModel,
    modelOptions,
    selectProvider,
    setDefaultProfile,
    saveProfile,
    removeProfile,
  };
}
