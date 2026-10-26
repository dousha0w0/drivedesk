import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Account,
  AppInfo,
  BackendSettings,
  Crumb,
  FileMeta,
  GdirectInfo,
  ListRequest,
  ListResponse,
  PreviewInfo,
  Quota,
  SettingsPatch,
  SmokeConfig,
  Transfer,
} from "@/types/drive";

/**
 * The only place the frontend talks to Rust. Every command name appears once,
 * so the IPC surface is easy to audit against `src-tauri/src/commands.rs`.
 */
export const api = {
  log: (level: string, message: string) => invoke<void>("log_message", { level, message }),
  appInfo: () => invoke<AppInfo>("app_info"),
  appReady: () => invoke<void>("app_ready"),

  accountsList: () => invoke<Account[]>("accounts_list"),
  accountLogin: (clientJson?: string | null) => invoke<Account>("account_login", { clientJson: clientJson ?? null }),
  accountSwitch: (email: string) => invoke<void>("account_switch", { email }),
  accountLogout: (email: string) => invoke<void>("account_logout", { email }),
  quotaGet: () => invoke<Quota>("quota_get"),

  filesList: (request: ListRequest) => invoke<ListResponse>("files_list", { request }),
  fileGet: (id: string) => invoke<FileMeta>("file_get", { id }),
  filePath: (id: string) => invoke<Crumb[]>("file_path", { id }),
  folderCreate: (parentId: string, name: string) => invoke<FileMeta>("folder_create", { parentId, name }),
  fileRename: (id: string, name: string) => invoke<FileMeta>("file_rename", { id, name }),
  fileMove: (id: string, newParent: string) => invoke<FileMeta>("file_move", { id, newParent }),
  fileCopy: (id: string, parent: string) => invoke<FileMeta>("file_copy", { id, parent }),
  fileTrash: (id: string, trashed: boolean) => invoke<FileMeta>("file_trash", { id, trashed }),
  fileStar: (id: string, starred: boolean) => invoke<FileMeta>("file_star", { id, starred }),
  fileDelete: (id: string) => invoke<void>("file_delete", { id }),

  downloadStart: (ids: string[], destDir?: string | null) =>
    invoke<void>("download_start", { ids, destDir: destDir ?? null }),
  uploadStart: (parentId: string, paths: string[]) => invoke<void>("upload_start", { parentId, paths }),

  transfersList: () => invoke<Transfer[]>("transfers_list"),
  transferCancel: (id: string) => invoke<void>("transfer_cancel", { id }),
  transferRetry: (id: string) => invoke<void>("transfer_retry", { id }),
  transfersClearFinished: () => invoke<void>("transfers_clear_finished"),

  previewPrepare: (id: string) => invoke<PreviewInfo>("preview_prepare", { id }),

  settingsGet: () => invoke<BackendSettings>("settings_get"),
  settingsUpdate: (patch: SettingsPatch) => invoke<BackendSettings>("settings_update", { patch }),
  cacheClear: () => invoke<void>("cache_clear"),

  gdirectProbe: (path?: string | null) => invoke<GdirectInfo>("gdirect_probe", { path: path ?? null }),

  systemOpenPath: (path: string) => invoke<void>("system_open_path", { path }),
  systemOpenUrl: (url: string) => invoke<void>("system_open_url", { url }),

  smokeConfig: () => invoke<SmokeConfig>("smoke_config"),
  smokeFixture: () => invoke<string>("smoke_fixture"),
  smokeReport: (report: string) => invoke<void>("smoke_report", { report }),
};

export const TRANSFER_EVENT = "drive://transfer";

export function onTransfer(listener: (transfer: Transfer) => void): Promise<UnlistenFn> {
  return listen<Transfer>(TRANSFER_EVENT, (event) => listener(event.payload));
}

/** Absolute path -> URL the webview may load from the asset protocol. */
export function assetUrl(path: string): string {
  return convertFileSrc(path);
}

