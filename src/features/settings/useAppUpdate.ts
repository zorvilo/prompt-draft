import { computed, ref } from "vue";
import {
  desktop,
  errorMessage,
  type AvailableUpdate,
  type Workspace,
} from "../../lib/tauri";
export function useAppUpdate(options: {
  getSource: () => Workspace["updateSource"];
  setSource: (source: Workspace["updateSource"]) => Promise<void>;
  prepareUpdate: () => Promise<void>;
}) {
  const updateInfo = ref<AvailableUpdate | null>(null);
  const showUpdateDialog = ref(false);
  const updateBusy = ref(false);
  const updateStage = ref<"check" | "download" | "install">("check");
  const updateError = ref("");
  const checkedUpdate = ref(false);
  const updateProgress = ref<number | undefined>();
  async function checkUpdate(silent = false) {
    if (updateBusy.value) return;
    updateBusy.value = true;
    updateStage.value = "check";
    updateError.value = "";
    try {
      updateInfo.value = await desktop.checkUpdate(options.getSource());
      selectedUpdateSource.value = options.getSource() ?? "gitee";
      checkedUpdate.value = true;
      if (!silent) showUpdateDialog.value = !!updateInfo.value;
    } catch (error) {
      updateError.value = errorMessage(error);
    } finally {
      updateBusy.value = false;
    }
  }
  const selectedUpdateSource = ref<"gitee" | "github">("gitee");
  const rememberUpdateSource = ref(true);
  async function changeUpdateSource(value: string) {
    try {
      await options.setSource(
        value === "ask" ? undefined : (value as "gitee" | "github"),
      );
      if (!updateInfo.value?.downloaded) await checkUpdate();
    } catch (error) {
      updateError.value = errorMessage(error);
    }
  }
  async function downloadUpdate(source: "gitee" | "github") {
    if (updateBusy.value || !updateInfo.value) return;
    updateBusy.value = true;
    updateStage.value = "download";
    updateError.value = "";
    updateProgress.value = undefined;
    try {
      if (rememberUpdateSource.value) await options.setSource(source);
      await desktop.downloadUpdate(source, ({ downloaded, total }) => {
        updateProgress.value = total
          ? Math.min(100, Math.floor((downloaded / total) * 100))
          : undefined;
      });
      updateInfo.value.downloaded = true;
      updateStage.value = "install";
      await options.prepareUpdate();
      await desktop.installUpdate();
    } catch (error) {
      updateError.value = errorMessage(error);
    } finally {
      updateBusy.value = false;
    }
  }
  async function installUpdate() {
    if (updateBusy.value) return;
    updateBusy.value = true;
    updateStage.value = "install";
    updateError.value = "";
    try {
      await options.prepareUpdate();
      await desktop.installUpdate();
    } catch (error) {
      updateError.value = errorMessage(error);
    } finally {
      updateBusy.value = false;
    }
  }
  const updateStatus = computed(() => {
    if (updateBusy.value)
      return updateStage.value === "check"
        ? "正在检查更新…"
        : updateStage.value === "download"
          ? "正在下载更新…"
          : "正在准备安装…";
    if (updateError.value) return "更新操作未完成";
    if (updateInfo.value?.downloaded) return "更新已就绪";
    if (updateInfo.value) return `发现新版本 ${updateInfo.value.version}`;
    return checkedUpdate.value ? "你使用的是最新版本" : "正在准备检查更新";
  });
  return {
    updateInfo,
    showUpdateDialog,
    updateBusy,
    updateStage,
    updateError,
    updateProgress,
    selectedUpdateSource,
    rememberUpdateSource,
    checkUpdate,
    changeUpdateSource,
    downloadUpdate,
    installUpdate,
    updateStatus,
  };
}
export type AppUpdate = ReturnType<typeof useAppUpdate>;
