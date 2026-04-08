import { writable } from "svelte/store";
import type { DownloadTask, AppSettings, VideoInfo } from "./types";

// Current view: 'browser' | 'downloads' | 'settings' | 'video'
export const currentView = writable<'browser' | 'downloads' | 'settings' | 'video'>('browser');

// Theme: 'light' | 'dark'
export const theme = writable<'light' | 'dark'>('light');

// Download tasks
export const downloadTasks = writable<DownloadTask[]>([]);

// Current video info from browser
export const currentVideoInfo = writable<VideoInfo | null>(null);

// URL of the video currently loaded in the video-webview
export const videoUrl = writable<string>('');

// App settings
export const settings = writable<AppSettings>({
  downloadPath: '',
  namingPattern: '{courseName} - {subTitle} - {lecturerName}',
  autoDownload: false,
  maxConcurrentDownloads: 3,
  defaultQuality: 'highest',
  extractAudio: false,
  audioFormat: 'mp3',
});

// Browser navigation state
export const browserState = writable({
  url: 'https://course.pku.edu.cn',
  canGoBack: false,
  canGoForward: false,
  isLoading: false,
  title: '教学网',
});
