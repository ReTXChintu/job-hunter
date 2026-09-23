import { createRootRoute, createRoute, createRouter, createHashHistory } from "@tanstack/react-router";
import { AppShell } from "./components/AppShell";
import { DashboardPage } from "./pages/Dashboard";
import { JobsPage } from "./pages/Jobs";
import { JobDetailPage } from "./pages/JobDetail";
import { ApplicationsPage } from "./pages/Applications";
import { ApplicationReviewPage } from "./pages/ApplicationReview";
import { CandidatePage } from "./pages/Candidate";
import { ResumesPage } from "./pages/Resumes";
import { AgentPage } from "./pages/Agent";
import { SettingsPage } from "./pages/Settings";
import { SetupPage } from "./pages/Setup";

const rootRoute = createRootRoute({ component: AppShell });

export const dashboardRoute = createRoute({ getParentRoute: () => rootRoute, path: "/", component: DashboardPage });
export const jobsRoute = createRoute({ getParentRoute: () => rootRoute, path: "/jobs", component: JobsPage });
export const jobDetailRoute = createRoute({ getParentRoute: () => rootRoute, path: "/jobs/$jobId", component: JobDetailPage });
export const applicationsRoute = createRoute({ getParentRoute: () => rootRoute, path: "/applications", component: ApplicationsPage });
export const applicationReviewRoute = createRoute({ getParentRoute: () => rootRoute, path: "/applications/$applicationId", component: ApplicationReviewPage });
export const candidateRoute = createRoute({ getParentRoute: () => rootRoute, path: "/candidate", component: CandidatePage });
export const resumesRoute = createRoute({ getParentRoute: () => rootRoute, path: "/resumes", component: ResumesPage });
export const agentRoute = createRoute({ getParentRoute: () => rootRoute, path: "/agent", component: AgentPage });
export const settingsRoute = createRoute({ getParentRoute: () => rootRoute, path: "/settings", component: SettingsPage });
export const setupRoute = createRoute({ getParentRoute: () => rootRoute, path: "/setup", component: SetupPage });

const routeTree = rootRoute.addChildren([
  dashboardRoute,
  jobsRoute,
  jobDetailRoute,
  applicationsRoute,
  applicationReviewRoute,
  candidateRoute,
  resumesRoute,
  agentRoute,
  settingsRoute,
  setupRoute,
]);

export const router = createRouter({ routeTree, history: createHashHistory(), defaultPreload: "intent" });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
