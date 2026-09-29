import '../models/envelope.dart';

const agentStateLabels = <String, String>{
  'IDLE': 'Idle',
  'INITIALIZING': 'Starting up',
  'DISCOVERING': 'Searching job sites',
  'EXTRACTING': 'Reading job postings',
  'DEDUPLICATING': 'Removing duplicates',
  'ANALYZING': 'Matching jobs to your profile',
  'PREPARING_APPLICATIONS': 'Preparing applications',
  'WAITING_FOR_APPROVAL': 'Waiting for your approval',
  'APPLYING': 'Applying',
  'UPDATING_PROFILE': 'Updating a job-site profile',
  'CHECKING_INBOX': 'Checking Gmail for replies',
  'COMPLETED': 'Finished',
  'FAILED': 'Last run failed',
  'MANUAL_ACTION_REQUIRED': 'Needs a manual step',
  'WAITING_FOR_USER': 'Waiting for your input',
  'PAUSED': 'Paused',
  'STOPPING': 'Stopping',
};

String agentStateLabel(String state) => agentStateLabels[state] ?? state;

const runKindLabels = <String, String>{
  'JOB_HUNT': 'Job hunt',
  'APPLICATION': 'Application',
  'RESUME_GENERATION': 'Preparing an application',
  'ANALYSIS': 'Job analysis',
  'PROFILE_SYNC': 'Job-site profile update',
  'INBOX_CHECK': 'Gmail check',
};

String runKindLabel(String kind) => runKindLabels[kind] ?? kind;

/// States in which the agent is doing something and a new run would be
/// refused; the desktop is still the judge (it answers `AGENT_BUSY`).
const _busyStates = {'INITIALIZING', 'DISCOVERING', 'EXTRACTING', 'DEDUPLICATING', 'ANALYZING', 'PREPARING_APPLICATIONS', 'APPLYING', 'UPDATING_PROFILE', 'CHECKING_INBOX', 'PAUSED', 'STOPPING'};

bool agentBusy(String state) => _busyStates.contains(state);

const desktopOfflineMessage = 'Your desktop is offline — open Job Hunter on your computer.';
const notConnectedMessage = "You're not connected to your Job Hunter server. Check your internet connection.";

/// The message to show for a failed request.
String friendlyError(RelayResponse resp) => switch (resp.errorCode) {
      'DESKTOP_OFFLINE' => desktopOfflineMessage,
      'NOT_CONNECTED' => notConnectedMessage,
      'TIMEOUT' || 'DESKTOP_TIMEOUT' => 'Your desktop took too long to answer. It may still be working on it — pull to refresh in a moment.',
      'AGENT_BUSY' => resp.errorMessage?.isNotEmpty == true ? resp.errorMessage! : 'The agent is busy with another run. Try again when it finishes.',
      _ => resp.errorMessage?.isNotEmpty == true ? resp.errorMessage! : "That didn't go through.",
    };
