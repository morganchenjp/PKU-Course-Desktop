import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { v4 as uuidv4 } from "uuid";
import { settings } from "./store";
import { generateFilename, formatDate } from "./naming";
import type { VideoInfo, DownloadTask } from "./types";

/**
 * Create a DownloadTask from VideoInfo, resolving paths and filenames
 * using the current app settings.
 */
export async function createDownloadTask(videoInfo: VideoInfo): Promise<DownloadTask> {
  const currentSettings = get(settings);
  const pattern = currentSettings.namingPattern;

  const filename = generateFilename(pattern, {
    courseName: videoInfo.courseName,
    subTitle: videoInfo.subTitle,
    lecturerName: videoInfo.lecturerName,
    date: formatDate(),
  });

  const downloadPath =
    currentSettings.downloadPath || (await invoke<string>("get_default_download_path"));

  // Resolve M3U8 URLs to the Blackboard download API endpoint (requires resourceId + token)
  const resolvedUrl = videoInfo.isM3u8 && videoInfo.resourceId
    ? `https://course.pku.edu.cn/webapps/bb-streammedia-hqy-BBLEARN/downloadVideo.action?resourceId=${videoInfo.resourceId}${videoInfo.jwt ? '&token=' + encodeURIComponent(videoInfo.jwt) : ''}`
    : videoInfo.downloadUrl;

  return {
    id: uuidv4(),
    videoInfo: {
      ...videoInfo,
      downloadUrl: resolvedUrl,
    },
    filename,
    filepath: `${downloadPath}/${filename}`,
    status: "pending" as const,
    progress: 0,
    speed: "-",
    eta: "-",
    createdAt: Date.now(),
  };
}
