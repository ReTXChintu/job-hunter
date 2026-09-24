/**
 * Agent protocol: the names of the Tauri commands and events exchanged
 * between the React UI and the Rust agent runtime, with typed payloads.
 */
import type {
  AgentEvent,
  AgentRun,
  AgentStatus,
  AnswerRecord,
  Application,
  ApplicationAnswer,
  ApplicationDetail,
  ApplicationListItem,
  ApplicationStatus,
  AppSettings,
  CandidateProfile,
  CoverLetter,
  Dashboard,
  Experience,
  Job,
  JobDetail,
  JobHuntOptions,
  JobListItem,
  LogEntry,
  LogLevel,
  MasterResumeRef,
  PairingCode,
  Project,
  RemoteDevice,
  RemoteStatus,
  Resume,
  ResumeDocument,
  SetupStatus,
  SyncStatus,
} from "@job-hunter/types";

/** Tauri events emitted by the backend. */
export const EVENTS = {
  agentEvent: "agent:event",
  agentStatus: "agent:status",
  syncStatus: "sync:status",
  log: "log:entry",
  dataChanged: "data:changed",
  remoteStatus: "remote:status",
} as const;

export interface EventPayloads {
  "agent:event": AgentEvent;
  "agent:status": AgentStatus;
  "sync:status": SyncStatus;
  "log:entry": LogEntry;
  "data:changed": string[];
  "remote:status": RemoteStatus;
}

/** Every command: name → { args, result }. */
export interface Commands {
  get_setup_status: { args: Record<string, never>; result: SetupStatus };
  get_settings: { args: Record<string, never>; result: AppSettings };
  save_settings: { args: { settings: AppSettings }; result: AppSettings };
  configure_mongodb: { args: { uri: string; database: string }; result: SyncStatus };
  test_mongodb: { args: { uri: string; database: string }; result: string };
  clear_mongodb: { args: Record<string, never>; result: null };
  get_sync_status: { args: Record<string, never>; result: SyncStatus };

  get_candidate_profile: { args: Record<string, never>; result: CandidateProfile };
  save_candidate_profile: { args: { profile: CandidateProfile }; result: CandidateProfile };
  list_experiences: { args: Record<string, never>; result: Experience[] };
  save_experience: { args: { experience: Experience }; result: Experience };
  delete_experience: { args: { id: string }; result: null };
  list_projects: { args: Record<string, never>; result: Project[] };
  save_project: { args: { project: Project }; result: Project };
  delete_project: { args: { id: string }; result: null };
  import_master_resume: { args: { path: string }; result: MasterResumeRef };
  parse_master_resume: { args: Record<string, never>; result: CandidateProfile };
  get_master_resume_text: { args: Record<string, never>; result: string | null };

  list_jobs: { args: Record<string, never>; result: JobListItem[] };
  get_job: { args: { id: string }; result: JobDetail };
  set_job_saved: { args: { id: string; saved: boolean }; result: Job };
  reject_job: { args: { id: string }; result: Job };
  delete_job: { args: { id: string }; result: null };

  list_applications: { args: Record<string, never>; result: ApplicationListItem[] };
  get_application: { args: { id: string }; result: ApplicationDetail };
  approve_application: { args: { id: string; applyNow?: boolean }; result: Application };
  reject_application: { args: { id: string; reason?: string }; result: Application };
  apply_application: { args: { id: string; simulate?: string }; result: AgentRun };
  answer_application_questions: { args: { id: string; answers: ApplicationAnswer[]; saveForReuse?: boolean }; result: Application };
  mark_manual_application_complete: { args: { id: string; note?: string }; result: Application };
  set_application_status: { args: { id: string; status: ApplicationStatus; note?: string }; result: Application };
  add_application_note: { args: { id: string; note: string }; result: Application };

  list_resumes: { args: Record<string, never>; result: Resume[] };
  update_resume_content: { args: { id: string; content: ResumeDocument }; result: Resume };
  update_cover_letter: { args: { id: string; text: string }; result: CoverLetter };
  read_text_file: { args: { path: string }; result: string };

  list_answers: { args: Record<string, never>; result: AnswerRecord[] };
  save_answer: { args: { record: AnswerRecord }; result: AnswerRecord };
  delete_answer: { args: { id: string }; result: null };

  get_agent_status: { args: Record<string, never>; result: AgentStatus };
  start_job_hunt: { args: { options?: JobHuntOptions }; result: AgentRun };
  stop_job_hunt: { args: Record<string, never>; result: boolean };
  pause_agent: { args: Record<string, never>; result: null };
  resume_agent: { args: Record<string, never>; result: null };
  discover_jobs: { args: { sources?: string[] }; result: AgentRun };
  analyze_job: { args: { jobId: string }; result: AgentRun };
  generate_resume: { args: { jobId: string }; result: AgentRun };
  generate_cover_letter: { args: { jobId: string }; result: AgentRun };
  list_agent_runs: { args: { limit?: number }; result: AgentRun[] };
  get_run_events: { args: { runId: string; limit?: number }; result: AgentEvent[] };

  get_logs: { args: { minLevel?: LogLevel; limit?: number }; result: LogEntry[] };
  clear_logs: { args: Record<string, never>; result: null };
  export_logs: { args: Record<string, never>; result: string };

  open_url: { args: { url: string }; result: null };
  open_path: { args: { path: string }; result: null };
  get_dashboard: { args: Record<string, never>; result: Dashboard };

  get_remote_status: { args: Record<string, never>; result: RemoteStatus };
  remote_register: {
    args: { relayUrl: string; email: string; password: string; deviceName?: string; allowInsecure?: boolean };
    result: RemoteStatus;
  };
  remote_login: {
    args: { relayUrl: string; email: string; password: string; deviceName?: string; allowInsecure?: boolean };
    result: RemoteStatus;
  };
  remote_logout: { args: Record<string, never>; result: RemoteStatus };
  remote_create_pairing_code: { args: Record<string, never>; result: PairingCode };
  remote_list_devices: { args: Record<string, never>; result: RemoteDevice[] };
  remote_revoke_device: { args: { deviceId: string }; result: RemoteStatus };
}

export type CommandName = keyof Commands;
export type CommandArgs<N extends CommandName> = Commands[N]["args"];
export type CommandResult<N extends CommandName> = Commands[N]["result"];

/** Machine-readable event kinds the agent emits (see Rust `AgentEvent.kind`). */
export const EVENT_KINDS = [
  "STATE_CHANGED",
  "STEP_STARTED",
  "STEP_DONE",
  "STEP_FAILED",
  "PROGRESS",
  "MESSAGE",
  "JOB_DISCOVERED",
  "JOB_ANALYZED",
  "RESUME_GENERATED",
  "APPLICATION_READY",
  "APPLICATION_UPDATED",
  "CLAUDE_ACTIVITY",
  "RUN_FINISHED",
] as const;

export type EventKind = (typeof EVENT_KINDS)[number];
