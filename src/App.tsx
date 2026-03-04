import { Routes, Route, Navigate } from "react-router-dom";
import { Sidebar } from "./components/layout/Sidebar";
import { Header } from "./components/layout/Header";
import { DashboardPage } from "./pages/DashboardPage";
import { ProjectsPage } from "./pages/ProjectsPage";
import { GCPage } from "./pages/GCPage";
import { ConflictsPage } from "./pages/ConflictsPage";
import { RulesPage } from "./pages/RulesPage";
import { PrunerPage } from "./pages/PrunerPage";
import { SettingsPage } from "./pages/SettingsPage";

export default function App() {
  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <Header />
        <main className="flex-1 overflow-y-auto p-6">
          <Routes>
            <Route path="/" element={<Navigate to="/projects" replace />} />
            <Route path="/projects" element={<ProjectsPage />} />
            <Route path="/dashboard" element={<DashboardPage />} />
            <Route path="/gc" element={<GCPage />} />
            <Route path="/conflicts" element={<ConflictsPage />} />
            <Route path="/rules" element={<RulesPage />} />
            <Route path="/pruner" element={<PrunerPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </main>
      </div>
    </div>
  );
}
