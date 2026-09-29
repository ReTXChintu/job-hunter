import 'job.dart';
import 'json.dart';

class ApplicationAnswer {
  final String question;
  final String answer;
  final String source;
  const ApplicationAnswer({required this.question, required this.answer, this.source = 'USER'});

  factory ApplicationAnswer.fromJson(Map<String, dynamic> json) => ApplicationAnswer(
        question: str(json['question']),
        answer: str(json['answer']),
        source: str(json['source'], 'USER'),
      );

  /// The wire shape for `answer_application_questions` /
  /// `answer_platform_questions`: `{question, answer}`.
  Map<String, dynamic> toJson() => {'question': question, 'answer': answer};
}

class PendingQuestion {
  final String id;
  final String question;
  final String fieldType;
  final List<String> options;
  final bool required;
  final String context;

  const PendingQuestion({required this.id, required this.question, this.fieldType = 'text', this.options = const [], this.required = false, this.context = ''});

  factory PendingQuestion.fromJson(Map<String, dynamic> json) => PendingQuestion(
        id: str(json['id']),
        question: str(json['question']),
        fieldType: str(json['fieldType'], 'text').toLowerCase(),
        options: strList(json['options']),
        required: boolOf(json['required']),
        context: str(json['context']),
      );

  /// Stable key for form state, even if the desktop left `id` empty.
  String get key => id.isNotEmpty ? id : question;
}

List<PendingQuestion> parseQuestions(dynamic v) => mapList(v).map(PendingQuestion.fromJson).toList();

class StatusChange {
  final String status;
  final DateTime? at;
  final String reason;
  const StatusChange({required this.status, required this.at, required this.reason});

  factory StatusChange.fromJson(Map<String, dynamic> json) => StatusChange(
        status: str(json['status']),
        at: dateOf(json['at']),
        reason: str(json['reason']),
      );
}

/// An employer's email about an application, found in Gmail by the
/// desktop's inbox check (inbox or spam).
class EmailReply {
  final String id;
  final String from;
  final String subject;
  final DateTime? receivedAt;
  final String folder;
  final String kind;
  final String summary;

  const EmailReply({required this.id, required this.from, required this.subject, required this.receivedAt, required this.folder, required this.kind, required this.summary});

  factory EmailReply.fromJson(Map<String, dynamic> json) => EmailReply(
        id: str(json['id']),
        from: str(json['from']),
        subject: str(json['subject']),
        receivedAt: dateOf(json['receivedAt']),
        folder: str(json['folder'], 'INBOX'),
        kind: str(json['kind'], 'OTHER'),
        summary: str(json['summary']),
      );

  bool get inSpam => folder.toUpperCase() == 'SPAM';
}

const replyKindLabels = <String, String>{
  'INTERVIEW': 'Interview',
  'ASSESSMENT': 'Assessment',
  'QUESTION': 'Question',
  'OFFER': 'Offer',
  'REJECTION': 'Rejection',
  'ACKNOWLEDGEMENT': 'Acknowledgement',
  'OTHER': 'Other',
};

String replyKindLabel(String kind) => replyKindLabels[kind.toUpperCase()] ?? kind;

class ResumeValidation {
  final String status;
  final int keywordCoverage;
  final List<String> missingKeywords;
  final List<String> unsupportedClaims;

  const ResumeValidation({required this.status, required this.keywordCoverage, required this.missingKeywords, required this.unsupportedClaims});

  factory ResumeValidation.fromJson(Map<String, dynamic> json) => ResumeValidation(
        status: str(json['status'], 'REVISE'),
        keywordCoverage: intOf(json['keywordCoverage']),
        missingKeywords: strList(json['missingKeywords']),
        unsupportedClaims: strList(json['unsupportedClaims']),
      );
}

class ResumeInfo {
  final String id;
  final String label;
  final String? pdfPath;
  final String? docxPath;
  final ResumeValidation? validation;
  final DateTime? filesDeletedAt;

  const ResumeInfo({required this.id, required this.label, this.pdfPath, this.docxPath, this.validation, this.filesDeletedAt});

  static ResumeInfo? fromJsonOrNull(dynamic json) {
    final map = asMap(json);
    if (map == null) return null;
    return ResumeInfo(
      id: str(map['id']),
      label: str(map['label']),
      pdfPath: optStr(map['pdfPath']),
      docxPath: optStr(map['docxPath']),
      validation: asMap(map['validation']) == null ? null : ResumeValidation.fromJson(asMap(map['validation'])!),
      filesDeletedAt: dateOf(map['filesDeletedAt']),
    );
  }
}

class CoverLetterInfo {
  final String id;
  final String text;
  final String? pdfPath;
  final String? docxPath;

  const CoverLetterInfo({required this.id, required this.text, this.pdfPath, this.docxPath});

  static CoverLetterInfo? fromJsonOrNull(dynamic json) {
    final map = asMap(json);
    if (map == null) return null;
    return CoverLetterInfo(id: str(map['id']), text: str(map['text']), pdfPath: optStr(map['pdfPath']), docxPath: optStr(map['docxPath']));
  }
}

class Application {
  final String id;
  final String jobId;
  final String status;
  final String applicationUrl;
  final String source;
  final String notes;
  final List<ApplicationAnswer> answers;
  final List<PendingQuestion> pendingQuestions;
  final List<StatusChange> statusHistory;
  final List<String> potentialIssues;
  final List<EmailReply> replies;
  final DateTime? approvedAt;
  final DateTime? appliedAt;
  final String? failureReason;
  final String? evidence;
  final bool manualCompleted;
  final DateTime? createdAt;
  final DateTime updatedAt;

  const Application({
    required this.id,
    required this.jobId,
    required this.status,
    required this.applicationUrl,
    this.source = '',
    required this.notes,
    required this.answers,
    required this.pendingQuestions,
    required this.statusHistory,
    required this.potentialIssues,
    this.replies = const [],
    this.approvedAt,
    this.appliedAt,
    this.failureReason,
    this.evidence,
    required this.manualCompleted,
    this.createdAt,
    required this.updatedAt,
  });

  factory Application.fromJson(Map<String, dynamic> json) => Application(
        id: str(json['id']),
        jobId: str(json['jobId']),
        status: str(json['status'], 'DISCOVERED'),
        applicationUrl: str(json['applicationUrl']),
        source: str(json['source']),
        notes: str(json['notes']),
        answers: mapList(json['answers']).map(ApplicationAnswer.fromJson).toList(),
        pendingQuestions: parseQuestions(json['pendingQuestions']),
        statusHistory: mapList(json['statusHistory']).map(StatusChange.fromJson).toList(),
        potentialIssues: strList(json['potentialIssues']),
        replies: mapList(json['replies']).map(EmailReply.fromJson).toList(),
        approvedAt: dateOf(json['approvedAt']),
        appliedAt: dateOf(json['appliedAt']),
        failureReason: optStr(json['failureReason']),
        evidence: optStr(json['evidence']),
        manualCompleted: boolOf(json['manualCompleted']),
        createdAt: dateOf(json['createdAt']),
        updatedAt: dateOf(json['updatedAt']) ?? dateOf(json['createdAt']) ?? epoch,
      );
}

/// What `list_applications` (and the server's `/v1/applications`) returns:
/// one row per application.
class ApplicationListItem {
  final Application application;
  final Job job;
  final JobAnalysis? analysis;

  const ApplicationListItem({required this.application, required this.job, this.analysis});

  factory ApplicationListItem.fromJson(Map<String, dynamic> json) => ApplicationListItem(
        application: Application.fromJson(mapOrEmpty(json['application'])),
        job: Job.fromJson(mapOrEmpty(json['job'])),
        analysis: JobAnalysis.fromJsonOrNull(json['analysis']),
      );
}

List<ApplicationListItem> parseApplicationList(dynamic data) =>
    mapList(data).map(ApplicationListItem.fromJson).where((i) => i.application.id.isNotEmpty).toList();

/// What `get_application` returns: the full review screen's worth of data.
/// The server's `/v1/applications/:id` has the same shape without the
/// resume and cover letter.
class ApplicationDetail {
  final Application application;
  final Job job;
  final JobAnalysis? analysis;
  final ResumeInfo? resume;
  final CoverLetterInfo? coverLetter;

  const ApplicationDetail({required this.application, required this.job, this.analysis, this.resume, this.coverLetter});

  factory ApplicationDetail.fromJson(Map<String, dynamic> json) => ApplicationDetail(
        application: Application.fromJson(mapOrEmpty(json['application'])),
        job: Job.fromJson(mapOrEmpty(json['job'])),
        analysis: JobAnalysis.fromJsonOrNull(json['analysis']),
        resume: ResumeInfo.fromJsonOrNull(json['resume']),
        coverLetter: CoverLetterInfo.fromJsonOrNull(json['coverLetter']),
      );
}
