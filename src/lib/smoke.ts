import { api, assetUrl } from "@/lib/api";
import { useFilesStore } from "@/stores/files";
import { useSessionStore } from "@/stores/session";
import { useTransfersStore } from "@/stores/transfers";
import { useUiStore } from "@/stores/ui";
import { errorMessage } from "@/lib/errors";
import type { FileMeta } from "@/types/drive";

interface Step {
  name: string;
  ok: boolean;
  detail?: string;
}

const wait = (ms: number): Promise<void> => {
  const { promise, resolve } = Promise.withResolvers<void>();
  window.setTimeout(resolve, ms);
  return promise;
};

/**
 * Scripted end-to-end run used by `DRIVEDESK_SMOKE=1`.
 *
 * It drives the same store actions the UI calls (no synthetic DOM events), so a
 * green report means the IPC surface, the mock Drive, the transfer queue and the
 * rendered shell all work together. The report is written by the Rust side and
 * the app exits.
 */
export async function runSmoke(): Promise<void> {
  const session = useSessionStore();
  const files = useFilesStore();
  const transfers = useTransfersStore();
  const ui = useUiStore();
  const steps: Step[] = [];

  const check = (name: string, ok: boolean, detail?: string) => {
    steps.push({ name, ok, detail });
  };

  const findBySuffix = (suffix: string): FileMeta | undefined =>
    files.entries.find((entry) => entry.name.endsWith(suffix));

  const waitForTransfers = async (count: number, timeoutMs = 20_000): Promise<boolean> => {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const finished = transfers.items.filter((item) => ["done", "error", "canceled"].includes(item.status));
      if (finished.length >= count) return finished.every((item) => item.status === "done");
      await wait(120);
    }
    return false;
  };

  try {
    check("session.accounts", session.accounts.length > 0, `${session.accounts.length} account(s)`);
    check("session.mock", session.isMock, `mock=${session.isMock}`);

    await files.navigate({ view: "folder", folderId: "root", query: "" });
    check("drive.rootEntries", files.entries.length >= 5, `${files.entries.length} entries`);
    check("drive.rootCrumbs", files.crumbs.length === 1, files.crumbs.map((crumb) => crumb.name).join("/"));

    const photos = files.entries.find((entry) => entry.name === "照片");
    if (photos) {
      await files.open(photos);
      check("drive.openFolder", files.entries.length === 40, `${files.entries.length} photos`);
      check("drive.crumbsDeep", files.crumbs.length === 2, files.crumbs.map((crumb) => crumb.name).join("/"));
    } else {
      check("drive.openFolder", false, "照片 folder missing");
    }

    await files.goBack();
    check("drive.goBack", files.entries.length >= 5 && files.crumbs.length === 1, `${files.entries.length} entries`);

    const design = files.entries.find((entry) => entry.name === "设计");
    if (design) await files.open(design);
    const photo = files.entries.find((entry) => entry.name === "封面.svg");
    if (photo) {
      await files.showPreview(photo.id);
      check("preview.imageKind", files.preview?.kind === "image", `kind=${files.preview?.kind}`);
      const path = files.preview?.path;
      const decoded = await new Promise<boolean>((resolve) => {
        const image = new Image();
        image.onload = () => resolve(image.naturalWidth > 0);
        image.onerror = () => resolve(false);
        image.src = path ? assetUrl(path) : "";
      });
      check("preview.assetProtocol", decoded, path ?? "no path");
    } else {
      check("preview.imageKind", false, "封面.svg missing");
    }

    await files.navigate({ view: "folder", folderId: "root", query: "" });
    await files.setSort("size");
    const filesOnly = files.visibleEntries.filter((entry) => entry.iconType !== "folder");
    const sizes = filesOnly.map((entry) => entry.size ?? -1);
    check(
      "table.sortBySize",
      files.sort.key === "size" && files.sort.direction === "desc" && sizes.every((size, index) => index === 0 || sizes[index - 1] >= size),
      `first=${sizes[0]} last=${sizes[sizes.length - 1]}`,
    );

    files.setFilter("预算");
    check("table.filter", files.visibleEntries.length === 1, `${files.visibleEntries.length} match`);
    files.setFilter("");

    const readme = findBySuffix("README.md");
    if (readme) {
      await files.showPreview(readme.id);
      check(
        "preview.text",
        files.preview?.kind === "text" && (files.preview.text ?? "").includes("DriveDesk"),
        `kind=${files.preview?.kind}`,
      );

      await files.download([readme.id]);
      const downloaded = await waitForTransfers(1);
      const transfer = transfers.items.find((item) => item.kind === "download");
      check("transfer.download", downloaded && (transfer?.received ?? 0) > 0, `${transfer?.status} ${transfer?.received}/${transfer?.total}`);

      const fixture = await api.smokeFixture();
      await files.upload([fixture]);
      const uploaded = await waitForTransfers(2);
      await files.reload();
      check(
        "transfer.upload",
        uploaded && files.entries.some((entry) => entry.name.startsWith("smoke-upload")),
        files.entries.map((entry) => entry.name).join(","),
      );
    } else {
      check("preview.text", false, "README.md missing");
      check("transfer.download", false, "README.md missing");
    }

    await files.createFolder("Smoke 测试");
    const created = findBySuffix("Smoke 测试");
    check("folder.create", Boolean(created), `${files.entries.length} entries`);
    if (created) {
      await files.rename(created.id, "Smoke 已改名");
      check("folder.rename", files.entries.some((entry) => entry.name === "Smoke 已改名"), "");

      const renamed = files.entries.find((entry) => entry.name === "Smoke 已改名");
      if (renamed) {
        await files.trash([renamed.id]);
        check("folder.trash", !files.entries.some((entry) => entry.id === renamed.id), "");

        await files.navigate({ view: "trash", query: "" });
        const inTrash = files.entries.find((entry) => entry.id === renamed.id);
        check("trash.listing", Boolean(inTrash), `${files.entries.length} trashed`);
        if (inTrash) {
          await files.deleteForever([inTrash.id]);
          check("trash.deleteForever", !files.entries.some((entry) => entry.id === inTrash.id), "");
        }
        await files.navigate({ view: "folder", folderId: "root", query: "" });
      }
    }

    await files.search("README");
    check("search.global", files.view === "search" && files.entries.length > 0, `${files.entries.length} results`);
    await files.navigate({ view: "folder", folderId: "root", query: "" });

    ui.setPrefs({ layout: "separated" });
    await wait(60);
    const separated = document.querySelector(".app-body")?.getAttribute("data-layout") === "separated";
    ui.setPrefs({ layout: "classic" });
    await wait(60);
    const classic = document.querySelector(".app-body")?.getAttribute("data-layout") === "classic";
    check("layout.modes", separated && classic, `separated=${separated} classic=${classic}`);

    check("shell.statusBar", Boolean(document.querySelector('[data-testid="status-bar"]')), "");
    check("shell.rows", document.querySelectorAll(".file-table__row").length > 0, `${document.querySelectorAll(".file-table__row").length} rows in DOM`);
  } catch (caught) {
    check("smoke.exception", false, errorMessage(caught));
  }

  const report = { ok: steps.every((step) => step.ok), steps };
  await api.smokeReport(JSON.stringify(report));
}
