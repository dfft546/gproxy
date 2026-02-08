
import type { ReactNode } from "react";

export function Card({
  title,
  subtitle,
  action,
  children
}: {
  title?: string;
  subtitle?: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="card-shell">
      {(title || subtitle || action) && (
        <header className="mb-5 flex flex-wrap items-start justify-between gap-3">
          <div>
            {title ? <h3 className="text-lg font-semibold text-slate-900">{title}</h3> : null}
            {subtitle ? <p className="mt-1 text-sm text-slate-500">{subtitle}</p> : null}
          </div>
          {action}
        </header>
      )}
      {children}
    </section>
  );
}

export function Button({
  children,
  type = "button",
  variant = "primary",
  disabled,
  onClick
}: {
  children: ReactNode;
  type?: "button" | "submit";
  variant?: "primary" | "neutral" | "danger";
  disabled?: boolean;
  onClick?: () => void;
}) {
  return (
    <button type={type} disabled={disabled} onClick={onClick} className={`btn btn-${variant}`}>
      {children}
    </button>
  );
}

export function FieldLabel({ children }: { children: ReactNode }) {
  return <label className="block text-xs font-semibold uppercase tracking-[0.12em] text-slate-500">{children}</label>;
}

export function TextInput({
  value,
  onChange,
  placeholder,
  type = "text",
  disabled
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "password" | "number";
  disabled?: boolean;
}) {
  return (
    <input
      className="input"
      value={value}
      type={type}
      disabled={disabled}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
    />
  );
}

export function TextArea({
  value,
  onChange,
  rows = 4,
  placeholder
}: {
  value: string;
  onChange: (value: string) => void;
  rows?: number;
  placeholder?: string;
}) {
  return (
    <textarea
      rows={rows}
      className="textarea"
      value={value}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
    />
  );
}

export function SelectInput({
  value,
  options,
  onChange
}: {
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (value: string) => void;
}) {
  return (
    <select className="select" value={value} onChange={(event) => onChange(event.target.value)}>
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}

export function Badge({ children, active }: { children: ReactNode; active?: boolean }) {
  return <span className={`badge ${active ? "badge-active" : ""}`}>{children}</span>;
}

export function Divider() {
  return <div className="h-px w-full bg-slate-200/80" />;
}
