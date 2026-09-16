import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import Layout from "./components/Layout";
import DashboardPage from "./pages/DashboardPage";
import LeadDetailPage from "./pages/LeadDetailPage";
import LeadsPage from "./pages/LeadsPage";
import SearchJobPage from "./pages/SearchJobPage";
import SearchPage from "./pages/SearchPage";
import SettingsPage from "./pages/SettingsPage";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route index element={<SearchPage />} />
          <Route path="buscar" element={<SearchPage />} />
          <Route path="jobs/:id" element={<SearchJobPage />} />
          <Route path="leads" element={<LeadsPage />} />
          <Route path="leads/:id" element={<LeadDetailPage />} />
          <Route path="painel" element={<DashboardPage />} />
          <Route path="config" element={<SettingsPage />} />
          {/* aliases antigos em inglês */}
          <Route path="search" element={<Navigate to="/" replace />} />
          <Route path="settings" element={<Navigate to="/config" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
