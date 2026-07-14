import {
  type MouseEvent as ReactMouseEvent,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { toast } from "sonner";
import { SoundcloudLogo } from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  CalendarClock,
  CheckCircle2,
  Download,
  ImageIcon,
  Loader2,
  Music,
  Scissors,
  UploadCloud,
  Wifi,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Button as GlassButton } from "@/components/ui/glass/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Toaster } from "@/components/ui/sonner";
import { DatePickerInput } from "@/components/ui/date-picker-input";
import { format } from "date-fns";
import { cn } from "@/lib/utils";
import {
  cancelUpload,
  checkForUpdate,
  connectPlatform,
  getAuthStatus,
  installUpdate,
  onUploadProgress,
  onUploadRetry,
  onUploadStage,
  openUrl,
  pickAudioFile,
  pickImageFile,
  upload,
  type Update,
} from "@/lib/api";
import type { AuthStatus, Platform, UploadOutcome } from "@/lib/types";

/** Live state for the upload progress area. */
type UploadProgressState = {
  platform: string;
  pct: number;
  /** `uploading` streams bytes; `retrying` is waiting to reconnect; `cancelling` is stopping. */
  phase: "uploading" | "retrying" | "cancelling";
  retry?: { attempt: number; maxAttempts: number; delaySecs: number };
};

/** True when a failed outcome is the result of the user cancelling. */
function isCancelled(o: UploadOutcome): boolean {
  return !o.success && (o.error ?? "").toLowerCase().includes("cancel");
}

function basename(p: string): string {
  const parts = p.split(/[\\/]/);
  return parts[parts.length - 1] || p;
}

function platformLabel(p: Platform): string {
  return p === "mixcloud" ? "Mixcloud" : "SoundCloud";
}

function ConnectedBadge() {
  return (
    <Badge variant="secondary" className="gap-1.5 text-emerald-300">
      <span className="size-2 rounded-full bg-emerald-400" />
      Connected
    </Badge>
  );
}

function startWindowDrag(event: ReactMouseEvent<HTMLElement>) {
  if (event.button !== 0) return;
  event.preventDefault();
  void getCurrentWindow().startDragging();
}

export default function App() {
  const layoutRef = useRef<HTMLElement>(null);
  const [uiScale, setUiScale] = useState(1);
  const [filePath, setFilePath] = useState("");
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState("");
  const [imagePath, setImagePath] = useState("");
  const [mixcloudEnabled, setMixcloudEnabled] = useState(true);
  const [soundcloudEnabled, setSoundcloudEnabled] = useState(false);
  const [scheduleEnabled, setScheduleEnabled] = useState(false);
  const [scheduleDate, setScheduleDate] = useState("");
  const [scheduleTime, setScheduleTime] = useState("");
  const [generatePreviews, setGeneratePreviews] = useState(false);

  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [connecting, setConnecting] = useState<Platform | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [progress, setProgress] = useState<UploadProgressState | null>(null);
  const [results, setResults] = useState<UploadOutcome[]>([]);
  const [update, setUpdate] = useState<Update | null>(null);
  const [installing, setInstalling] = useState(false);

  const refreshAuth = async () => {
    try {
      setAuth(await getAuthStatus());
    } catch (e) {
      console.warn("Failed to load auth status:", e);
    }
  };

  useEffect(() => {
    refreshAuth();
    checkForUpdate().then(setUpdate);
    const unlistenStage = onUploadStage((msg) => toast.message(msg));
    const unlistenProgress = onUploadProgress((p) =>
      setProgress((prev) => ({
        platform: p.platform,
        pct: p.total > 0 ? Math.round((p.sent / p.total) * 100) : 0,
        // Bytes are flowing again → back to uploading, unless the user is
        // actively cancelling (keep that state until the upload returns).
        phase: prev?.phase === "cancelling" ? "cancelling" : "uploading",
        retry: undefined,
      })),
    );
    const unlistenRetry = onUploadRetry((r) =>
      setProgress((prev) => ({
        platform: r.platform,
        // The next attempt restarts from the beginning.
        pct: 0,
        phase: prev?.phase === "cancelling" ? "cancelling" : "retrying",
        retry: {
          attempt: r.attempt,
          maxAttempts: r.maxAttempts,
          delaySecs: r.delaySecs,
        },
      })),
    );
    return () => {
      unlistenStage.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      unlistenRetry.then((fn) => fn());
    };
  }, []);

  useLayoutEffect(() => {
    const layout = layoutRef.current;
    if (!layout) return;

    let animationFrame = 0;
    const updateScale = () => {
      cancelAnimationFrame(animationFrame);
      animationFrame = requestAnimationFrame(() => {
        const contentHeight = layout.scrollHeight;
        const nextScale = Math.max(
          0.5,
          Math.min(
            1,
            window.innerWidth / 620,
            (window.innerHeight - 8) / contentHeight,
          ),
        );
        setUiScale((currentScale) =>
          Math.abs(currentScale - nextScale) < 0.001
            ? currentScale
            : nextScale,
        );
      });
    };

    const observer = new ResizeObserver(updateScale);
    observer.observe(layout);
    updateScale();
    window.addEventListener("resize", updateScale);
    return () => {
      cancelAnimationFrame(animationFrame);
      observer.disconnect();
      window.removeEventListener("resize", updateScale);
    };
  }, []);

  const soundcloudConnected = auth?.soundcloud.connected ?? false;
  const mixcloudConnected = auth?.mixcloud.connected ?? false;
  const selectedPlatformsConnected =
    (!mixcloudEnabled || mixcloudConnected) &&
    (!soundcloudEnabled || soundcloudConnected);
  const scheduleComplete =
    !scheduleEnabled || (scheduleDate !== "" && scheduleTime !== "");

  const handleSelectFile = async () => {
    const p = await pickAudioFile();
    if (p) setFilePath(p);
  };

  const handleSelectImage = async () => {
    const p = await pickImageFile();
    if (p) setImagePath(p);
  };

  const handleConnect = async (platform: Platform) => {
    if (connecting !== null) return;
    setConnecting(platform);
    try {
      await connectPlatform(platform);
      await refreshAuth();
      if (platform === "soundcloud") {
        setSoundcloudEnabled(true);
      } else {
        setMixcloudEnabled(true);
      }
      toast.success(`${platformLabel(platform)} connected`);
    } catch (e) {
      toast.error(`${platformLabel(platform)} connection failed`, {
        description: String(e),
      });
    } finally {
      setConnecting(null);
    }
  };

  const canUpload =
    filePath !== "" &&
    title.trim() !== "" &&
    (mixcloudEnabled || soundcloudEnabled) &&
    selectedPlatformsConnected &&
    scheduleComplete &&
    !isUploading;

  const handleUpload = async () => {
    if (!filePath || !title.trim()) {
      toast.error("File and title are required");
      return;
    }
    if (!mixcloudEnabled && !soundcloudEnabled) {
      toast.error("Select at least one platform");
      return;
    }
    const disconnected = [
      mixcloudEnabled && !mixcloudConnected ? "Mixcloud" : null,
      soundcloudEnabled && !soundcloudConnected ? "SoundCloud" : null,
    ].filter(Boolean);
    if (disconnected.length > 0) {
      toast.error(`Connect ${disconnected.join(" and ")} before uploading`);
      return;
    }
    if (!scheduleComplete) {
      toast.error("Choose both a publish date and time");
      return;
    }
    setIsUploading(true);
    setResults([]);
    setProgress(null);
    try {
      const outcomes = await upload({
        filePath,
        title,
        description,
        imagePath,
        tags,
        mixcloud: mixcloudEnabled,
        soundcloud: soundcloudEnabled,
        scheduleEnabled,
        scheduleDate,
        scheduleTime,
        generatePreviews,
      });
      setResults(outcomes);
      for (const o of outcomes) {
        if (o.success) {
          toast.success(`${o.platform} · uploaded`, {
            description: o.title,
            action: o.url
              ? { label: "Open", onClick: () => openUrl(o.url) }
              : undefined,
          });
        } else if (isCancelled(o)) {
          toast.message(`${o.platform} · upload cancelled`);
        } else {
          toast.error(`${o.platform} upload failed`, {
            description: o.error ?? "Unknown upload error",
          });
        }
      }

      const failed = outcomes.filter((outcome) => !outcome.success);
      if (failed.length === 0) {
        setFilePath("");
        setTitle("");
        setDescription("");
        setImagePath("");
        setTags("");
      } else {
        // Keep the form for retrying, but deselect platforms that already
        // succeeded so a retry cannot create duplicate uploads.
        if (outcomes.some((o) => o.platform === "Mixcloud" && o.success)) {
          setMixcloudEnabled(false);
        }
        if (outcomes.some((o) => o.platform === "SoundCloud" && o.success)) {
          setSoundcloudEnabled(false);
        }
      }
      await refreshAuth();
    } catch (e) {
      toast.error("Upload failed", { description: String(e) });
    } finally {
      setIsUploading(false);
      setProgress(null);
    }
  };

  const handleCancel = async () => {
    setProgress((prev) => (prev ? { ...prev, phase: "cancelling" } : prev));
    try {
      await cancelUpload();
    } catch (e) {
      toast.error("Couldn't cancel the upload", { description: String(e) });
    }
  };

  const handleInstallUpdate = async () => {
    if (!update) return;
    setInstalling(true);
    toast.loading("Downloading update…");
    try {
      await installUpdate(update);
    } catch (e) {
      toast.error("Update failed", { description: String(e) });
      setInstalling(false);
    }
  };

  // Compact card styling so the whole form fits the window without scrolling.
  const cardCls = "app-card glass-surface gap-2 py-3";
  const headCls = "px-4";
  const bodyCls = "px-4";
  const titleCls = "flex items-center gap-2 text-sm";

  return (
    <div className="flex h-dvh w-full flex-col overflow-hidden antialiased">
      <div
        data-tauri-drag-region
        aria-hidden="true"
        className="window-drag-region fixed inset-x-0 top-0 z-50 h-7 cursor-grab active:cursor-grabbing"
      />
      <Toaster
        position="top-center"
        richColors
        closeButton
        offset={{ top: 64 }}
      />
      <main
        ref={layoutRef}
        className="app-main mx-auto flex min-h-full w-full max-w-xl shrink-0 flex-col gap-2.5 px-5 pt-8 pb-4"
        style={{
          transform: `scale(${uiScale})`,
          transformOrigin: "top center",
          width: `${100 / uiScale}%`,
        }}
      >
        {/* Header (also the window drag region, since the title bar is hidden) */}
        <header
          className="app-header cursor-grab select-none text-center active:cursor-grabbing"
          onMouseDown={startWindowDrag}
        >
          <h1 className="text-gradient text-2xl font-bold tracking-tight">
            DJ Mix Uploader
          </h1>
          <p className="text-xs text-muted-foreground">
            Publish your mixes to Mixcloud &amp; SoundCloud
          </p>
        </header>

        {/* Update banner */}
        {update && (
          <Alert className="glass-surface flex items-center justify-between gap-3 py-2">
            <div>
              <AlertTitle>Update available — v{update.version}</AlertTitle>
              <AlertDescription>Ready to install.</AlertDescription>
            </div>
            <Button
              size="sm"
              variant="gradient"
              onClick={handleInstallUpdate}
              disabled={installing}
              className="shrink-0"
            >
              {installing ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Download className="size-4" />
              )}
              Update
            </Button>
          </Alert>
        )}

        {/* The fixed-size window uses compact responsive spacing at short heights. */}
        <div className="app-content flex shrink-0 flex-col gap-2.5">
        {/* Audio file */}
        <Card className={cardCls}>
          <CardHeader className={headCls}>
            <CardTitle className={titleCls}>
              <Music className="size-4" /> Audio File
            </CardTitle>
          </CardHeader>
          <CardContent className={cn(bodyCls, "flex flex-col gap-2")}>
            <button
              type="button"
              onClick={handleSelectFile}
              className={cn(
                "audio-drop flex h-16 w-full items-center justify-center rounded-lg border-2 border-dashed px-4 text-center text-xs transition-[border-color,color]",
                filePath
                  ? "border-emerald-400/60 text-emerald-300"
                  : "border-white/15 text-muted-foreground hover:border-white/30",
              )}
            >
              {filePath ? (
                <span className="flex items-center gap-2 truncate">
                  <CheckCircle2 className="size-4 shrink-0" />
                  {basename(filePath)}
                </span>
              ) : (
                <span>🎶 Click to select an audio file</span>
              )}
            </button>
          </CardContent>
        </Card>

        {/* Track information */}
        <Card className={cardCls}>
          <CardHeader className={headCls}>
            <CardTitle className="text-sm">Track Information</CardTitle>
          </CardHeader>
          <CardContent className={cn(bodyCls, "flex flex-col gap-2")}>
            <div className="flex flex-col gap-1">
              <Label htmlFor="title" className="text-xs">
                Title
              </Label>
              <Input
                id="title"
                placeholder="Enter track title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="description" className="text-xs">
                Description
              </Label>
              <Textarea
                id="description"
                placeholder="Optional description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="track-description min-h-10 resize-none"
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="tags" className="text-xs">
                Tags
              </Label>
              <Input
                id="tags"
                placeholder="house, techno, electronic"
                value={tags}
                onChange={(e) => setTags(e.target.value)}
              />
            </div>
          </CardContent>
        </Card>

        {/* Platforms + Schedule (two columns) */}
        <div className="grid grid-cols-2 gap-2.5">
          {/* Platforms */}
          <Card className={cn(cardCls, "h-full")}>
            <CardHeader className={headCls}>
              <CardTitle className="text-sm">Platforms</CardTitle>
            </CardHeader>
            <CardContent className={cn(bodyCls, "flex flex-col gap-2.5")}>
              {/* Mixcloud */}
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <Checkbox
                    id="mixcloud"
                    checked={mixcloudEnabled}
                    onCheckedChange={(v) => setMixcloudEnabled(v === true)}
                  />
                  <Label htmlFor="mixcloud" className="text-sm">
                    Mixcloud
                  </Label>
                </div>
                {mixcloudConnected ? (
                  <ConnectedBadge />
                ) : (
                  <Button
                    size="sm"
                    variant="glass"
                    onClick={() => handleConnect("mixcloud")}
                    disabled={connecting !== null}
                  >
                    {connecting === "mixcloud" && (
                      <Loader2 className="size-4 animate-spin" />
                    )}
                    Connect
                  </Button>
                )}
              </div>

              {/* SoundCloud */}
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <Checkbox
                    id="soundcloud"
                    checked={soundcloudEnabled}
                    onCheckedChange={(v) => setSoundcloudEnabled(v === true)}
                  />
                  <Label htmlFor="soundcloud" className="text-sm">
                    SoundCloud
                  </Label>
                </div>
                {soundcloudConnected ? (
                  <ConnectedBadge />
                ) : (
                  <Button
                    size="sm"
                    variant="glass"
                    onClick={() => handleConnect("soundcloud")}
                    disabled={connecting !== null}
                  >
                    {connecting === "soundcloud" ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : (
                      <SoundcloudLogo
                        weight="fill"
                        className="size-4 text-[#ff5500]"
                      />
                    )}
                    Connect
                  </Button>
                )}
              </div>

              {!mixcloudEnabled && !soundcloudEnabled ? (
                <p className="text-xs text-amber-400">⚠️ Pick a platform</p>
              ) : !selectedPlatformsConnected ? (
                <p className="text-xs text-amber-400">Connect selected platforms</p>
              ) : null}
            </CardContent>
          </Card>

          {/* Schedule */}
          <Card className={cn(cardCls, "h-full")}>
            <CardHeader className={headCls}>
              <CardTitle className={titleCls}>
                <CalendarClock className="size-4" /> Schedule
              </CardTitle>
            </CardHeader>
            <CardContent className={cn(bodyCls, "flex flex-col gap-2")}>
              <div className="flex items-center gap-2">
                <Switch
                  id="schedule"
                  checked={scheduleEnabled}
                  onCheckedChange={setScheduleEnabled}
                />
                <Label htmlFor="schedule" className="text-sm">
                  Publish later
                </Label>
              </div>
              {scheduleEnabled && (
                <div className="flex flex-col gap-1.5">
                  <div className="flex flex-col gap-1">
                    <Label className="text-[11px] text-muted-foreground">
                      Date
                    </Label>
                    <DatePickerInput
                      aria-label="Date"
                      placeholder="Pick a date"
                      value={
                        scheduleDate
                          ? new Date(`${scheduleDate}T00:00:00`)
                          : undefined
                      }
                      onChange={(d) =>
                        setScheduleDate(d ? format(d, "yyyy-MM-dd") : "")
                      }
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <Label htmlFor="schedule-time" className="text-[11px] text-muted-foreground">
                      Time
                    </Label>
                    <Input
                      id="schedule-time"
                      type="time"
                      aria-label="Time"
                      value={scheduleTime}
                      onChange={(e) => setScheduleTime(e.target.value)}
                    />
                  </div>
                  <p className="text-[11px] text-muted-foreground">
                    Mixcloud Pro · your local time → UTC
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Artwork + Preview (two columns) */}
        <div className="grid grid-cols-2 gap-2.5">
          {/* Artwork */}
          <Card className={cn(cardCls, "h-full")}>
            <CardHeader className={headCls}>
              <CardTitle className={titleCls}>
                <ImageIcon className="size-4" /> Artwork
              </CardTitle>
            </CardHeader>
            <CardContent className={cn(bodyCls, "flex flex-col gap-1.5")}>
              {imagePath && (
                <span className="flex items-center gap-1.5 truncate text-xs text-emerald-300">
                  <CheckCircle2 className="size-3.5 shrink-0" />
                  {basename(imagePath)}
                </span>
              )}
              <Button size="sm" variant="glass" onClick={handleSelectImage}>
                {imagePath ? "Change…" : "Select Artwork…"}
              </Button>
            </CardContent>
          </Card>

          {/* Preview snippets */}
          <Card className={cn(cardCls, "h-full")}>
            <CardHeader className={headCls}>
              <CardTitle className={titleCls}>
                <Scissors className="size-4" /> Previews
              </CardTitle>
            </CardHeader>
            <CardContent className={cn(bodyCls, "flex items-center gap-2")}>
              <Switch
                id="previews"
                checked={generatePreviews}
                onCheckedChange={setGeneratePreviews}
              />
              <Label htmlFor="previews" className="text-xs leading-tight">
                Generate 30/60/90s snippets
              </Label>
            </CardContent>
          </Card>
        </div>

        </div>

        {/* Upload */}
        <GlassButton
          variant="gradient"
          effect="glow"
          className="h-11 text-base"
          onClick={handleUpload}
          disabled={!canUpload}
        >
          {isUploading ? (
            <Loader2 className="size-5 animate-spin" />
          ) : (
            <UploadCloud className="size-5" />
          )}
          {isUploading ? "Uploading…" : "Upload"}
        </GlassButton>

        {/* Upload progress */}
        {isUploading && progress && (
          <div className="space-y-1.5">
            <div className="flex items-center justify-between gap-2 text-xs">
              {progress.phase === "retrying" && progress.retry ? (
                <span className="flex items-center gap-1.5 text-amber-300">
                  <Wifi className="size-3.5 animate-pulse" />
                  Connection unstable — reconnecting… (attempt{" "}
                  {progress.retry.attempt} of {progress.retry.maxAttempts})
                </span>
              ) : progress.phase === "cancelling" ? (
                <span className="text-muted-foreground">Cancelling…</span>
              ) : (
                <span className="text-muted-foreground">
                  Uploading to{" "}
                  {progress.platform === "mixcloud" ? "Mixcloud" : "SoundCloud"}…
                </span>
              )}
              <div className="flex shrink-0 items-center gap-2">
                {progress.phase !== "retrying" && (
                  <span className="tabular-nums text-muted-foreground">
                    {progress.pct}%
                  </span>
                )}
                <button
                  type="button"
                  onClick={handleCancel}
                  disabled={progress.phase === "cancelling"}
                  className="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-muted-foreground transition-colors hover:text-red-300 disabled:opacity-50"
                  title="Cancel upload"
                >
                  <X className="size-3.5" />
                  Cancel
                </button>
              </div>
            </div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-white/10">
              <div
                className={cn(
                  "h-full rounded-full transition-[width] duration-150",
                  progress.phase === "retrying" && "animate-pulse",
                )}
                style={{
                  width:
                    progress.phase === "retrying" ? "100%" : `${progress.pct}%`,
                  backgroundImage:
                    progress.phase === "retrying"
                      ? "none"
                      : "var(--gradient-text)",
                  backgroundColor:
                    progress.phase === "retrying"
                      ? "rgb(251 191 36 / 0.35)"
                      : undefined,
                }}
              />
            </div>
          </div>
        )}

        {/* Uploaded-track links */}
        {!isUploading && results.length > 0 && (
          <div className="glass-surface rounded-xl px-4 py-3 text-sm">
            <p className="mb-1.5 flex items-center gap-1.5 text-xs text-muted-foreground">
              <CheckCircle2 className="size-3.5" /> Upload results
            </p>
            <ul className="space-y-1">
              {results.map((r) => {
                const cancelled = isCancelled(r);
                return (
                  <li
                    key={r.platform}
                    className="flex items-center justify-between gap-2"
                  >
                    <span
                      className={cn(
                        "truncate",
                        r.success
                          ? "text-emerald-300"
                          : cancelled
                            ? "text-muted-foreground"
                            : "text-red-300",
                      )}
                    >
                      {r.platform}
                    </span>
                    {r.success && r.url ? (
                      <button
                        type="button"
                        onClick={() => openUrl(r.url)}
                        className="shrink-0 text-[var(--glass-accent)] underline-offset-2 hover:underline"
                      >
                        Open ↗
                      </button>
                    ) : r.success ? (
                      <span className="shrink-0 text-muted-foreground">done</span>
                    ) : cancelled ? (
                      <span className="shrink-0 text-xs text-muted-foreground">
                        cancelled
                      </span>
                    ) : (
                      <span className="max-w-56 truncate text-xs text-red-300" title={r.error ?? undefined}>
                        {r.error ?? "failed"}
                      </span>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>
        )}

        {/* Decorative footer yields its space to transient upload/update UI. */}
        {!update && !isUploading && results.length === 0 ? (
          <footer className="app-footer text-center text-[11px] text-muted-foreground">
            Made with ❤️ in San Francisco
          </footer>
        ) : null}
      </main>
    </div>
  );
}
