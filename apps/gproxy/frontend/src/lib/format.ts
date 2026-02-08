export function formatDateTime(value?: string | number | null): string {
  if (!value) {
    return "-";
  }
  const date = typeof value === "number" ? new Date(value) : new Date(String(value));
  if (Number.isNaN(date.getTime())) {
    return "-";
  }
  return date.toLocaleString();
}

export function mask(value: string, head = 4, tail = 3): string {
  if (!value) {
    return "";
  }
  if (value.length <= head + tail) {
    return value;
  }
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

export function nowRfc3339(): string {
  return new Date().toISOString();
}

export function beforeHoursRfc3339(hours: number): string {
  return new Date(Date.now() - hours * 3600 * 1000).toISOString();
}

export function asNumber(text: string): number {
  const value = Number(text);
  if (Number.isNaN(value)) {
    throw new Error("invalid_number");
  }
  return value;
}

export function optional(text: string): string | null {
  const v = text.trim();
  return v.length ? v : null;
}

export function toRfc3339Input(value: string): string {
  return value.replace(/\.\d{3}Z$/, "Z");
}
