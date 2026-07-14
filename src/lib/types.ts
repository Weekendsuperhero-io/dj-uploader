export type Platform = "mixcloud" | "soundcloud";

export interface PlatformStatus {
  connected: boolean;
  expires_in_seconds: number | null;
  needs_refresh: boolean;
}

export interface AuthStatus {
  mixcloud: PlatformStatus;
  soundcloud: PlatformStatus;
}

/** Where a track landed (from the Rust `UploadOutcome`). */
export interface UploadOutcome {
  platform: string;
  title: string;
  url: string;
  success: boolean;
  error: string | null;
}

/** Byte-level upload progress (from the Rust `UploadProgress`). */
export interface UploadProgress {
  platform: string;
  sent: number;
  total: number;
}

/** A transient failure triggered an automatic retry (Rust `UploadRetry`). */
export interface UploadRetry {
  platform: string;
  /** The attempt about to run (1-based). */
  attempt: number;
  maxAttempts: number;
  /** Seconds until the next attempt starts. */
  delaySecs: number;
  reason: string;
}

/** Mirrors the Rust `UploadParams` (serde camelCase). */
export interface UploadParams {
  filePath: string;
  title: string;
  description: string;
  imagePath: string;
  tags: string;
  mixcloud: boolean;
  soundcloud: boolean;
  scheduleEnabled: boolean;
  scheduleDate: string;
  scheduleTime: string;
  generatePreviews: boolean;
  /** Pin uploads to HTTP/1.1 — compatibility mode for flaky networks. */
  forceHttp1: boolean;
}
