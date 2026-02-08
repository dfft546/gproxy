
export type NavItem = {
  id: string;
  label: string;
};

export function Sidebar({
  active,
  onChange,
  items
}: {
  active: string;
  onChange: (next: string) => void;
  items: NavItem[];
}) {
  return (
    <aside className="sidebar-shell">
      <nav className="space-y-1">
        {items.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onChange(item.id)}
            className={`nav-item ${active === item.id ? "nav-item-active" : ""}`}
          >
            {item.label}
          </button>
        ))}
      </nav>
    </aside>
  );
}
