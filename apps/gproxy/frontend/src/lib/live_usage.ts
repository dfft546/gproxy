export type LiveUsageRow = {
  name: string;
  percent: number | null;
  resetAt: string | number | null;
};

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  return value as Record<string, unknown>;
}

function asNumber(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function toUsagePercent(value: unknown, mode: "used_percent" | "remaining_fraction"): number | null {
  const raw = asNumber(value);
  if (raw === null) {
    return null;
  }
  if (mode === "remaining_fraction") {
    return raw <= 1 ? (1 - raw) * 100 : 100 - raw;
  }
  return raw;
}

function toResetAt(value: unknown): string | number | null {
  if (typeof value === "string" && value.trim()) {
    return value;
  }
  const num = asNumber(value);
  if (num === null) {
    return null;
  }
  // Upstream may return epoch seconds; normalize to ms for Date formatting.
  return num < 1_000_000_000_000 ? num * 1000 : num;
}

function pushRow(
  rows: LiveUsageRow[],
  name: string,
  percent: number | null,
  resetAt: string | number | null
) {
  if (percent === null && resetAt === null) {
    return;
  }
  rows.push({ name, percent, resetAt });
}

function parseCodexUsage(data: Record<string, unknown>): LiveUsageRow[] {
  const rows: LiveUsageRow[] = [];
  const windows: Array<{ prefix: string; source: unknown }> = [
    { prefix: "rate_limit", source: data.rate_limit },
    { prefix: "code_review_rate_limit", source: data.code_review_rate_limit }
  ];

  for (const { prefix, source } of windows) {
    const root = asRecord(source);
    if (!root) {
      continue;
    }
    for (const key of ["primary_window", "secondary_window"]) {
      const window = asRecord(root[key]);
      if (!window) {
        continue;
      }
      pushRow(
        rows,
        `${prefix}.${key}`,
        toUsagePercent(window.used_percent, "used_percent"),
        toResetAt(window.reset_at ?? window.resetAt)
      );
    }
  }

  return rows;
}

function parseAntigravityUsage(data: Record<string, unknown>): LiveUsageRow[] {
  const rows: LiveUsageRow[] = [];
  const models = asRecord(data.models);
  if (!models) {
    return rows;
  }

  for (const [modelId, raw] of Object.entries(models)) {
    const model = asRecord(raw);
    if (!model) {
      continue;
    }
    const quota = asRecord(model.quotaInfo);
    pushRow(
      rows,
      modelId,
      toUsagePercent(quota?.remainingFraction, "remaining_fraction"),
      toResetAt(quota?.resetTime)
    );
  }

  return rows;
}

function parseGeminiCliUsage(data: Record<string, unknown>): LiveUsageRow[] {
  const rows: LiveUsageRow[] = [];
  const buckets = Array.isArray(data.buckets) ? data.buckets : [];

  for (const item of buckets) {
    const bucket = asRecord(item);
    if (!bucket) {
      continue;
    }
    const model = typeof bucket.modelId === "string" ? bucket.modelId : "unknown";
    const tokenType =
      typeof bucket.tokenType === "string" ? bucket.tokenType : "";
    const name = tokenType ? `${model} (${tokenType})` : model;
    pushRow(
      rows,
      name,
      toUsagePercent(bucket.remainingFraction, "remaining_fraction"),
      toResetAt(bucket.resetTime)
    );
  }

  return rows;
}

function parseClaudeCodeUsage(data: Record<string, unknown>): LiveUsageRow[] {
  const rows: LiveUsageRow[] = [];
  for (const [name, raw] of Object.entries(data)) {
    const section = asRecord(raw);
    if (!section) {
      continue;
    }
    pushRow(
      rows,
      name,
      toUsagePercent(section.utilization, "used_percent"),
      toResetAt(section.resets_at ?? section.resetAt)
    );
  }
  return rows;
}

export function parseLiveUsageRows(
  providerName: string,
  payload: Record<string, unknown>
): LiveUsageRow[] {
  const provider = providerName.toLowerCase();
  if (provider === "codex") {
    return parseCodexUsage(payload);
  }
  if (provider === "antigravity") {
    return parseAntigravityUsage(payload);
  }
  if (provider === "geminicli") {
    return parseGeminiCliUsage(payload);
  }
  if (provider === "claudecode") {
    return parseClaudeCodeUsage(payload);
  }
  return [];
}

export function formatUsagePercent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) {
    return "-";
  }
  const rounded = Math.round(value * 10) / 10;
  return `${Number.isInteger(rounded) ? rounded.toFixed(0) : rounded.toFixed(1)}%`;
}
