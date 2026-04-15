<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { browserState, currentVideoInfo } from "../lib/store";
  import type { VideoInfo } from "../lib/types";
  import VideoInfoCard from "./VideoInfoCard.svelte";

  let unlistenMessage: (() => void) | null = null;
  let showVideoCard = $state(false);

  onMount(async () => {
    try {
      // Listen for messages from the browser webview (video detection, page loads)
      unlistenMessage = await listen("webview-message", (event: any) => {
        handleWebviewMessage(event.payload);
      });

      browserState.update(state => ({
        ...state,
        url: 'https://course.pku.edu.cn',
        isLoading: false,
      }));
    } catch (error) {
      console.error("[BrowserView] init failed:", error);
    }
  });

  onDestroy(() => {
    if (unlistenMessage) unlistenMessage();
  });

  function handleWebviewMessage(payload: any) {
    switch (payload.type) {
      case 'video-info': {
        const videoInfo: VideoInfo = payload.data;
        currentVideoInfo.set(videoInfo);
        showVideoCard = true;
        break;
      }
      case 'page-loaded':
        browserState.update(state => ({
          ...state,
          isLoading: false,
          title: payload.data.title || state.title,
          url: payload.data.url || state.url,
        }));
        break;
      case 'navigation-state':
        browserState.update(state => ({
          ...state,
          canGoBack: payload.data.canGoBack,
          canGoForward: payload.data.canGoForward,
        }));
        break;
    }
  }

  function closeVideoCard() {
    showVideoCard = false;
  }

  function goToDownloads() {
    invoke('show_main_view', { view: 'downloads' }).catch(() => {});
  }
</script>

<!-- BrowserView is now just a thin wrapper: the actual browser UI is the
     browser-webview with its injected nav-bar.  This component only listens
     for events and renders the VideoInfoCard overlay when needed. -->
<div class="browser-view">
  {#if showVideoCard && $currentVideoInfo}
    <VideoInfoCard
      videoInfo={$currentVideoInfo}
      onClose={closeVideoCard}
      onDownload={goToDownloads}
    />
  {/if}
</div>

<style>
  .browser-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    position: relative;
  }
</style>
