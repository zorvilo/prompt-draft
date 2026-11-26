import { desktop, errorMessage } from "@/api/desktop";
import { useWorkspace } from "@/store/modules/workspace";
import { open, save as pickSave } from "@tauri-apps/plugin-dialog";
import { NText, useMessage } from "naive-ui";
import { h } from "vue";

import { useConfirmDialog } from "@/components/useConfirmDialog";
import type { ComputedRef } from "vue";
export function useBackup(
  workspace: ReturnType<typeof useWorkspace>,
  hasRequests: ComputedRef<boolean>,
) {
  const { saveError, restoreBackup, save } = workspace;
  const message = useMessage();
  const confirm = useConfirmDialog();
  function backup() {
    confirm(
      "导出完整备份",
      () =>
        h(
          "div",
          {
            style: {
              display: "flex",
              flexDirection: "column",
              gap: "16px",
              lineHeight: "1.7",
            },
          },
          [
            h(
              "p",
              { style: { margin: "0" } },
              "导出为 JSON 文件，可在新电脑上恢复完整数据。",
            ),
            h(
              "ul",
              {
                style: {
                  margin: "0",
                  paddingLeft: "20px",
                  display: "grid",
                  gap: "6px",
                },
              },
              [
                h("li", "项目、草稿、聊天记录及用户模板"),
                h("li", "供应商配置、API Key 及应用设置"),
                h("li", "用量记录、价格目录及目录缓存"),
              ],
            ),
            h(
              "div",
              {
                style: { display: "flex", flexDirection: "column", gap: "4px" },
              },
              [
                h(
                  NText,
                  {
                    type: "warning",
                    strong: true,
                    style: { fontSize: "15px" },
                  },
                  { default: () => "API Key 将以明文保存" },
                ),
                h(
                  NText,
                  { depth: 2 },
                  { default: () => "请妥善保管备份文件，不要公开分享。" },
                ),
              ],
            ),
          ],
        ),
      async () => {
        if (hasRequests.value) throw new Error("请等待当前优化完成后再备份。");
        await save();
        if (saveError.value) throw new Error(saveError.value);
        const path = await pickSave({
          defaultPath: "词稿完整备份.json",
          filters: [{ name: "词稿完整备份", extensions: ["json"] }],
        });
        if (!path) return;
        await desktop.exportBackup(path);
        message.success("完整备份已导出");
      },
      "继续导出",
    );
  }

  async function restore() {
    try {
      if (hasRequests.value) throw new Error("请等待当前优化完成后再恢复。");
      const path = await open({
        multiple: false,
        filters: [{ name: "词稿完整备份", extensions: ["json"] }],
      });
      if (!path) return;
      const incoming = await desktop.previewBackup(path);
      confirm(
        "恢复完整备份",
        () =>
          h(
            "div",
            {
              style: {
                display: "flex",
                flexDirection: "column",
                gap: "16px",
                lineHeight: "1.7",
              },
            },
            [
              h(
                "p",
                { style: { margin: "0" } },
                "备份已通过校验，将恢复以下数据及全部配置。",
              ),
              h(
                "ul",
                {
                  style: {
                    margin: "0",
                    paddingLeft: "20px",
                    display: "grid",
                    gap: "6px",
                  },
                },
                [
                  h(
                    "li",
                    `${incoming.projects} 个项目、${incoming.drafts} 份草稿、${incoming.messages} 条聊天消息`,
                  ),
                  h(
                    "li",
                    `${incoming.providers} 个供应商及 API Key、用户模板和应用设置`,
                  ),
                  h(
                    "li",
                    `${incoming.usage} 条用量记录、${incoming.prices} 条价格及目录缓存`,
                  ),
                ],
              ),
              h(
                "div",
                {
                  style: {
                    display: "flex",
                    flexDirection: "column",
                    gap: "4px",
                  },
                },
                [
                  h(
                    NText,
                    {
                      type: "warning",
                      strong: true,
                      style: { fontSize: "15px" },
                    },
                    {
                      default: () =>
                        "将覆盖当前全部数据、API Key 和设置，不会合并",
                    },
                  ),
                  h(
                    NText,
                    { depth: 2 },
                    { default: () => "建议先导出当前备份，再执行恢复。" },
                  ),
                ],
              ),
              h(
                NText,
                { depth: 3, style: { fontSize: "12px" } },
                {
                  default: () =>
                    "恢复成功后界面将重新加载。新电脑的开机自启需要重新设置。",
                },
              ),
            ],
          ),
        async () => {
          if (hasRequests.value)
            throw new Error("请等待当前优化完成后再恢复。");
          await save();
          if (saveError.value) throw new Error(saveError.value);
          await restoreBackup(path);
        },
        "覆盖并恢复",
      );
    } catch (error) {
      message.error(errorMessage(error));
    }
  }

  return { backup, restore };
}
