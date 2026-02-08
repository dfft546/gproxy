import { useEffect, useMemo, useRef, useState } from "react";

import { safeParseJson } from "../lib/api";
import { Button, Card } from "../components/ui";
import { useI18n } from "../i18n";

type Props = {
  adminKey: string;
  notify: (kind: "success" | "error" | "info", message: string) => void;
};

type ConnectState = "disconnected" | "connecting" | "connected" | "error";

type LogRow = {
  id: number;
  kind: string;
  text: string;
};

const LOG_LIMIT = 500;

function parseLogPayload(raw: string): { kind: string; text: string } {
  const parsed = safeParseJson(raw);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    return { kind: "raw", text: raw };
  }
  const obj = parsed as Record<string, unknown>;
  const keys = Object.keys(obj);
  const kind = keys.length === 1 ? keys[0] : "event";
  return {
    kind,
    text: JSON.stringify(obj, null, 2)
  };
}

export function EventStreamSection({ adminKey, notify }: Props) {
  const { t } = useI18n();
  const [state, setState] = useState<ConnectState>("disconnected");
  const [errorText, setErrorText] = useState("");
  const [logs, setLogs] = useState<LogRow[]>([]);
  const wsRef = useRef<WebSocket | null>(null);
  const logIdRef = useRef(0);
  const manualCloseRef = useRef(false);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const statusText = useMemo(() => {
    if (state === "connected") {
      return t("events.status_connected");
    }
    if (state === "connecting") {
      return t("events.status_connecting");
    }
    if (state === "error") {
      return t("events.status_error");
    }
    return t("events.status_disconnected");
  }, [state, t]);

  const pushLog = (raw: string) => {
    const parsed = parseLogPayload(raw);
    setLogs((prev) => {
      const next = [
        ...prev,
        {
          id: ++logIdRef.current,
          kind: parsed.kind,
          text: parsed.text
        }
      ];
      return next.length > LOG_LIMIT ? next.slice(next.length - LOG_LIMIT) : next;
    });
    requestAnimationFrame(() => {
      if (scrollRef.current) {
        scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
      }
    });
  };

  const disconnect = () => {
    manualCloseRef.current = true;
    if (wsRef.current) {
      wsRef.current.close(1000, "manual_disconnect");
      wsRef.current = null;
    }
    setState("disconnected");
  };

  const connect = () => {
    if (!adminKey.trim()) {
      notify("error", t("auth.required"));
      return;
    }
    const current = wsRef.current;
    if (
      current &&
      (current.readyState === WebSocket.OPEN || current.readyState === WebSocket.CONNECTING)
    ) {
      return;
    }

    manualCloseRef.current = false;
    setState("connecting");
    setErrorText("");

    const scheme = window.location.protocol === "https:" ? "wss" : "ws";
    const url = `${scheme}://${window.location.host}/admin/events/ws?admin_key=${encodeURIComponent(adminKey)}`;
    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setState("connected");
      notify("success", t("events.connected_ok"));
    };

    ws.onmessage = (event) => {
      if (typeof event.data === "string") {
        pushLog(event.data);
        return;
      }
      if (event.data instanceof Blob) {
        void event.data.text().then((text) => pushLog(text));
      }
    };

    ws.onerror = () => {
      setErrorText(t("events.connect_failed"));
    };

    ws.onclose = () => {
      wsRef.current = null;
      if (manualCloseRef.current) {
        setState("disconnected");
        return;
      }
      setState("error");
      setErrorText(t("events.disconnected_unexpected"));
    };
  };

  useEffect(() => {
    return () => {
      disconnect();
    };
  }, []);

  return (
    <Card
      title={t("events.title")}
      subtitle={t("events.subtitle")}
      action={
        <div className="flex flex-wrap gap-2">
          {state === "connected" || state === "connecting" ? (
            <Button variant="danger" onClick={disconnect}>
              {t("events.disconnect")}
            </Button>
          ) : (
            <Button onClick={connect}>{t("events.connect")}</Button>
          )}
          <Button variant="neutral" onClick={() => setLogs([])}>
            {t("events.clear")}
          </Button>
        </div>
      }
    >
      <div className="mb-3 flex flex-wrap items-center gap-2 text-sm">
        <span className="font-semibold text-slate-700">{t("events.status_label")}:</span>
        <span
          className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-semibold ${
            state === "connected"
              ? "border border-emerald-200 bg-emerald-50 text-emerald-700"
              : state === "connecting"
                ? "border border-amber-200 bg-amber-50 text-amber-700"
                : state === "error"
                  ? "border border-rose-200 bg-rose-50 text-rose-700"
                  : "border border-slate-200 bg-slate-100 text-slate-600"
          }`}
        >
          {statusText}
        </span>
        {errorText ? <span className="text-xs text-rose-600">{errorText}</span> : null}
      </div>
      <div
        ref={scrollRef}
        className="h-[520px] overflow-auto rounded-xl border border-slate-200 bg-slate-950 p-3"
      >
        {logs.length === 0 ? (
          <div className="text-xs text-slate-400">{t("events.empty")}</div>
        ) : (
          <div className="space-y-2">
            {logs.map((row) => (
              <div key={row.id} className="rounded-lg border border-slate-800 bg-slate-900/70 p-2">
                <div className="mb-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-sky-300">
                  {row.kind}
                </div>
                <pre className="overflow-x-auto whitespace-pre-wrap break-all text-xs text-emerald-100">
                  {row.text}
                </pre>
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}
