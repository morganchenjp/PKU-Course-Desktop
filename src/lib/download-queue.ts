import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { downloadTasks, settings } from "./store";
import { createDownloadTask } from "./download-utils";
import type { VideoInfo } from "./types";

// Deduplication window: ignore duplicate download requests for the same URL
// within this many milliseconds.  This prevents the triple-fire on WebView2
// caused by the iframe direct XHR plus two top-frame postMessage relays.
const DEDUP_MS = 3000;
const recentDownloads = new Map<string, number>();

function isDuplicate(url: string): boolean {
  const last = recentDownloads.get(url);
  if (last && Date.now() - last < DEDUP_MS) {
    return true;
  }
  recentDownloads.set(url, Date.now());
  return false;
}

/**
 * Hand a freshly detected video off to the Rust download pipeline,
 * respecting the user's max-concurrent-downloads setting.
 *
 * If we're under the slot limit, the task is added in `downloading` state
 * and `browser_download` is invoked immediately.  Otherwise it's added in
 * `pending` state — the next slot to free up will pick it up via
 * `startNextPendingDownload()`.
 */
export async function enqueueDownload(videoInfo: VideoInfo): Promise<void> {
  if (isDuplicate(videoInfo.downloadUrl)) {
    console.log("[download-queue] dedup: skipping duplicate for", videoInfo.downloadUrl);
    return;
  }

  const task = await createDownloadTask(videoInfo);
  const currentTasks = get(downloadTasks);
  const currentSettings = get(settings);
  const downloadingCount = currentTasks.filter((t) => t.status === "downloading").length;

  if (downloadingCount < currentSettings.maxConcurrentDownloads) {
    // Under limit: start immediately
    const startedTask = { ...task, status: "downloading" as const, startedAt: Date.now() };
    downloadTasks.update((tasks) => [...tasks, startedTask]);
    console.log("[download-queue] starting:", task.filename);
    await invoke("browser_download", {
      taskId: startedTask.id,
      url: startedTask.videoInfo.downloadUrl,
      filepath: startedTask.filepath,
    });
  } else {
    // At limit: queue as pending
    const pendingTask = { ...task, status: "pending" as const };
    downloadTasks.update((tasks) => [...tasks, pendingTask]);
    console.log("[download-queue] queued (at limit):", task.filename);
  }
}

/**
 * After a download completes or errors out, try to promote the oldest
 * pending task to `downloading` so the slot doesn't sit idle.  No-op if
 * we're already at the limit or if there are no pending tasks.
 */
export async function startNextPendingDownload(): Promise<void> {
  const currentTasks = get(downloadTasks);
  const currentSettings = get(settings);
  const downloadingCount = currentTasks.filter((t) => t.status === "downloading").length;
  if (downloadingCount >= currentSettings.maxConcurrentDownloads) return;

  const pending = currentTasks.find((t) => t.status === "pending");
  if (!pending) return;

  downloadTasks.update((tasks) =>
    tasks.map((t) =>
      t.id === pending.id
        ? { ...t, status: "downloading" as const, startedAt: Date.now() }
        : t,
    ),
  );

  try {
    await invoke("browser_download", {
      taskId: pending.id,
      url: pending.videoInfo.downloadUrl,
      filepath: pending.filepath,
    });
    console.log("[download-queue] started queued:", pending.filename);
  } catch (e) {
    console.error("[download-queue] failed to start queued download:", e);
  }
}
