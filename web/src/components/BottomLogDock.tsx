import { useCallback, useEffect, useMemo, useState } from "react";
import { AlertTriangle, ChevronDown, ChevronUp, FileText, RefreshCw } from "lucide-react";

import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Button } from "@nous-research/ui/ui/components/button";
import { Badge } from "@nous-research/ui/ui/components/badge";

const LOG_FILES = [
  { value: "agent", label: "agent" },
  { value: "errors", label: "errors" },
  { value: "dashboard-out", label: "dashboard out" },
  { value: "dashboard-err", label: "dashboard err" },
] as const;

function classifyLine(line: string): "error" | "warning" | "muted" | "normal" {
  const upper = line.toUpperCase();
  if (upper.includes("ERROR") || upper.includes("CRITICAL") || upper.includes("FATAL")) {
    return "error";
  }
  if (upper.includes("WARNING") || upper.includes("WARN")) return "warning";
  if (!line.trim()) return "muted";
  return "normal";
}

const LINE_CLASS: Record<ReturnType<typeof classifyLine>, string> = {
  error: "text-red-300",
  warning: "text-amber-200",
  muted: "text-midground/35",
  normal: "text-midground/80",
};

export function BottomLogDock() {
  const [expanded, setExpanded] = useState(false);
  const [file, setFile] = useState<(typeof LOG_FILES)[number]["value"]>("agent");
  const [lines, setLines] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchLogs = useCallback(() => {
    setLoading(true);
    api
      .getLogs({ file, lines: expanded ? 80 : 8 })
      .then((resp) => {
        setLines(resp.lines);
        setError(null);
      })
      .catch((err) => setError(err instanceof Error ? err.message : String(err)))
      .finally(() => setLoading(false));
  }, [expanded, file]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  useEffect(() => {
    const interval = window.setInterval(fetchLogs, expanded ? 2500 : 5000);
    return () => window.clearInterval(interval);
  }, [expanded, fetchLogs]);

  const latest = useMemo(() => lines.at(-1)?.trim() || "No recent log output", [lines]);
  const hasErrors = lines.some((line) => classifyLine(line) === "error");

  return (
    <div className="fixed inset-x-0 bottom-0 z-50 pointer-events-none">
      <div className="mx-auto max-w-[1600px] px-3 pb-2 lg:pl-[17rem] lg:pr-4">
        <div
          className={cn(
            "pointer-events-auto overflow-hidden border border-white/10",
            "bg-[#0b0d10]/95 text-midground shadow-[0_-18px_60px_rgba(0,0,0,0.35)]",
            "backdrop-blur-md",
          )}
        >
          <div className="flex min-h-10 items-center gap-2 border-b border-white/10 px-2.5 py-1.5">
            <Button
              ghost
              size="icon"
              aria-label={expanded ? "Collapse live logs" : "Expand live logs"}
              onClick={() => setExpanded((v) => !v)}
              className="h-7 w-7 shrink-0 text-midground/75 hover:text-midground"
            >
              {expanded ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
            </Button>

            <div className="flex min-w-0 flex-1 items-center gap-2">
              <FileText className="h-3.5 w-3.5 shrink-0 text-amber-300" />
              <span className="shrink-0 text-[0.68rem] font-semibold uppercase tracking-[0.12em] text-midground/70">
                Live logs
              </span>
              <Badge tone={hasErrors ? "destructive" : "secondary"} className="hidden text-[10px] sm:inline-flex">
                {file}
              </Badge>
              {hasErrors && <AlertTriangle className="h-3.5 w-3.5 shrink-0 text-red-300" />}
              <span
                className={cn(
                  "min-w-0 truncate font-mono-ui text-[11px]",
                  error ? "text-red-300" : "text-midground/65",
                )}
              >
                {error || latest}
              </span>
            </div>

            <select
              value={file}
              onChange={(e) => setFile(e.target.value as typeof file)}
              className={cn(
                "h-7 max-w-32 shrink-0 rounded border border-white/10 bg-white/[0.04] px-2",
                "text-[11px] text-midground/80 outline-none hover:bg-white/[0.07]",
              )}
              aria-label="Log file"
            >
              {LOG_FILES.map((item) => (
                <option key={item.value} value={item.value}>
                  {item.label}
                </option>
              ))}
            </select>

            <Button
              ghost
              size="icon"
              aria-label="Refresh logs"
              onClick={fetchLogs}
              className="h-7 w-7 shrink-0 text-midground/65 hover:text-midground"
            >
              <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
            </Button>
          </div>

          {expanded && (
            <div className="max-h-56 overflow-auto px-3 py-2 font-mono-ui text-[11px] leading-5">
              {error ? (
                <div className="text-red-300">{error}</div>
              ) : lines.length ? (
                lines.map((line, index) => (
                  <div key={`${index}-${line}`} className={cn("whitespace-pre-wrap break-words", LINE_CLASS[classifyLine(line)])}>
                    {line}
                  </div>
                ))
              ) : (
                <div className="text-midground/45">No recent log output</div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
