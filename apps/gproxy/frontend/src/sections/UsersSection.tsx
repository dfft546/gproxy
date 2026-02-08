import { useCallback, useEffect, useMemo, useState } from "react";

import { request, formatApiError } from "../lib/api";
import type { UserKeyRow, UserRow } from "../lib/types";
import { Badge, Button, Card, FieldLabel, TextInput } from "../components/ui";
import { useI18n } from "../i18n";

type Props = {
  adminKey: string;
  notify: (kind: "success" | "error" | "info", message: string) => void;
};

export function UsersSection({ adminKey, notify }: Props) {
  const { t } = useI18n();
  const [users, setUsers] = useState<UserRow[]>([]);
  const [activeUserId, setActiveUserId] = useState<number | null>(null);
  const [keys, setKeys] = useState<UserKeyRow[]>([]);
  const [userDraft, setUserDraft] = useState({ id: "", name: "", enabled: true });
  const [keyDraft, setKeyDraft] = useState({ label: "", key: "", enabled: true });
  const [plaintextKey, setPlaintextKey] = useState("");

  const loadUsers = useCallback(async () => {
    try {
      const data = await request<{ users: UserRow[] }>("/admin/users", { adminKey });
      const rows = data.users ?? [];
      setUsers(rows);
      if (rows.length > 0 && activeUserId === null) {
        setActiveUserId(rows[0].id);
      }
    } catch (error) {
      notify("error", formatApiError(error));
    }
  }, [activeUserId, adminKey, notify]);

  const loadKeys = useCallback(async () => {
    if (activeUserId === null) {
      return;
    }
    try {
      const data = await request<{ keys: UserKeyRow[] }>(`/admin/users/${activeUserId}/keys`, {
        adminKey
      });
      setKeys(data.keys ?? []);
    } catch (error) {
      notify("error", formatApiError(error));
    }
  }, [activeUserId, adminKey, notify]);

  useEffect(() => {
    void loadUsers();
  }, [loadUsers]);

  useEffect(() => {
    void loadKeys();
  }, [loadKeys]);

  const nextUserId = useMemo(() => {
    if (users.length === 0) {
      return 1;
    }
    return Math.max(...users.map((item) => item.id)) + 1;
  }, [users]);

  const saveUser = async () => {
    try {
      const id = Number(userDraft.id || nextUserId);
      const name = userDraft.name.trim() || `user-${id}`;
      await request(`/admin/users/${id}`, {
        method: "PUT",
        adminKey,
        body: {
          name,
          enabled: userDraft.enabled
        }
      });
      notify("success", t("users.create_user_ok"));
      setUserDraft({ id: "", name: "", enabled: true });
      await loadUsers();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const deleteUser = async (userId: number) => {
    if (!confirm(`${t("common.delete")}? user #${userId}`)) {
      return;
    }
    try {
      await request(`/admin/users/${userId}`, { method: "DELETE", adminKey });
      notify("success", t("users.delete_user_ok"));
      if (activeUserId === userId) {
        setActiveUserId(null);
      }
      await loadUsers();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const toggleUser = async (user: UserRow) => {
    try {
      await request(`/admin/users/${user.id}/enabled`, {
        method: "PUT",
        adminKey,
        body: { enabled: !user.enabled }
      });
      await loadUsers();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const createKey = async () => {
    if (activeUserId === null) {
      return;
    }
    try {
      const data = await request<{ id: number; key: string }>(`/admin/users/${activeUserId}/keys`, {
        method: "POST",
        adminKey,
        body: {
          key: keyDraft.key.trim() || undefined,
          label: keyDraft.label.trim() || undefined,
          enabled: keyDraft.enabled
        }
      });
      setPlaintextKey(data.key);
      setKeyDraft({ label: "", key: "", enabled: true });
      notify("success", t("users.create_key_ok"));
      await loadKeys();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const updateKeyLabel = async (keyId: number, label: string) => {
    try {
      await request(`/admin/user_keys/${keyId}`, {
        method: "PUT",
        adminKey,
        body: { label: label.trim() || null }
      });
      notify("success", t("users.update_key_ok"));
      await loadKeys();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const toggleKey = async (key: UserKeyRow) => {
    try {
      await request(`/admin/user_keys/${key.id}/enabled`, {
        method: "PUT",
        adminKey,
        body: { enabled: !key.enabled }
      });
      await loadKeys();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  const deleteKey = async (keyId: number) => {
    if (!confirm(`${t("common.delete")}? key #${keyId}`)) {
      return;
    }
    try {
      await request(`/admin/user_keys/${keyId}`, { method: "DELETE", adminKey });
      notify("success", t("users.delete_key_ok"));
      await loadKeys();
    } catch (error) {
      notify("error", formatApiError(error));
    }
  };

  return (
    <div className="space-y-5">
      <Card title={t("users.new_user")} subtitle="PUT /admin/users/{id}">
        <div className="grid gap-4 md:grid-cols-3">
          <div>
            <FieldLabel>{t("users.user_id")}</FieldLabel>
            <div className="mt-2">
              <TextInput value={userDraft.id} type="number" onChange={(value) => setUserDraft((prev) => ({ ...prev, id: value }))} placeholder={String(nextUserId)} />
            </div>
          </div>
          <div>
            <FieldLabel>{t("users.user_name")}</FieldLabel>
            <div className="mt-2">
              <TextInput value={userDraft.name} onChange={(value) => setUserDraft((prev) => ({ ...prev, name: value }))} />
            </div>
          </div>
          <div className="flex items-end gap-2">
            <input
              id="user-enabled"
              type="checkbox"
              checked={userDraft.enabled}
              onChange={(event) => setUserDraft((prev) => ({ ...prev, enabled: event.target.checked }))}
            />
            <label htmlFor="user-enabled" className="text-sm text-slate-700">
              {t("common.enabled")}
            </label>
          </div>
        </div>
        <div className="mt-4">
          <Button onClick={() => void saveUser()}>{t("common.save")}</Button>
        </div>
      </Card>

      <Card title="Users" subtitle="/admin/users">
        {users.length === 0 ? (
          <div className="text-sm text-slate-500">{t("common.empty")}</div>
        ) : (
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {users.map((user) => (
              <div key={user.id} className={`provider-card ${activeUserId === user.id ? "provider-card-active" : ""}`}>
                <button type="button" className="w-full text-left" onClick={() => setActiveUserId(user.id)}>
                  <div className="flex items-center justify-between gap-2">
                    <div>
                      <div className="text-sm font-semibold text-slate-900">#{user.id} {user.name}</div>
                    </div>
                    <Badge active={user.enabled}>{user.enabled ? t("common.enabled") : t("common.disabled")}</Badge>
                  </div>
                </button>
                <div className="mt-3 flex gap-2">
                  <Button variant="neutral" onClick={() => void toggleUser(user)}>
                    {user.enabled ? t("common.disabled") : t("common.enabled")}
                  </Button>
                  <Button variant="danger" onClick={() => void deleteUser(user.id)}>{t("common.delete")}</Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card title={t("users.new_key")} subtitle={activeUserId !== null ? `/admin/users/${activeUserId}/keys` : ""}>
        {activeUserId === null ? (
          <div className="text-sm text-slate-500">{t("common.empty")}</div>
        ) : (
          <>
            <div className="grid gap-4 md:grid-cols-3">
              <div>
                <FieldLabel>{t("users.label")}</FieldLabel>
                <div className="mt-2">
                  <TextInput value={keyDraft.label} onChange={(value) => setKeyDraft((prev) => ({ ...prev, label: value }))} />
                </div>
              </div>
              <div>
                <FieldLabel>Key (optional)</FieldLabel>
                <div className="mt-2">
                  <TextInput value={keyDraft.key} onChange={(value) => setKeyDraft((prev) => ({ ...prev, key: value }))} type="password" />
                </div>
              </div>
              <div className="flex items-end gap-2">
                <input
                  id="key-enabled"
                  type="checkbox"
                  checked={keyDraft.enabled}
                  onChange={(event) => setKeyDraft((prev) => ({ ...prev, enabled: event.target.checked }))}
                />
                <label htmlFor="key-enabled" className="text-sm text-slate-700">{t("common.enabled")}</label>
              </div>
            </div>
            <div className="mt-4 flex flex-wrap gap-2">
              <Button onClick={() => void createKey()}>{t("common.create")}</Button>
              {plaintextKey ? (
                <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
                  {t("users.plaintext_key")}: <span className="font-semibold">{plaintextKey}</span>
                </div>
              ) : null}
            </div>

            <div className="mt-5 space-y-3">
              {keys.map((key) => (
                <KeyRow
                  key={key.id}
                  row={key}
                  onToggle={() => void toggleKey(key)}
                  onDelete={() => void deleteKey(key.id)}
                  onSaveLabel={(label) => void updateKeyLabel(key.id, label)}
                />
              ))}
            </div>
          </>
        )}
      </Card>
    </div>
  );
}

function KeyRow({
  row,
  onToggle,
  onDelete,
  onSaveLabel
}: {
  row: UserKeyRow;
  onToggle: () => void;
  onDelete: () => void;
  onSaveLabel: (label: string) => void;
}) {
  const [label, setLabel] = useState(row.label ?? "");
  const { t } = useI18n();

  return (
    <div className="rounded-2xl border border-slate-200 bg-white/70 p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <div className="text-sm font-semibold text-slate-900">#{row.id}</div>
        </div>
        <Badge active={row.enabled}>{row.enabled ? t("common.enabled") : t("common.disabled")}</Badge>
      </div>
      <div className="mt-3 grid gap-3 md:grid-cols-[1fr_auto_auto_auto]">
        <TextInput value={label} onChange={setLabel} />
        <Button variant="neutral" onClick={() => onSaveLabel(label)}>{t("common.save")}</Button>
        <Button variant="neutral" onClick={onToggle}>{row.enabled ? t("common.disabled") : t("common.enabled")}</Button>
        <Button variant="danger" onClick={onDelete}>{t("common.delete")}</Button>
      </div>
    </div>
  );
}
