import { NavLink, Outlet } from "react-router-dom";

const link = ({ isActive }: { isActive: boolean }) =>
  `rounded-lg px-3 py-2 text-sm transition ${
    isActive
      ? "bg-neutral-900 text-white"
      : "text-neutral-600 hover:bg-neutral-200/60 hover:text-neutral-900"
  }`;

export default function Layout() {
  return (
    <div className="flex min-h-screen">
      <aside className="flex w-56 shrink-0 flex-col border-r border-neutral-200 bg-white px-4 py-6">
        <div className="mb-1 text-sm font-semibold tracking-tight">Local Lead</div>
        <div className="mb-6 text-xs text-neutral-500">Prospector · MVP</div>
        <nav className="flex flex-col gap-1">
          <NavLink to="/" className={link} end>
            Dashboard
          </NavLink>
          <NavLink to="/search" className={link}>
            Search
          </NavLink>
          <NavLink to="/leads" className={link}>
            Leads
          </NavLink>
          <NavLink to="/settings" className={link}>
            Settings
          </NavLink>
        </nav>
        <div className="mt-auto text-[11px] leading-relaxed text-neutral-400">
          Local-first.
          <br />
          Keys never leave your machine except to Google Places.
        </div>
      </aside>
      <main className="mx-auto w-full max-w-5xl px-8 py-8">
        <Outlet />
      </main>
    </div>
  );
}
