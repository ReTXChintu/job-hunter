import 'job.dart';

class ApplicationAnswer {
  final String question;
  final String answer;
  final String source;
  const ApplicationAnswer({required this.question, required this.answer, required this.source});

  factory ApplicationAnswer.fromJson(Map<String, dynamic> json) => ApplicationAnswer(
        question: json['question'] as String? ?? '',
        answer: json['answer'] as String? ?? '',
        source: json['source'] as String? ?? 'USER',
      );

  Map<String, dynamic> toJson() => {'question': question, 'answer': answer};
}

class PendingQuestion {
  final String id;
  final String question;
  final String fieldType;
  final List<String> options;
  final bool required;
  final String context;

  const PendingQuestion({required this.id, required this.question, required this.fieldType, required this.options, required this.required, required this.context});

  factory PendingQuestion.fromJson(Map<String, dynamic> json) => PendingQuestion(
        id: json['id'] as String? ?? '',
        question: json['question'] as String? ?? '',
        fieldType: json['fieldType'] as String? ?? 'text',
        options: ((json['options'] as List?) ?? const []).cast<String>(),
        required: json['required'] == true,
        context: json['context'] as String? ?? '',
      );
}

class StatusChange {
  final String status;
  final DateTime at;
  final String reason;
  const StatusChange({required this.status, required this.at, required this.reason});

  factory StatusChange.fromJson(Map<String, dynamic> json) => StatusChange(
        status: json['status'] as String? ?? '',
        at: DateTime.tryParse(json['at'] as String? ?? '') ?? DateTime.now(),
        reason: json['reason'] as String? ?? '',
      );
}

class ResumeValidation {
  final String status;
  final int keywordCoverage;
  final List<String> missingKeywords;
  final List<String> unsupportedClaims;

  const ResumeValidation({required this.status, required this.keywordCoverage, required this.missingKeywords, required this.unsupportedClaims});

  factory ResumeValidation.fromJson(Map<String, dynamic> json) => ResumeValidation(
        status: json['status'] as String? ?? 'REVISE',
        keywordCoverage: (json['keywordCoverage'] as num?)?.toInt() ?? 0,
        missingKeywords: ((json['missingKeywords'] as List?) ?? const []).cast<String>(),
        unsupportedClaims: ((json['unsupportedClaims'] as List?) ?? const []).cast<String>(),
      );
}

class ResumeInfo {
  final String id;
  final String label;
  final String? pdfPath;
  final String? docxPath;
  final ResumeValidation? validation;

  const ResumeInfo({required this.id, required this.label, this.pdfPath, this.docxPath, this.validation});

  static ResumeInfo? fromJsonOrNull(dynamic json) {
    if (json is! Map) return null;
    final map = json.cast<String, dynamic>();
    return ResumeInfo(
      id: map['id'] as String? ?? '',
      label: map['label'] as String? ?? '',
      pdfPath: map['pdfPath'] as String?,
      docxPath: map['docxPath'] as String?,
      validation: map['validation'] is Map ? ResumeValidation.fromJson((map['validation'] as Map).cast<String, dynamic>()) : null,
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
    if (json is! Map) return null;
    final map = json.cast<String, dynamic>();
    return CoverLetterInfo(id: map['id'] as String? ?? '', text: map['text'] as String? ?? '', pdfPath: map['pdfPath'] as String?, docxPath: map['docxPath'] as String?);
  }
}

class Application {
  final String id;
  final String jobId;
  final String status;
  final String applicationUrl;
  final String notes;
  final List<ApplicationAnswer> answers;
  final List<PendingQuestion> pendingQuestions;
  final List<StatusChange> statusHistory;
  final List<String> potentialIssues;
  final DateTime? approvedAt;
  final DateTime? appliedAt;
  final String? failureReason;
  final String? evidence;
  final bool manualCompleted;
  final DateTime updatedAt;

  const Application({
    required this.id,
    required this.jobId,
    required this.status,
    required this.applicationUrl,
    required this.notes,
    required this.answers,
    required this.pendingQuestions,
    required this.statusHistory,
    required this.potentialIssues,
    this.approvedAt,
    this.appliedAt,
    this.failureReason,
    this.evidence,
    required this.manualCompleted,
    required this.updatedAt,
  });

  factory Application.fromJson(Map<String, dynamic> json) => Application(
        id: json['id'] as String? ?? '',
        jobId: json['jobId'] as String? ?? '',
        status: json['status'] as String? ?? 'DISCOVERED',
        applicationUrl: json['applicationUrl'] as String? ?? '',
        notes: json['notes'] as String? ?? '',
        answers: ((json['answers'] as List?) ?? const []).map((e) => ApplicationAnswer.fromJson((e as Map).cast<String, dynamic>())).toList(),
        pendingQuestions: ((json['pendingQuestions'] as List?) ?? const []).map((e) => PendingQuestion.fromJson((e as Map).cast<String, dynamic>())).toList(),
        statusHistory: ((json['statusHistory'] as List?) ?? const []).map((e) => StatusChange.fromJson((e as Map).cast<String, dynamic>())).toList(),
        potentialIssues: ((json['potentialIssues'] as List?) ?? const []).cast<String>(),
        approvedAt: DateTime.tryParse(json['approvedAt'] as String? ?? ''),
        appliedAt: DateTime.tryParse(json['appliedAt'] as String? ?? ''),
        failureReason: json['failureReason'] as String?,
        evidence: json['evidence'] as String?,
        manualCompleted: json['manualCompleted'] == true,
        updatedAt: DateTime.tryParse(json['updatedAt'] as String? ?? '') ?? DateTime.now(),
      );
}

/// What `list_applications` returns: one row per application.
class ApplicationListItem {
  final Application application;
  final Job job;
  final JobAnalysis? analysis;

  const ApplicationListItem({required this.application, required this.job, this.analysis});

  factory ApplicationListItem.fromJson(Map<String, dynamic> json) => ApplicationListItem(
        application: Application.fromJson((json['application'] as Map).cast<String, dynamic>()),
        job: Job.fromJson((json['job'] as Map).cast<String, dynamic>()),
        analysis: json['analysis'] is Map ? JobAnalysis.fromJson((json['analysis'] as Map).cast<String, dynamic>()) : null,
      );
}

/// What `get_application` returns: the full review screen's worth of data.
class ApplicationDetail {
  final Application application;
  final Job job;
  final JobAnalysis? analysis;
  final ResumeInfo? resume;
  final CoverLetterInfo? coverLetter;

  const ApplicationDetail({required this.application, required this.job, this.analysis, this.resume, this.coverLetter});

  factory ApplicationDetail.fromJson(Map<String, dynamic> json) => ApplicationDetail(
        application: Application.fromJson((json['application'] as Map).cast<String, dynamic>()),
        job: Job.fromJson((json['job'] as Map).cast<String, dynamic>()),
        analysis: json['analysis'] is Map ? JobAnalysis.fromJson((json['analysis'] as Map).cast<String, dynamic>()) : null,
        resume: ResumeInfo.fromJsonOrNull(json['resume']),
        coverLetter: CoverLetterInfo.fromJsonOrNull(json['coverLetter']),
      );
}
