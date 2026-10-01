import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AnswerRecord, AppSettings, ApplicationAnswer, ApplicationStatus, CandidateProfile, Experience, JobHuntOptions, LogLevel, Project, ProjectPick, ResumeDocument } from "@job-hunter/types";
import type { SkillGroup } from "@job-hunter/agent-protocol";
import { invoke } from "./tauri";

interface AuthArgs {
  email: string;
  password: string;
  deviceName?: string;
}

export const keys = {
  setup: ["setup"] as const,
  settings: ["settings"] as const,
  sync: ["sync"] as const,
  profile: ["profile"] as const,
  experiences: ["experiences"] as const,
  projects: ["projects"] as const,
  masterResumeText: ["masterResumeText"] as const,
  jobs: ["jobs"] as const,
  job: (id: string) => ["job", id] as const,
  applications: ["applications"] as const,
  application: (id: string) => ["application", id] as const,
  resumes: ["resumes"] as const,
  answers: ["answers"] as const,
  agentStatus: ["agentStatus"] as const,
  runs: ["runs"] as const,
  runEvents: (id: string) => ["runEvents", id] as const,
  logs: (level: LogLevel) => ["logs", level] as const,
  dashboard: ["dashboard"] as const,
  file: (path: string) => ["file", path] as const,
  remoteStatus: ["remoteStatus"] as const,
  remoteDevices: ["remoteDevices"] as const,
  platformProfiles: ["platformProfiles"] as const,
  notifications: ["notifications"] as const,
  publishingPlan: ["publishingPlan"] as const,
};

/** Collections → query keys to invalidate when the backend reports a change. */
export const COLLECTION_KEYS: Record<string, readonly (readonly string[])[]> = {
  jobs: [keys.jobs, keys.dashboard, ["job"]],
  job_analyses: [keys.jobs, keys.dashboard, ["job"], ["application"]],
  applications: [keys.applications, keys.jobs, keys.dashboard, ["application"], ["job"]],
  resumes: [keys.resumes, ["job"], ["application"]],
  cover_letters: [["job"], ["application"]],
  candidate_profiles: [keys.profile, keys.setup, keys.platformProfiles],
  experiences: [keys.experiences, keys.platformProfiles],
  projects: [keys.projects, keys.platformProfiles],
  platform_profiles: [keys.platformProfiles],
  publishing_plans: [keys.publishingPlan, keys.platformProfiles],
  notifications: [keys.notifications],
  application_answers: [keys.answers],
  agent_runs: [keys.runs, keys.dashboard],
  agent_events: [["runEvents"]],
  settings: [keys.settings],
  users: [],
};

export const useSetupStatus = () => useQuery({ queryKey: keys.setup, queryFn: () => invoke("get_setup_status"), staleTime: 30_000 });
export const useSettings = () => useQuery({ queryKey: keys.settings, queryFn: () => invoke("get_settings") });
export const useSyncStatus = () => useQuery({ queryKey: keys.sync, queryFn: () => invoke("get_sync_status"), refetchInterval: 30_000 });
export const useProfile = () => useQuery({ queryKey: keys.profile, queryFn: () => invoke("get_candidate_profile") });
export const useExperiences = () => useQuery({ queryKey: keys.experiences, queryFn: () => invoke("list_experiences") });
export const useProjects = () => useQuery({ queryKey: keys.projects, queryFn: () => invoke("list_projects") });
export const useMasterResumeText = () => useQuery({ queryKey: keys.masterResumeText, queryFn: () => invoke("get_master_resume_text") });
export const useJobs = () => useQuery({ queryKey: keys.jobs, queryFn: () => invoke("list_jobs") });
export const useJob = (id: string) => useQuery({ queryKey: keys.job(id), queryFn: () => invoke("get_job", { id }), enabled: !!id });
export const useApplications = () => useQuery({ queryKey: keys.applications, queryFn: () => invoke("list_applications") });
export const useApplication = (id: string) => useQuery({ queryKey: keys.application(id), queryFn: () => invoke("get_application", { id }), enabled: !!id });
export const useResumes = () => useQuery({ queryKey: keys.resumes, queryFn: () => invoke("list_resumes") });
export const useAnswers = () => useQuery({ queryKey: keys.answers, queryFn: () => invoke("list_answers") });
export const useAgentStatus = () => useQuery({ queryKey: keys.agentStatus, queryFn: () => invoke("get_agent_status"), refetchInterval: 15_000 });
export const useRuns = (limit = 20) => useQuery({ queryKey: [...keys.runs, limit], queryFn: () => invoke("list_agent_runs", { limit }) });
export const useRunEvents = (runId: string | null) => useQuery({ queryKey: keys.runEvents(runId ?? ""), queryFn: () => invoke("get_run_events", { runId: runId!, limit: 500 }), enabled: !!runId });
export const useLogs = (minLevel: LogLevel) => useQuery({ queryKey: keys.logs(minLevel), queryFn: () => invoke("get_logs", { minLevel, limit: 1000 }) });
export const useDashboard = () => useQuery({ queryKey: keys.dashboard, queryFn: () => invoke("get_dashboard"), refetchInterval: 20_000 });
export const useTextFile = (path: string | null | undefined) => useQuery({ queryKey: keys.file(path ?? ""), queryFn: () => invoke("read_text_file", { path: path! }), enabled: !!path, staleTime: 60_000 });

function useInvalidate() {
  const qc = useQueryClient();
  return (...groups: (readonly (readonly string[])[])[]) => {
    for (const group of groups) for (const key of group) void qc.invalidateQueries({ queryKey: key });
  };
}

export function useSaveSettings() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (settings: AppSettings) => invoke("save_settings", { settings }), onSuccess: () => inv([keys.settings, keys.setup, keys.dashboard]) });
}

export const useServerInfo = () => useQuery({ queryKey: ["serverInfo"], queryFn: () => invoke("get_server_info"), staleTime: Infinity });

export const useAboutInfo = () => useQuery({ queryKey: ["aboutInfo"], queryFn: () => invoke("get_about_info"), staleTime: Infinity });

/** Checked on start and every 6 hours; also refetched by "Check for updates". */
export const useUpdateCheck = () =>
  useQuery({ queryKey: ["updateCheck"], queryFn: () => invoke("check_for_update"), staleTime: 60 * 60_000, refetchInterval: 6 * 60 * 60_000, retry: false });

export function useInstallUpdate() {
  return useMutation({ mutationFn: () => invoke("install_update") });
}

export function useTestBackend() {
  return useMutation({ mutationFn: () => invoke("test_backend") });
}

export function useBackendRegister() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: AuthArgs) => invoke("backend_register", args), onSuccess: () => inv([keys.setup, keys.sync, keys.settings, keys.dashboard]) });
}

export function useBackendLogin() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: AuthArgs) => invoke("backend_login", args), onSuccess: () => inv([keys.setup, keys.sync, keys.settings, keys.dashboard]) });
}

export function useBackendLogout() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("backend_logout"), onSuccess: () => inv([keys.setup, keys.sync, keys.settings, keys.dashboard]) });
}

export function useSaveProfile() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (profile: CandidateProfile) => invoke("save_candidate_profile", { profile }), onSuccess: () => inv([keys.profile, keys.setup]) });
}

export function useSaveExperience() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (experience: Experience) => invoke("save_experience", { experience }), onSuccess: () => inv([keys.experiences]) });
}

export function useDeleteExperience() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (id: string) => invoke("delete_experience", { id }), onSuccess: () => inv([keys.experiences]) });
}

export function useSaveProject() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (project: Project) => invoke("save_project", { project }), onSuccess: () => inv([keys.projects]) });
}

/** Claude drafts a project from the candidate's description; nothing is saved. */
export function useDraftProject() {
  return useMutation({ mutationFn: (args: { description: string; experienceId?: string | null }) => invoke("draft_project", args) });
}

export function useDeleteProject() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (id: string) => invoke("delete_project", { id }), onSuccess: () => inv([keys.projects]) });
}

/** "I know this": add skills an analysis listed as missing to the profile. */
export function useMarkSkillsKnown() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (args: { skills: string[]; group: SkillGroup }) => invoke("mark_skills_known", args),
    onSuccess: () => inv([keys.profile, keys.jobs, keys.platformProfiles, ["job"], ["application"]]),
  });
}

// ---- job-site profiles ---------------------------------------------------------------------

export const usePlatformProfiles = () => useQuery({ queryKey: keys.platformProfiles, queryFn: () => invoke("list_platform_profiles") });
export const usePublishingPlan = () => useQuery({ queryKey: keys.publishingPlan, queryFn: () => invoke("get_publishing_plan") });

/** Claude suggests which projects to feature; nothing is saved. */
export function useSuggestProjectPicks() {
  return useMutation({ mutationFn: () => invoke("suggest_project_picks") });
}

export function useSavePublishingPlan() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (args: { featuredProjectIds: string[]; picks: ProjectPick[] }) => invoke("save_publishing_plan", args),
    onSuccess: () => inv([keys.publishingPlan, keys.platformProfiles]),
  });
}

/** Update a site's profile; `resume` continues an update that stopped part-way. */
export function useSyncPlatformProfile() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (args: { platform: string; resume?: boolean }) => invoke("sync_platform_profile", args),
    onSuccess: () => inv([keys.platformProfiles, keys.agentStatus, keys.runs]),
  });
}

/** Look for employers' replies in Gmail (inbox and spam) now. */
export function useCheckInbox() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("check_inbox_now"), onSuccess: () => inv([keys.agentStatus, keys.runs]) });
}

/** Re-create an application's files that were deleted after it was sent. */
export function useRestoreApplicationFiles() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (id: string) => invoke("restore_application_files", { id }), onSuccess: () => inv([keys.resumes, ["application"], ["job"], ["file"]]) });
}

export const useNotifications = () => useQuery({ queryKey: keys.notifications, queryFn: () => invoke("list_notifications", { limit: 50 }) });

export function useMarkNotificationsRead() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (ids?: string[]) => invoke("mark_notifications_read", { ids }), onSuccess: () => inv([keys.notifications]) });
}

export function useAnswerPlatformQuestions() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (args: { platform: string; answers: ApplicationAnswer[] }) => invoke("answer_platform_questions", args),
    onSuccess: () => inv([keys.platformProfiles, keys.answers, keys.agentStatus, keys.runs]),
  });
}

/** A job the user found: text and/or screenshots/PDFs → resume, application, Gmail draft. */
export function useAddSharedJob() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (args: { text: string; files: string[]; draftEmail: boolean }) => invoke("add_shared_job", args),
    onSuccess: () => inv([keys.agentStatus, keys.runs]),
  });
}

/** "Update all now": queue every site that's behind. */
export function useQueueProfileUpdates() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("queue_profile_updates"), onSuccess: () => inv([keys.platformProfiles, keys.agentStatus]) });
}

export function useSetPlatformAutoSync() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { platform: string; enabled: boolean }) => invoke("set_platform_auto_sync", args), onSuccess: () => inv([keys.platformProfiles]) });
}

export function useImportMasterResume() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (path: string) => invoke("import_master_resume", { path }), onSuccess: () => inv([keys.profile, keys.setup, keys.masterResumeText]) });
}

export function useParseMasterResume() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("parse_master_resume"), onSuccess: () => inv([keys.profile, keys.setup, keys.experiences, keys.projects]) });
}

export function useSetJobSaved() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; saved: boolean }) => invoke("set_job_saved", args), onSuccess: () => inv([keys.jobs, ["job"]]) });
}

export function useRejectJob() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (id: string) => invoke("reject_job", { id }), onSuccess: () => inv([keys.jobs, keys.applications, keys.dashboard, ["job"], ["application"]]) });
}

export function useDeleteJob() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (id: string) => invoke("delete_job", { id }), onSuccess: () => inv([keys.jobs, keys.applications, keys.dashboard]) });
}

export function useApproveApplication() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; applyNow?: boolean }) => invoke("approve_application", args), onSuccess: () => inv([keys.applications, keys.jobs, keys.dashboard, keys.agentStatus, ["application"], ["job"]]) });
}

export function useRejectApplication() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; reason?: string }) => invoke("reject_application", args), onSuccess: () => inv([keys.applications, keys.jobs, keys.dashboard, ["application"], ["job"]]) });
}

export function useApplyApplication() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; simulate?: string }) => invoke("apply_application", args), onSuccess: () => inv([keys.applications, keys.agentStatus, keys.dashboard, ["application"]]) });
}

export function useAnswerQuestions() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; answers: ApplicationAnswer[]; saveForReuse?: boolean }) => invoke("answer_application_questions", args), onSuccess: () => inv([keys.applications, keys.answers, keys.agentStatus, ["application"]]) });
}

export function useMarkManualComplete() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; note?: string }) => invoke("mark_manual_application_complete", args), onSuccess: () => inv([keys.applications, keys.jobs, keys.dashboard, ["application"], ["job"]]) });
}

export function useSetApplicationStatus() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; status: ApplicationStatus; note?: string }) => invoke("set_application_status", args), onSuccess: () => inv([keys.applications, keys.jobs, keys.dashboard, ["application"], ["job"]]) });
}

export function useAddApplicationNote() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; note: string }) => invoke("add_application_note", args), onSuccess: () => inv([keys.applications, ["application"]]) });
}

export function useUpdateResumeContent() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; content: ResumeDocument }) => invoke("update_resume_content", args), onSuccess: () => inv([keys.resumes, ["job"], ["application"], ["file"]]) });
}

export function useUpdateCoverLetter() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: { id: string; text: string }) => invoke("update_cover_letter", args), onSuccess: () => inv([["job"], ["application"], ["file"]]) });
}

export function useSaveAnswer() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (record: AnswerRecord) => invoke("save_answer", { record }), onSuccess: () => inv([keys.answers]) });
}

export function useDeleteAnswer() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (id: string) => invoke("delete_answer", { id }), onSuccess: () => inv([keys.answers]) });
}

export function useStartJobHunt() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (options?: JobHuntOptions) => invoke("start_job_hunt", { options }), onSuccess: () => inv([keys.agentStatus, keys.runs, keys.dashboard]) });
}

export function useStopJobHunt() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("stop_job_hunt"), onSuccess: () => inv([keys.agentStatus]) });
}

export function usePauseAgent() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("pause_agent"), onSuccess: () => inv([keys.agentStatus]) });
}

export function useResumeAgent() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("resume_agent"), onSuccess: () => inv([keys.agentStatus]) });
}

export function useDiscoverJobs() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (sources?: string[]) => invoke("discover_jobs", { sources }), onSuccess: () => inv([keys.agentStatus, keys.runs]) });
}

export function useAnalyzeJob() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (jobId: string) => invoke("analyze_job", { jobId }), onSuccess: () => inv([keys.agentStatus, keys.runs]) });
}

export function useGenerateResume() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (jobId: string) => invoke("generate_resume", { jobId }), onSuccess: () => inv([keys.agentStatus, keys.runs]) });
}

export function useClearLogs() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("clear_logs"), onSuccess: () => inv([["logs"]]) });
}

export function useExportLogs() {
  return useMutation({ mutationFn: () => invoke("export_logs") });
}

export function useOpenUrl() {
  return useMutation({ mutationFn: (url: string) => invoke("open_url", { url }) });
}

export function useOpenPath() {
  return useMutation({ mutationFn: (path: string) => invoke("open_path", { path }) });
}

// ---- mobile companion app (relay) ------------------------------------------------------------

export const useRemoteStatus = () => useQuery({ queryKey: keys.remoteStatus, queryFn: () => invoke("get_remote_status") });
export const useRemoteDevices = (enabled: boolean) => useQuery({ queryKey: keys.remoteDevices, queryFn: () => invoke("remote_list_devices"), enabled, refetchInterval: enabled ? 15_000 : false });

export function useRemoteRegister() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: AuthArgs) => invoke("remote_register", args), onSuccess: () => inv([keys.remoteStatus, keys.remoteDevices, keys.setup]) });
}

export function useRemoteLogin() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (args: AuthArgs) => invoke("remote_login", args), onSuccess: () => inv([keys.remoteStatus, keys.remoteDevices, keys.setup]) });
}

export function useRemoteLogout() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => invoke("remote_logout"), onSuccess: () => inv([keys.remoteStatus, keys.remoteDevices, keys.setup]) });
}

export function useRemoteCreatePairingCode() {
  return useMutation({ mutationFn: () => invoke("remote_create_pairing_code") });
}

export function useRemoteRevokeDevice() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: (deviceId: string) => invoke("remote_revoke_device", { deviceId }), onSuccess: () => inv([keys.remoteStatus, keys.remoteDevices]) });
}
