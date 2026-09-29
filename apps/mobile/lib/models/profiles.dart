import 'application.dart';
import 'json.dart';

/// A job site's profile as the desktop keeps it (Rust `PlatformProfile`).
class PlatformProfile {
  final String platform;
  final String status; // NEVER_SYNCED | SYNCING | SYNCED | NEEDS_INPUT | MANUAL_ACTION_REQUIRED | FAILED
  final String message;
  final List<PendingQuestion> pendingQuestions;
  final String? resumeSessionId;
  final DateTime? lastSyncedAt;
  final DateTime? lastAttemptAt;
  final bool autoSync;
  final String profileUrl;
  final List<String> changes;
  final List<String> skipped;

  const PlatformProfile({
    required this.platform,
    required this.status,
    this.message = '',
    this.pendingQuestions = const [],
    this.resumeSessionId,
    this.lastSyncedAt,
    this.lastAttemptAt,
    this.autoSync = false,
    this.profileUrl = '',
    this.changes = const [],
    this.skipped = const [],
  });

  factory PlatformProfile.fromJson(Map<String, dynamic> json) => PlatformProfile(
        platform: str(json['platform']),
        status: str(json['status'], 'NEVER_SYNCED'),
        message: str(json['message']),
        pendingQuestions: parseQuestions(json['pendingQuestions']),
        resumeSessionId: optStr(json['resumeSessionId']),
        lastSyncedAt: dateOf(json['lastSyncedAt']),
        lastAttemptAt: dateOf(json['lastAttemptAt']),
        autoSync: boolOf(json['autoSync']),
        profileUrl: str(json['profileUrl']),
        changes: strList(json['changes']),
        skipped: strList(json['skipped']),
      );

  bool get isSyncing => status == 'SYNCING';
  bool get canResume => resumeSessionId != null;
}

/// `list_platform_profiles` rows: the profile plus whether it's behind the
/// desktop's current profile.
class PlatformProfileView {
  final PlatformProfile profile;
  final bool outOfDate;

  const PlatformProfileView({required this.profile, required this.outOfDate});

  factory PlatformProfileView.fromJson(Map<String, dynamic> json) => PlatformProfileView(
        profile: PlatformProfile.fromJson(mapOrEmpty(json['profile'])),
        outOfDate: boolOf(json['outOfDate']),
      );
}

const platformStatusLabels = <String, String>{
  'NEVER_SYNCED': 'Never updated',
  'SYNCING': 'Updating…',
  'SYNCED': 'Up to date',
  'NEEDS_INPUT': 'Needs your answers',
  'MANUAL_ACTION_REQUIRED': 'Needs a manual step',
  'FAILED': 'Update failed',
};

String platformStatusLabel(PlatformProfileView v) {
  if (v.profile.status == 'SYNCED' && v.outOfDate) return 'Out of date';
  return platformStatusLabels[v.profile.status] ?? v.profile.status;
}

/// A saved answer to an application question (Candidate > Additional
/// details on the desktop; Rust `AnswerRecord`).
class AnswerRecord {
  final String id;
  final String question;
  final String answer;
  final String category;
  final int timesUsed;
  final DateTime? updatedAt;

  const AnswerRecord({required this.id, required this.question, required this.answer, this.category = '', this.timesUsed = 0, this.updatedAt});

  factory AnswerRecord.fromJson(Map<String, dynamic> json) => AnswerRecord(
        id: str(json['id']),
        question: str(json['question']),
        answer: str(json['answer']),
        category: str(json['category']),
        timesUsed: intOf(json['timesUsed']),
        updatedAt: dateOf(json['updatedAt']),
      );
}
