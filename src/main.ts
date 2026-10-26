import { createApp } from "vue";
import { createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import App from "./App.vue";
import { i18n } from "./i18n";
import "./styles/globals.css";

const report = (level: string, message: string) => {
  void invoke("log_message", { level, message }).catch(() => {});
};

window.addEventListener("error", (event) => {
  report("error", `${event.message} @ ${event.filename}:${event.lineno}:${event.colno}`);
});
window.addEventListener("unhandledrejection", (event) => {
  report("error", `unhandled rejection: ${String(event.reason)}`);
});

report("info", `bundle booting (${navigator.userAgent})`);

createApp(App).use(createPinia()).use(i18n).mount("#app");

report("info", "vue mounted");
