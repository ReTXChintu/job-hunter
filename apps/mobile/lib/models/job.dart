import 'application.dart';
import 'json.dart';

/// Mirrors the Rust `Job` / `JobAnalysis` structs (see
/// `crates/job-hunter-core/src/domain/job.rs` and `packages/types`), trimmed
/// to what the phone shows. Parsing never throws on a missing field.
class Job {
  final String id;
  final String title;
  final String company;
  final String location;
  final String? employmentType;
  final String? remote;
  final String? salary;
  final String? seniority;
  final String? postedAt;
  final String source;
  final String url;
  final String description;
  final List<String> requirements;
  final List<String> responsibilities;
  final List<String> skills;
  final String status;

  /// Set for postings that ask for the resume by email (sent from Gmail).
  final String? applyEmail;
  final String? contactName;
  final String? applicationId;
  final DateTime? discoveredAt;
  final DateTime? updatedAt;

  const Job({
    required this.id,
    required this.title,
    required this.company,
    required this.location,
    this.employmentType,
    this.remote,
    this.salary,
    this.seniority,
    this.postedAt,
    required this.source,
    required this.url,
    required this.description,
    required this.requirements,
    required this.responsibilities,
    required this.skills,
    required this.status,
    this.applyEmail,
    this.contactName,
    this.applicationId,
    this.discoveredAt,
    this.updatedAt,
  });

  factory Job.fromJson(Map<String, dynamic> json) => Job(
        id: str(json['id']),
        title: str(json['title']),
        company: str(json['company']),
        location: str(json['location']),
        employmentType: optStr(json['employmentType']),
        remote: optStr(json['remote']),
        salary: optStr(json['salary']),
        seniority: optStr(json['seniority']),
        postedAt: optStr(json['postedAt']),
        source: str(json['source']),
        url: str(json['url']),
        description: str(json['description']),
        requirements: strList(json['requirements']),
        responsibilities: strList(json['responsibilities']),
        skills: strList(json['skills']),
        status: str(json['status'], 'DISCOVERED'),
        applyEmail: optStr(json['applyEmail']),
        contactName: optStr(json['contactName']),
        applicationId: optStr(json['applicationId']),
        discoveredAt: dateOf(json['discoveredAt']) ?? dateOf(json['createdAt']),
        updatedAt: dateOf(json['updatedAt']),
      );

  bool get isEmailPost => applyEmail != null && applyEmail!.trim().isNotEmpty;

  /// "Acme · Bengaluru" without dangling separators.
  String get companyLine => [company, location].where((s) => s.trim().isNotEmpty).join(' · ');
}

class JobAnalysis {
  final String jobId;
  final bool relevant;
  final int matchScore;
  final List<String> matchedSkills;
  final List<String> missingSkills;
  final bool requiredExperienceMet;
  final bool seniorityMatch;
  final bool locationMatch;
  final String salaryAssessment;
  final List<String> concerns;
  final List<String> importantKeywords;
  final String summary;
  final DateTime? updatedAt;

  const JobAnalysis({
    this.jobId = '',
    required this.relevant,
    required this.matchScore,
    required this.matchedSkills,
    required this.missingSkills,
    required this.requiredExperienceMet,
    required this.seniorityMatch,
    required this.locationMatch,
    required this.salaryAssessment,
    required this.concerns,
    required this.importantKeywords,
    required this.summary,
    this.updatedAt,
  });

  factory JobAnalysis.fromJson(Map<String, dynamic> json) => JobAnalysis(
        jobId: str(json['jobId']),
        relevant: boolOf(json['relevant']),
        matchScore: intOf(json['matchScore']),
        matchedSkills: strList(json['matchedSkills']),
        missingSkills: strList(json['missingSkills']),
        requiredExperienceMet: boolOf(json['requiredExperienceMet']),
        seniorityMatch: boolOf(json['seniorityMatch']),
        locationMatch: boolOf(json['locationMatch']),
        salaryAssessment: str(json['salaryAssessment']),
        concerns: strList(json['concerns']),
        importantKeywords: strList(json['importantKeywords']),
        summary: str(json['summary']),
        updatedAt: dateOf(json['updatedAt']),
      );

  static JobAnalysis? fromJsonOrNull(dynamic json) {
    final map = asMap(json);
    return map == null ? null : JobAnalysis.fromJson(map);
  }
}

/// What `list_jobs` returns: a job, its analysis, and its application's
/// status if one exists.
class JobListItem {
  final Job job;
  final JobAnalysis? analysis;
  final String? applicationStatus;

  const JobListItem({required this.job, this.analysis, this.applicationStatus});

  factory JobListItem.fromJson(Map<String, dynamic> json) => JobListItem(
        job: Job.fromJson(mapOrEmpty(json['job'])),
        analysis: JobAnalysis.fromJsonOrNull(json['analysis']),
        applicationStatus: optStr(json['applicationStatus']),
      );

  bool get hasApplication => applicationStatus != null || job.applicationId != null;
}

/// What `get_job` returns.
class JobDetail {
  final Job job;
  final JobAnalysis? analysis;
  final Application? application;
  final ResumeInfo? resume;
  final CoverLetterInfo? coverLetter;

  const JobDetail({required this.job, this.analysis, this.application, this.resume, this.coverLetter});

  factory JobDetail.fromJson(Map<String, dynamic> json) => JobDetail(
        job: Job.fromJson(mapOrEmpty(json['job'])),
        analysis: JobAnalysis.fromJsonOrNull(json['analysis']),
        application: asMap(json['application']) == null ? null : Application.fromJson(asMap(json['application'])!),
        resume: ResumeInfo.fromJsonOrNull(json['resume']),
        coverLetter: CoverLetterInfo.fromJsonOrNull(json['coverLetter']),
      );
}

/// Builds the job list from the server's raw collections, for when the
/// desktop is offline: `/v1/data/jobs`, `/v1/data/job_analyses` (newest per
/// job wins) and `/v1/applications` (for each job's application status).
List<JobListItem> buildJobItemsFromServer(dynamic jobsResp, dynamic analysesResp, dynamic applicationsResp) {
  List<Map<String, dynamic>> docs(dynamic resp) => resp is Map ? mapList(resp['documents']) : mapList(resp);

  final analyses = <String, JobAnalysis>{};
  for (final raw in docs(analysesResp)) {
    final a = JobAnalysis.fromJson(raw);
    if (a.jobId.isEmpty) continue;
    final existing = analyses[a.jobId];
    if (existing == null || (a.updatedAt ?? epoch).isAfter(existing.updatedAt ?? epoch)) analyses[a.jobId] = a;
  }
  final statuses = <String, String>{};
  for (final raw in mapList(applicationsResp)) {
    final app = asMap(raw['application']);
    if (app == null) continue;
    final jobId = str(app['jobId']);
    if (jobId.isNotEmpty) statuses[jobId] = str(app['status']);
  }
  return docs(jobsResp).map(Job.fromJson).where((j) => j.id.isNotEmpty).map((j) => JobListItem(job: j, analysis: analyses[j.id], applicationStatus: statuses[j.id])).toList();
}
