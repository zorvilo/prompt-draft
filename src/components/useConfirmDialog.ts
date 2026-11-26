import { errorMessage } from "@/api/desktop";
import { useDialog, useMessage } from "naive-ui";
import { h } from "vue";
export function useConfirmDialog() {
  const dialog = useDialog();
  const message = useMessage();
  function confirm(
    title: string,
    content: string | (() => ReturnType<typeof h>),
    action: () => Promise<void>,
    positiveText = "确认",
  ) {
    dialog.warning({
      title,
      content,
      positiveText,
      negativeText: "取消",
      showIcon: false,
      onPositiveClick: async () => {
        try {
          await action();
        } catch (error) {
          message.error(errorMessage(error));
          return false;
        }
      },
    });
  }

  return confirm;
}
