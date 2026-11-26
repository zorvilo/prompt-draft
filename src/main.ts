import { invoke } from "@tauri-apps/api/core";
import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

try {
  createApp(App).mount("#app");
} catch {
  const root = document.getElementById("app");
  if (root) {
    root.replaceChildren();
    const panel = document.createElement("div");
    panel.className = "boot-screen";
    panel.setAttribute("role", "alert");
    const text = document.createElement("p");
    text.textContent = "启动失败，请重新加载。";
    const retry = document.createElement("button");
    retry.textContent = "重新加载";
    retry.addEventListener("click", () => window.location.reload());
    panel.append(text, retry);
    root.append(panel);
    void invoke("window_action", { action: "show" }).catch(() => {});
  }
}
