/// Mirrors the Rust `Job` / `JobAnalysis` structs (see
/// `crates/job-hunter-core/src/domain/job.rs` and `packages/types`). The
/// phone only ever reads these -- they arrive as part of an
/// `ApplicationListItem` / `ApplicationDetail`, never fetched on their own.
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
  });

  factory Job.fromJson(Map<String, dynamic> json) => Job(
        id: json['id'] as String? ?? '',
        title: json['title'] as String? ?? '',
        company: json['company'] as String? ?? '',
        location: json['location'] as String? ?? '',
        employmentType: json['employmentType'] as String?,
        remote: json['remote'] as String?,
        salary: json['salary'] as String?,
        seniority: json['seniority'] as String?,
        postedAt: json['postedAt'] as String?,
        source: json['source'] as String? ?? '',
        url: json['url'] as String? ?? '',
        description: json['description'] as String? ?? '',
        requirements: ((json['requirements'] as List?) ?? const []).cast<String>(),
        responsibilities: ((json['responsibilities'] as List?) ?? const []).cast<String>(),
        skills: ((json['skills'] as List?) ?? const []).cast<String>(),
        status: json['status'] as String? ?? 'DISCOVERED',
      );
}

class JobAnalysis {
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

  const JobAnalysis({
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
  });

  factory JobAnalysis.fromJson(Map<String, dynamic> json) => JobAnalysis(
        relevant: json['relevant'] == true,
        matchScore: (json['matchScore'] as num?)?.toInt() ?? 0,
        matchedSkills: ((json['matchedSkills'] as List?) ?? const []).cast<String>(),
        missingSkills: ((json['missingSkills'] as List?) ?? const []).cast<String>(),
        requiredExperienceMet: json['requiredExperienceMet'] == true,
        seniorityMatch: json['seniorityMatch'] == true,
        locationMatch: json['locationMatch'] == true,
        salaryAssessment: json['salaryAssessment'] as String? ?? '',
        concerns: ((json['concerns'] as List?) ?? const []).cast<String>(),
        importantKeywords: ((json['importantKeywords'] as List?) ?? const []).cast<String>(),
        summary: json['summary'] as String? ?? '',
      );
}
