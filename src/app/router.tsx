import { BrowserRouter, Route, Routes } from "react-router-dom";
import { AppShell } from "../components/layout/AppShell";
import DashboardPage from "../pages/dashboard";
import DiagnosticsPage from "../pages/diagnostics";
import LivePreviewPage from "../pages/live-preview";
import SystemSettingsPage from "../pages/system-settings";
import TargetWindowPage from "../pages/target-window";
import TaskConfigPage from "../pages/task-config";
import TemplatesPage from "../pages/templates";

export function AppRouter() {
  return (
    <BrowserRouter
      future={{
        v7_startTransition: true,
        v7_relativeSplatPath: true
      }}
    >
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/task-config" element={<TaskConfigPage />} />
          <Route path="/templates" element={<TemplatesPage />} />
          <Route path="/target-window" element={<TargetWindowPage />} />
          <Route path="/live-preview" element={<LivePreviewPage />} />
          <Route path="/diagnostics" element={<DiagnosticsPage />} />
          <Route path="/system-settings" element={<SystemSettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
