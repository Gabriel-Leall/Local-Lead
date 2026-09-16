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
        <div className="mb-1 text-sm font-semibold tracking-tight">Caça-Cliente</div>
        <div className="mb-6 text-xs text-neutral-500">Radar de negócios locais</div>
        <nav className="flex flex-col gap-1">
          <NavLink to="/" className={link} end>
            Buscar
          </NavLink>
          <NavLink to="/leads" className={link}>
            Leads
          </NavLink>
          <NavLink to="/painel" className={link}>
            Painel
          </NavLink>
          <NavLink to="/config" className={link}>
            Configurações
          </NavLink>
        </nav>
        <div className="mt-auto text-[11px] leading-relaxed text-neutral-400">
          Local-first.
          <br />
          Sua chave só sai da máquina para o Google Places.
        </div>
      </aside>
      <main className="mx-auto w-full max-w-5xl px-8 py-8">
        <Outlet />
      </main>
    </div>
  );
}
