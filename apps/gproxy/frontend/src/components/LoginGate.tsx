import { useActionState } from "react";

import { Button, Card, FieldLabel } from "./ui";
import { useI18n } from "../i18n";

type AuthFormState = {
  message?: string;
};

export function LoginGate({
  initialKey,
  onLogin
}: {
  initialKey: string;
  onLogin: (key: string) => Promise<{ ok: boolean; message?: string }>;
}) {
  const { t } = useI18n();

  const [state, submitAction, pending] = useActionState<AuthFormState, FormData>(
    async (_prev, formData) => {
      const key = String(formData.get("adminKey") ?? "").trim();
      if (!key) {
        return { message: t("auth.required") };
      }
      const result = await onLogin(key);
      if (!result.ok) {
        return { message: result.message ?? t("auth.failed") };
      }
      return { message: undefined };
    },
    { message: undefined }
  );

  return (
    <div className="mx-auto flex min-h-screen w-full max-w-lg items-center justify-center px-6 py-12">
      <Card title={t("auth.title")} subtitle={t("auth.description")}>
        <form action={submitAction} className="space-y-4">
          <div>
            <FieldLabel>{t("auth.admin_key")}</FieldLabel>
            <input
              type="password"
              name="adminKey"
              defaultValue={initialKey}
              placeholder={t("auth.placeholder")}
              autoComplete="current-password"
              className="mt-2 input"
            />
          </div>
          <Button type="submit" variant="primary" disabled={pending}>
            {pending ? t("auth.validating") : t("auth.submit")}
          </Button>
          {state.message ? <p className="text-sm text-rose-600">{state.message}</p> : null}
        </form>
      </Card>
    </div>
  );
}
