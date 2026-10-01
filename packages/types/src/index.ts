/**
 * Shared domain types. These mirror the Rust structs in
 * `crates/job-hunter-core/src/domain` (serde camelCase) exactly.
 */

export type RemotePreference = "ANY" | "REMOTE" | "HYBRID" | "ONSITE";
export type RelocationPreference = "NOT_SPECIFIED" | "YES" | "NO" | "MAYBE";
export type EmploymentType = "FULL_TIME" | "PART_TIME" | "CONTRACT" | "FREELANCE" | "INTERNSHIP";
export type ResumeFormat = "PDF" | "DOCX";

export interface PersonalInfo {
  name: string;
  email: string;
  phone: string;
  location: string;
  linkedin: string;
  github: string;
  portfolio: string;
  currentTitle: string;
}

export interface SalaryPreference {
  minimum: number | null;
  maximum: number | null;
  currency: string;
  period: string;
}

export interface CareerPreferences {
  targetRoles: string[];
  preferredLocations: string[];
  remotePreference: RemotePreference;
  hybridAcceptable: boolean;
  relocation: RelocationPreference;
  minimumExperienceYears: number | null;
  salary: SalaryPreference;
  employmentTypes: EmploymentType[];
  noticePeriod: string;
  notes: string;
}

export interface SkillGroups {
  frontend: string[];
  backend: string[];
  database: string[];
  devops: string[];
  cloud: string[];
  testing: string[];
  other: string[];
}

export interface Education {
  id: string;
  institution: string;
  degree: string;
  field: string;
  startDate: string;
  endDate: string;
  grade: string;
  description: string;
}

export interface Certification {
  id: string;
  name: string;
  issuer: string;
  date: string;
  url: string;
}

export interface MasterResumeRef {
  id: string;
  originalFileName: string;
  storedPath: string;
  textPath: string;
  format: ResumeFormat;
  importedAt: string;
  textChars: number;
}

export interface CandidateProfile {
  id: string;
  userId: string;
  personal: PersonalInfo;
  preferences: CareerPreferences;
  skills: SkillGroups;
  education: Education[];
  certifications: Certification[];
  achievements: string[];
  languages: string[];
  masterResume: MasterResumeRef | null;
  summary: string;
  createdAt: string;
  updatedAt: string;
}

export interface ProfileCompleteness {
  ready: boolean;
  missing: string[];
  hasMasterResume: boolean;
}

export interface Experience {
  id: string;
  userId: string;
  company: string;
  role: string;
  location: string;
  startDate: string;
  endDate: string | null;
  description: string;
  technologies: string[];
  achievements: string[];
  projects: string[];
  createdAt: string;
  updatedAt: string;
}

export interface Project {
  id: string;
  userId: string;
  experienceId: string | null;
  name: string;
  description: string;
  role: string;
  technologies: string[];
  responsibilities: string[];
  achievements: string[];
  url: string;
  createdAt: string;
  updatedAt: string;
}

// ---- jobs -----------------------------------------------------------------

export type JobStatus =
  | "DISCOVERED"
  | "ANALYZED"
  | "NOT_RELEVANT"
  | "SHORTLISTED"
  | "READY_FOR_REVIEW"
  | "APPROVED"
  | "APPLYING"
  | "APPLIED"
  | "MANUAL_ACTION_REQUIRED"
  | "WAITING_FOR_USER"
  | "REJECTED"
  | "INTERVIEW"
  | "OFFER"
  | "WITHDRAWN";

export interface JobSource {
  platform: string;
  url: string;
  sourceJobId: string | null;
}

export interface Job {
  id: string;
  userId: string;
  discoveredAt: string;
  postedAt: string | null;
  source: string;
  sourceJobId: string | null;
  url: string;
  canonicalUrl: string | null;
  company: string;
  title: string;
  location: string;
  employmentType: string | null;
  remote: string | null;
  salary: string | null;
  seniority: string | null;
  description: string;
  requirements: string[];
  responsibilities: string[];
  skills: string[];
  sources: JobSource[];
  status: JobStatus;
  runId: string | null;
  analysisId: string | null;
  applicationId: string | null;
  saved: boolean;
  detailsComplete: boolean;
  /** Set for postings that ask for the resume by email; the application is sent from Gmail. */
  applyEmail: string | null;
  contactName: string | null;
  dedupKey: string;
  createdAt: string;
  updatedAt: string;
}

export interface JobAnalysis {
  id: string;
  jobId: string;
  userId: string;
  relevant: boolean;
  matchScore: number;
  matchedSkills: string[];
  missingSkills: string[];
  requiredExperienceMet: boolean;
  seniorityMatch: boolean;
  locationMatch: boolean;
  employmentTypeMatch: boolean;
  salaryAssessment: string;
  requiredQualifications: string[];
  niceToHave: string[];
  concerns: string[];
  importantKeywords: string[];
  summary: string;
  runId: string | null;
  createdAt: string;
  updatedAt: string;
}

// ---- resumes ------------------------------------------------------------------

export interface ResumeContact {
  name: string;
  email: string;
  phone: string;
  location: string;
  linkedin: string;
  github: string;
  portfolio: string;
}

export interface ResumeSkillSection {
  name: string;
  items: string[];
}

export interface ResumeExperience {
  company: string;
  role: string;
  location: string;
  startDate: string;
  endDate: string;
  bullets: string[];
}

export interface ResumeProject {
  name: string;
  description: string;
  technologies: string[];
  bullets: string[];
  url: string;
}

export interface ResumeEducation {
  institution: string;
  degree: string;
  field: string;
  startDate: string;
  endDate: string;
  grade: string;
}

export interface ResumeDocument {
  contact: ResumeContact;
  headline: string;
  summary: string;
  skills: ResumeSkillSection[];
  experience: ResumeExperience[];
  projects: ResumeProject[];
  education: ResumeEducation[];
  certifications: string[];
  achievements: string[];
  languages: string[];
}

export type AtsStatus = "PASS" | "REVISE" | "FAIL";

export interface AtsValidation {
  status: AtsStatus;
  keywordCoverage: number;
  missingKeywords: string[];
  unsupportedClaims: string[];
  missingRequirements: string[];
  formattingIssues: string[];
  notes: string;
  iteration: number;
}

export type ResumeKind = "MASTER" | "GENERATED";

export interface Resume {
  id: string;
  userId: string;
  kind: ResumeKind;
  jobId: string | null;
  company: string;
  jobTitle: string;
  version: number;
  label: string;
  docxPath: string | null;
  pdfPath: string | null;
  htmlPath: string | null;
  jsonPath: string | null;
  content: ResumeDocument | null;
  validation: AtsValidation | null;
  runId: string | null;
  userEdited: boolean;
  /** Set when the files were deleted after applying; they can be re-created. */
  filesDeletedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CoverLetter {
  id: string;
  userId: string;
  jobId: string;
  resumeId: string | null;
  version: number;
  text: string;
  docxPath: string | null;
  pdfPath: string | null;
  txtPath: string | null;
  userEdited: boolean;
  filesDeletedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

// ---- applications ---------------------------------------------------------------

export type ApplicationStatus =
  | "DISCOVERED"
  | "ANALYZED"
  | "SHORTLISTED"
  | "READY_FOR_REVIEW"
  | "APPROVED"
  | "APPLYING"
  | "APPLIED"
  | "MANUAL_ACTION_REQUIRED"
  | "WAITING_FOR_USER"
  | "REJECTED"
  | "INTERVIEW"
  | "OFFER"
  | "WITHDRAWN";

export type AnswerSource = "KNOWN" | "USER" | "PROFILE" | "SKIPPED";

export interface ApplicationAnswer {
  question: string;
  answer: string;
  source: AnswerSource;
}

export interface PendingQuestion {
  id: string;
  question: string;
  fieldType: string;
  options: string[];
  required: boolean;
  context: string;
}

export interface StatusChange {
  status: ApplicationStatus;
  at: string;
  reason: string;
}

export interface Application {
  id: string;
  userId: string;
  jobId: string;
  status: ApplicationStatus;
  resumeId: string | null;
  coverLetterId: string | null;
  applicationUrl: string;
  source: string;
  notes: string;
  answers: ApplicationAnswer[];
  pendingQuestions: PendingQuestion[];
  statusHistory: StatusChange[];
  potentialIssues: string[];
  approvedAt: string | null;
  appliedAt: string | null;
  failureReason: string | null;
  evidence: string | null;
  claudeSessionId: string | null;
  runId: string | null;
  manualCompleted: boolean;
  /** Employers' replies found in Gmail (inbox or spam). */
  replies: EmailReply[];
  createdAt: string;
  updatedAt: string;
}

export type ReplyKind = "INTERVIEW" | "ASSESSMENT" | "QUESTION" | "OFFER" | "REJECTION" | "ACKNOWLEDGEMENT" | "OTHER";

/** An employer's email about an application, found in Gmail by the inbox check. */
export interface EmailReply {
  id: string;
  from: string;
  subject: string;
  receivedAt: string;
  folder: "INBOX" | "SPAM" | string;
  kind: ReplyKind | string;
  summary: string;
  foundAt: string;
}

export interface AnswerRecord {
  id: string;
  userId: string;
  question: string;
  normalizedQuestion: string;
  answer: string;
  category: string;
  timesUsed: number;
  createdAt: string;
  updatedAt: string;
}

// ---- agent -------------------------------------------------------------------------

export type AgentState =
  | "IDLE"
  | "INITIALIZING"
  | "DISCOVERING"
  | "EXTRACTING"
  | "DEDUPLICATING"
  | "ANALYZING"
  | "PREPARING_APPLICATIONS"
  | "WAITING_FOR_APPROVAL"
  | "APPLYING"
  | "UPDATING_PROFILE"
  | "CHECKING_INBOX"
  | "COMPLETED"
  | "FAILED"
  | "MANUAL_ACTION_REQUIRED"
  | "WAITING_FOR_USER"
  | "PAUSED"
  | "STOPPING";

export type EventLevel = "INFO" | "SUCCESS" | "WARN" | "ERROR";

export interface AgentEvent {
  id: string;
  runId: string;
  at: string;
  level: EventLevel;
  kind: string;
  message: string;
  data: unknown;
}

export interface RunStats {
  jobsDiscovered: number;
  jobsNew: number;
  duplicatesRemoved: number;
  jobsAnalyzed: number;
  relevant: number;
  awaitingApproval: number;
  approved: number;
  applied: number;
  manualAction: number;
  errors: number;
  claudeCostUsd: number;
}

export type RunKind = "JOB_HUNT" | "APPLICATION" | "RESUME_GENERATION" | "ANALYSIS" | "PROFILE_SYNC" | "INBOX_CHECK";

export interface AgentRun {
  id: string;
  userId: string;
  kind: RunKind;
  state: AgentState;
  startedAt: string;
  finishedAt: string | null;
  stats: RunStats;
  currentActivity: string | null;
  progress: number | null;
  error: string | null;
  mock: boolean;
  sources: string[];
  jobIds: string[];
  applicationId: string | null;
  updatedAt: string;
}

export interface AgentStatus {
  state: AgentState;
  runId: string | null;
  runKind: RunKind | null;
  currentActivity: string | null;
  progress: number | null;
  stats: RunStats;
  startedAt: string | null;
  error: string | null;
  mock: boolean;
  paused: boolean;
}

// ---- settings / status ---------------------------------------------------------------

export interface JobSourceConfig {
  platform: string;
  enabled: boolean;
}

export interface ResumeSettings {
  generateCoverLetter: boolean;
  atsMaxIterations: number;
  minimumKeywordCoverage: number;
  generatePdf: boolean;
  generateDocx: boolean;
  /** Delete an application's generated files once it's sent (content is kept). */
  deleteFilesAfterApplied: boolean;
}

export interface BrowserSettings {
  chromePath: string | null;
  keepTabsOpen: boolean;
}

export interface ClaudeSettings {
  cliPath: string | null;
  model: string | null;
  maxBudgetUsdPerCall: number;
  maxTurnsBrowser: number;
  timeoutSeconds: number;
}

export interface RemoteSettings {
  relayUrl: string;
  accountEmail: string;
  deviceId: string | null;
  deviceName: string;
}

/** Non-secret configuration for the self-hosted `@job-hunter/backend`
 * connection (see `apps/backend`, `docs/backend.md`). The device token
 * itself lives in the OS credential store, never here. */
export interface BackendSettings {
  backendUrl: string;
  accountEmail: string;
  deviceId: string | null;
  deviceName: string;
}

export interface AppSettings {
  jobSources: JobSourceConfig[];
  maxJobsPerSource: number;
  maxApplicationsPerRun: number;
  approvalMode: "REVIEW_REQUIRED";
  recencyDays: number;
  resume: ResumeSettings;
  browser: BrowserSettings;
  claude: ClaudeSettings;
  mockMode: boolean;
  setupCompleted: boolean;
  minimumMatchScore: number;
  remote: RemoteSettings;
  backend: BackendSettings;
  /** Hours between Gmail checks for employers' replies (inbox and spam); 0 = off. */
  inboxCheckHours: number;
  profileSync: ProfileSyncSettings;
}

/** When job-site profiles that fell behind update without clicking Update. */
export type ProfileSyncMode = "NIGHTLY" | "ASK" | "IMMEDIATE" | "OFF";

export interface ProfileSyncSettings {
  mode: ProfileSyncMode;
  /** Local hour (0-23); nightly updates start only in the 6 hours after it. */
  nightlyHour: number;
}

export interface ClaudeStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
  authenticated: boolean;
  authMethod: string | null;
  account: string | null;
  subscription: string | null;
  error: string | null;
}

export interface ChromeStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
  extensionInstalled: boolean;
  extensionVersion: string | null;
  profilesChecked: string[];
  error: string | null;
}

export interface SyncStatus {
  configured: boolean;
  connected: boolean;
  pending: number;
  lastSyncAt: string | null;
  lastError: string | null;
  target: string | null;
  syncing: boolean;
}

export interface SetupStatus {
  claude: ClaudeStatus;
  chrome: ChromeStatus;
  backend: SyncStatus;
  profile: ProfileCompleteness;
  mockMode: boolean;
  setupCompleted: boolean;
  dataDir: string;
}

export type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";

export interface LogEntry {
  id: string;
  at: string;
  level: LogLevel;
  target: string;
  message: string;
}

export interface UserFacingError {
  code: string;
  message: string;
  details: string | null;
  recoverable: boolean;
}

// ---- composite views returned by the backend ------------------------------------------

export interface JobListItem {
  job: Job;
  analysis: JobAnalysis | null;
  applicationStatus: ApplicationStatus | null;
}

export interface JobDetail {
  job: Job;
  analysis: JobAnalysis | null;
  application: Application | null;
  resume: Resume | null;
  coverLetter: CoverLetter | null;
  resumes: Resume[];
}

export interface ApplicationListItem {
  application: Application;
  job: Job;
  analysis: JobAnalysis | null;
}

export interface ApplicationDetail {
  application: Application;
  job: Job;
  analysis: JobAnalysis | null;
  resume: Resume | null;
  coverLetter: CoverLetter | null;
}

export interface DashboardCounts {
  jobsDiscovered: number;
  relevant: number;
  awaitingApproval: number;
  approved: number;
  applied: number;
  manualAction: number;
  waitingForUser: number;
  interviews: number;
  rejected: number;
  offers: number;
}

export interface Dashboard {
  today: DashboardCounts;
  total: DashboardCounts;
  latestJobs: JobListItem[];
  recentApplications: ApplicationListItem[];
  pendingActions: ApplicationListItem[];
  lastRun: AgentRun | null;
  agent: AgentStatus;
  sync: SyncStatus;
}

// ---- job-site profiles ---------------------------------------------------------------

export type PlatformSyncStatus = "NEVER_SYNCED" | "SYNCING" | "SYNCED" | "NEEDS_INPUT" | "MANUAL_ACTION_REQUIRED" | "FAILED";

export interface PlatformProfile {
  id: string;
  userId: string;
  platform: string;
  status: PlatformSyncStatus;
  autoSync: boolean;
  profileUrl: string;
  lastSyncedAt: string | null;
  lastAttemptAt: string | null;
  syncedHash: string | null;
  syncedProjectNames: string[];
  changes: string[];
  skipped: string[];
  message: string;
  pendingQuestions: PendingQuestion[];
  runId: string | null;
  /** Session of an update that stopped part-way; "Resume" continues it. */
  resumeSessionId: string | null;
  /** Set by "Update all now": runs as soon as the agent is free. */
  queuedAt: string | null;
  announcedHash: string | null;
  createdAt: string;
  updatedAt: string;
}

/** Mirrors `agent::profile_sync::PlatformProfileView`. */
export interface PlatformProfileView {
  profile: PlatformProfile;
  outOfDate: boolean;
}

export interface ProjectPick {
  projectId: string;
  selected: boolean;
  reason: string;
}

export interface PublishingPlan {
  id: string;
  userId: string;
  featuredProjectIds: string[];
  picks: ProjectPick[];
  confirmedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

// ---- notifications -------------------------------------------------------------------

export interface Notification {
  id: string;
  userId: string;
  level: EventLevel;
  /** JOB_HUNT_FINISHED, JOB_HUNT_FAILED, APPLICATION_NEEDS_INPUT, APPLICATION_MANUAL_ACTION,
   * APPLICATION_SUBMITTED, PROFILE_UPDATED, PROFILE_NEEDS_INPUT, PROFILE_MANUAL_ACTION, PROFILE_UPDATE_FAILED */
  kind: string;
  title: string;
  body: string;
  /** "applications" | "application" (with linkId) | "job-sites" | "jobs" | "agent" */
  linkPage: string;
  linkId: string | null;
  read: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface JobHuntOptions {
  discoverOnly?: boolean;
  sources?: string[];
}

// ---- mobile companion app (relay) -----------------------------------------------------

export interface RemoteStatus {
  configured: boolean;
  connected: boolean;
  relayUrl: string;
  accountEmail: string;
  deviceName: string;
  lastError: string | null;
}

export interface RemoteDevice {
  id: string;
  name: string;
  kind: "desktop" | "mobile";
  platform: string;
  createdAt: string;
  lastSeenAt: string | null;
  online: boolean;
}

export interface PairingCode {
  code: string;
  expiresAt: string;
}
