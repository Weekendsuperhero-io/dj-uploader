import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type {
  AuthStatus,
  Platform,
  UploadOutcome,
  UploadParams,
  UploadProgress,
  UploadRetry,
} from "./types";

/** Per-platform authorization status from the on-disk token store. */
export function getAuthStatus(): Promise<AuthStatus> {
  return invoke<AuthStatus>("get_auth_status");
}

/** Run the browser OAuth flow for a platform (blocks in Rust until the callback). */
export function connectPlatform(platform: Platform): Promise<void> {
  return invoke("connect_platform", { platform });
}

/** Forget the stored token for a platform. */
export function disconnectPlatform(platform: Platform): Promise<void> {
  return invoke("disconnect_platform", { platform });
}

/** Upload to the selected platforms; resolves with one outcome (URL) per platform. */
export function upload(params: UploadParams): Promise<UploadOutcome[]> {
  return invoke<UploadOutcome[]>("upload", { params });
}

/** Ask the in-flight upload to stop (aborts the current attempt and any retries). */
export function cancelUpload(): Promise<void> {
  return invoke("cancel_upload");
}

/** Subscribe to human-readable upload stage messages. */
export function onUploadStage(cb: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>("upload-stage", (e) => cb(e.payload));
}

/** Subscribe to byte-level upload progress (for the progress bar). */
export function onUploadProgress(cb: (p: UploadProgress) => void): Promise<UnlistenFn> {
  return listen<UploadProgress>("upload-progress", (e) => cb(e.payload));
}

/** Subscribe to automatic-retry notifications (for the "reconnecting…" state). */
export function onUploadRetry(cb: (r: UploadRetry) => void): Promise<UnlistenFn> {
  return listen<UploadRetry>("upload-retry", (e) => cb(e.payload));
}

async function pickFile(name: string, extensions: string[]): Promise<string | null> {
  const res = await open({
    multiple: false,
    directory: false,
    filters: [{ name, extensions }],
  });
  return typeof res === "string" ? res : null;
}

export function pickAudioFile(): Promise<string | null> {
  return pickFile("Audio", ["mp3", "m4a", "wav", "flac"]);
}

export function pickImageFile(): Promise<string | null> {
  return pickFile("Image", ["jpg", "jpeg", "png"]);
}

/** Check GitHub releases for an update. Returns null if none or on error. */
export async function checkForUpdate(): Promise<Update | null> {
  try {
    return await check();
  } catch (e) {
    console.warn("Update check failed:", e);
    return null;
  }
}

/** Download + install an update, then relaunch. */
export async function installUpdate(update: Update): Promise<void> {
  await update.downloadAndInstall();
  await relaunch();
}

export { openUrl };
export type { Update };
