import { useEffect, useMemo, useState } from "react";

import { request, formatApiError } from "../lib/api";
import type { OAuthCallbackResponse, OAuthStartResponse, ProviderDetail } from "../lib/types";
import { Button, Card, FieldLabel, TextInput } from "../components/ui";
import { useI18n } from "../i18n";

const SUPPORTED = ["codex", "claudecode", "geminicli", "antigravity"];

type Props = {
  adminKey: string;
  providers: ProviderDetail[];
  notify: (kind: "success" | "error" | "info", message: string) => void;
  onOAuthDone: () => void;
};

function scalarEntries(source: Record<string, unknown>): Array<[string, string]> {
  return Object.entries(source)
    .filter(([, value]) => ["string", "number", "boolean"].includes(typeof value))
    .map(([key, value]) => [key, String(value)]);
}

export function OAuthSection({ adminKey, providers, notify, onOAuthDone }: Props) {
  const { t } = useI18n();

  const candidates = useMemo(() => {
    const names = providers.map((item) => item.name);
    return SUPPORTED.filter((name) => names.includes(name));
  }, [providers]);

  const [provider, setProvider] = useState(candidates[0] ?? "codex");
  const [startParams, setStartParams] = useState({ redirect_uri: "", scope: "", project_id: "" });
  const [callbackParams, setCallbackParams] = useState({ state: "", code: "", callback_url: "", project_id: "" });
  const [startResponse, setStartResponse] = useState<OAuthStartResponse | null>(null);
  const [callbackResponse, setCallbackResponse] = useState<OAuthCallbackResponse | null>(null);

  useEffect(() => {
    if (candidates.length === 0) {
      return;
    }
    if (!candidates.includes(provider)) {
      setProvider(candidates[0]);
    }
  }, [candidates, provider]);

  const runStart = async () => {
    try {
      const data = await request<OAuthStartResponse>(`/${provider}/oauth`, {
        userKey: adminKey,
        query: {
          redirect_uri: startParams.redirect_uri.trim() || undefined,
          scope: startParams.scope.trim() || undefined,
          project_id: startParams.project_id.trim() || undefined
        }
      });
      setStartResponse(data);
      if (typeof data.state === "string") {
        setCallbackParams((prev) => ({ ...prev, state: data.state as string }));
      }
      notify("success", t("oauth.start_ok"));
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const runCallback = async () => {
    try {
      if (!callbackParams.code.trim() && !callbackParams.callback_url.trim() && provider !== "codex") {
        throw new Error(t("errors.missing_code_or_callback"));
      }
      if (!callbackParams.state.trim() && provider === "codex") {
        throw new Error(t("errors.missing_state"));
      }

      const data = await request<OAuthCallbackResponse>(`/${provider}/oauth/callback`, {
        userKey: adminKey,
        query: {
          state: callbackParams.state.trim() || undefined,
          code: callbackParams.code.trim() || undefined,
          callback_url: callbackParams.callback_url.trim() || undefined,
          project_id: callbackParams.project_id.trim() || undefined
        }
      });
      setCallbackResponse(data);
      notify("success", t("oauth.callback_ok"));
      onOAuthDone();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const startInfo = startResponse ? scalarEntries(startResponse) : [];
  const callbackInfo = callbackResponse ? scalarEntries(callbackResponse) : [];

  return (
    <div className="space-y-5">
      <Card title={t("oauth.title")} subtitle={t("oauth.subtitle")}>
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <FieldLabel>{t("common.provider")}</FieldLabel>
            <select className="mt-2 select" value={provider} onChange={(event) => setProvider(event.target.value)}>
              {candidates.map((item) => (
                <option key={item} value={item}>
                  {item}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="mt-5 grid gap-4 md:grid-cols-3">
          <div>
            <FieldLabel>{t("oauth.redirect_uri")}</FieldLabel>
            <div className="mt-2">
              <TextInput
                value={startParams.redirect_uri}
                onChange={(value) => setStartParams((prev) => ({ ...prev, redirect_uri: value }))}
              />
            </div>
          </div>
          <div>
            <FieldLabel>{t("oauth.scope")}</FieldLabel>
            <div className="mt-2">
              <TextInput
                value={startParams.scope}
                onChange={(value) => setStartParams((prev) => ({ ...prev, scope: value }))}
              />
            </div>
          </div>
          <div>
            <FieldLabel>{t("oauth.project_id")}</FieldLabel>
            <div className="mt-2">
              <TextInput
                value={startParams.project_id}
                onChange={(value) => setStartParams((prev) => ({ ...prev, project_id: value }))}
              />
            </div>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap gap-2">
          <Button onClick={() => void runStart()}>{t("oauth.start")}</Button>
          {startResponse?.auth_url ? (
            <Button variant="neutral" onClick={() => window.open(String(startResponse.auth_url), "_blank", "noopener,noreferrer")}>
              {t("oauth.open_auth")}
            </Button>
          ) : null}
        </div>

        {startInfo.length > 0 ? (
          <div className="mt-5 rounded-xl border border-slate-200 bg-slate-50 p-4">
            <div className="grid gap-2 md:grid-cols-2">
              {startInfo.map(([key, value]) => (
                <div key={key} className="text-sm text-slate-700">
                  <span className="font-semibold">{key}:</span> {value}
                </div>
              ))}
            </div>
          </div>
        ) : null}
      </Card>

      <Card title={t("oauth.callback")} subtitle={`/${provider}/oauth/callback`}>
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <FieldLabel>{t("oauth.state")}</FieldLabel>
            <div className="mt-2">
              <TextInput
                value={callbackParams.state}
                onChange={(value) => setCallbackParams((prev) => ({ ...prev, state: value }))}
              />
            </div>
          </div>
          <div>
            <FieldLabel>{t("oauth.code")}</FieldLabel>
            <div className="mt-2">
              <TextInput
                value={callbackParams.code}
                onChange={(value) => setCallbackParams((prev) => ({ ...prev, code: value }))}
              />
            </div>
          </div>
          <div className="md:col-span-2">
            <FieldLabel>{t("oauth.callback_url")}</FieldLabel>
            <div className="mt-2">
              <TextInput
                value={callbackParams.callback_url}
                onChange={(value) => setCallbackParams((prev) => ({ ...prev, callback_url: value }))}
              />
            </div>
          </div>
          <div>
            <FieldLabel>{t("oauth.project_id")}</FieldLabel>
            <div className="mt-2">
              <TextInput
                value={callbackParams.project_id}
                onChange={(value) => setCallbackParams((prev) => ({ ...prev, project_id: value }))}
              />
            </div>
          </div>
        </div>

        <div className="mt-4">
          <Button onClick={() => void runCallback()}>{t("oauth.callback")}</Button>
        </div>

        {callbackInfo.length > 0 ? (
          <div className="mt-5 rounded-xl border border-slate-200 bg-slate-50 p-4">
            <div className="grid gap-2 md:grid-cols-2">
              {callbackInfo.map(([key, value]) => (
                <div key={key} className="text-sm text-slate-700">
                  <span className="font-semibold">{key}:</span> {value}
                </div>
              ))}
            </div>
          </div>
        ) : null}
      </Card>
    </div>
  );
}
