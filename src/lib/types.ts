export interface VideoInfo {
  courseName: string;
  subTitle: string;
  lecturerName: string;
  downloadUrl: string;
  isM3u8: boolean;
  m3u8Url?: string;
  resourceId?: string;
  jwt?: string;
  timestamp: number;
}

export interface DownloadTask {
  id: string;
  videoInfo: VideoInfo;
  filename: string;
  filepath: string;
  status: 'pending' | 'downloading' | 'paused' | 'completed' | 'error';
  progress: number;
  speed: string;
  eta: string;
  error?: string;
  createdAt: number;
  startedAt?: number;
  completedAt?: number;
}

export interface AppSettings {
  downloadPath: string;
  namingPattern: string;
  autoDownload: boolean;
  maxConcurrentDownloads: number;
  defaultQuality: 'highest' | 'high' | 'medium' | 'low';
  extractAudio: boolean;
  audioFormat: 'mp3' | 'aac' | 'wav';
}

export interface NamingPatternVars {
  courseName: string;
  subTitle: string;
  lecturerName: string;
  date: string;
  index?: number;
}

export interface BrowserMessage {
  type: 'video-info' | 'page-loaded' | 'navigation-state';
  data: any;
}
