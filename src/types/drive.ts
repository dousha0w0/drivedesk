/** Mirrors `src-tauri/src/types.rs`. Keep the two in lockstep. */

export type IconType =
  | "folder"
  | "doc"
  | "sheet"
  | "slide"
  | "draw"
  | "form"
  | "code"
  | "shortcut"
  | "pdf"
  | "image"
  | "video"
  | "audio"
  | "text"
  | "archive"
  | "office"
  | "file";

export interface FileMeta {
  id: string;
  name: string;
  mimeType: string;
  size: number | null;
  modifiedTime: string | null;
  createdTime: string | null;
  owner: string | null;
  ownerEmail: string | null;
  parents: string[];
  starred: boolean;
  trashed: boolean;
  md5: string | null;
  webViewLink: string | null;
  thumbnailLink: string | null;
  iconType: IconType;
  shared: boolean;
  canEdit: boolean;
  canDownload: boolean;
}

export type ViewKind = "folder" | "recent" | "starred" | "trash" | "shared" | "search";

export type SortKey = "name" | "size" | "modified" | "type";
export type SortDirection = "asc" | "desc";

export interface ListRequest {
  view: ViewKind;
  folderId?: string | null;
  query?: string | null;
  pageToken?: string | null;
  pageSize?: number | null;
  sortBy?: SortKey | null;
  sortDir?: SortDirection | null;
}

export interface ListResponse {
  files: FileMeta[];
  nextPageToken: string | null;
  total: number | null;
}

export interface Crumb {
  id: string;
  name: string;
}

export interface Quota {
  limit: number | null;
  usage: number | null;
  driveUsage: number | null;
  user: string | null;
}

export interface Account {
  email: string;
  name: string;
  picture: string | null;
  isDefault: boolean;
  hasClient: boolean;
}

export interface AppInfo {
  version: string;
  mock: boolean;
  dataDir: string;
  cacheDir: string;
  defaultDownloadDir: string;
  secretStorage: string;
  accounts: Account[];
  /** True when this build carries an OAuth client (one-click sign-in). */
  builtinClient: boolean;
  gdirectFound: boolean;
  gdirectPath: string | null;
  gdirectVersion: string | null;
  platform: string;
}

export type PreviewKind = "text" | "image" | "pdf" | "video" | "audio" | "office" | "unsupported";

export interface PreviewInfo {
  kind: PreviewKind;
  mime: string;
  size: number | null;
  path: string | null;
  text: string | null;
  truncated: boolean;
  message: string | null;
}

export interface GdirectInfo {
  found: boolean;
  path: string | null;
  version: string | null;
}

export interface SmokeConfig {
  enabled: boolean;
  output: string | null;
  keepOpenMs: number;
}

export type TransferKind = "download" | "upload";
export type TransferStatus = "queued" | "running" | "done" | "error" | "canceled";

export interface Transfer {
  id: string;
  kind: TransferKind;
  name: string;
  fileId: string | null;
  localPath: string | null;
  destDir: string | null;
  status: TransferStatus;
  received: number;
  total: number | null;
  speedBps: number;
  error: string | null;
  startedAt: number | null;
  finishedAt: number | null;
  expandedCount: number | null;
}

export interface BackendSettings {
  downloadDir: string;
  downloadConcurrency: number;
  uploadConcurrency: number;
  gdirectEnabled: boolean;
  gdirectPath: string | null;
}

export interface SettingsPatch {
  downloadDir?: string;
  downloadConcurrency?: number;
  uploadConcurrency?: number;
  gdirectEnabled?: boolean;
  gdirectPath?: string | null;
}

export type ThemeMode = "light" | "dark" | "system";
export type Palette = "neutral" | "graphite" | "vscode";
export type CornerStyle = "default" | "small" | "none";
export type LayoutMode = "classic" | "separated";
export type Density = "comfortable" | "compact";
export type RightPanelTab = "preview" | "details";

export interface UiPreferences {
  themeMode: ThemeMode;
  palette: Palette;
  corners: CornerStyle;
  layout: LayoutMode;
  density: Density;
  rightPanelTab: RightPanelTab;
  rightPanelOpen: boolean;
  transferDockOpen: boolean;
  sidebarWidth: number;
  rightPanelWidth: number;
  dockHeight: number;
}
