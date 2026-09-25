import { Layout } from "./components/Layout";
import { ApplicationDetailPage } from "./pages/ApplicationDetail";
import { ApplicationsPage } from "./pages/Applications";
import { JobsPage } from "./pages/Jobs";
import { LoginPage } from "./pages/Login";
import { OverviewPage } from "./pages/Overview";
import { useRoute } from "./route";
import { useSession } from "./session";

export function App() {
  const { session } = useSession();
  const route = useRoute();
  if (!session) return <LoginPage />;
  return (
    <Layout route={route}>
      {route.page === "overview" ? <OverviewPage /> : null}
      {route.page === "applications" ? <ApplicationsPage group={route.group} /> : null}
      {route.page === "application" ? <ApplicationDetailPage id={route.id} /> : null}
      {route.page === "jobs" ? <JobsPage /> : null}
    </Layout>
  );
}
