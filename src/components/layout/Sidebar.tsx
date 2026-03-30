import { NavLink } from "react-router-dom";

const navigationItems = [
  { to: "/", label: "总览", badge: "Live" },
  { to: "/task-config", label: "任务配置", badge: "Config" },
  { to: "/templates", label: "模板库", badge: "Assets" },
  { to: "/target-window", label: "目标窗口", badge: "Win32" },
  { to: "/live-preview", label: "实时预览", badge: "Preview" },
  { to: "/diagnostics", label: "日志诊断", badge: "Logs" },
  { to: "/system-settings", label: "系统设置", badge: "System" }
];

export function Sidebar() {
  return (
    <aside className="relative flex w-[260px] shrink-0 flex-col overflow-hidden border-r border-white/5 bg-panel/20 backdrop-blur-xl">
      <div className="flex shrink-0 items-center px-6 pt-8 pb-6">
        <div className="flex flex-col gap-1">
          <h1 className="text-xl font-medium tracking-wide text-white/90">Autoclick</h1>
          <p className="text-[12px] text-muted tracking-wide">Tauri 2 Console</p>
        </div>
      </div>
      <nav className="flex-1 overflow-y-auto px-4 pb-6">
        <div className="flex flex-col gap-1">
          <p className="px-2 mb-2 mt-4 text-[10px] font-semibold uppercase tracking-widest text-muted/50">Menu</p>
          {navigationItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === "/"}
              className={({ isActive }) =>
                [
                  "group relative flex items-center justify-between rounded-lg px-3 py-2.5 transition-all duration-300 outline-none",
                  isActive
                    ? "bg-gradient-to-r from-accent/20 to-transparent text-white before:absolute before:left-0 before:top-1/4 before:bottom-1/4 before:w-1 before:rounded-r-full before:bg-accent"
                    : "text-muted hover:bg-white/[0.04] hover:text-white focus-visible:bg-white/[0.04]"
                ].join(" ")
              }
            >
              <span className="text-[13px] font-medium tracking-wide">{item.label}</span>
              <span className="text-[10px] uppercase tracking-widest opacity-40 group-hover:opacity-60 transition-opacity">
                {item.badge}
              </span>
            </NavLink>
          ))}
        </div>
      </nav>
    </aside>
  );
}

