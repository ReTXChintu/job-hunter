import '../models/application.dart';
import '../models/job.dart';

/// The Applications tab's filter chips. Each application falls into at most
/// one bucket by its status; early statuses (discovered/analyzed/
/// shortlisted) only show under "All".
enum ApplicationBucket { all, needsYou, pendingApproval, inProgress, applied, interviewsOffers, closed }

const _bucketStatuses = <ApplicationBucket, Set<String>>{
  ApplicationBucket.needsYou: {'WAITING_FOR_USER', 'MANUAL_ACTION_REQUIRED'},
  ApplicationBucket.pendingApproval: {'READY_FOR_REVIEW'},
  ApplicationBucket.inProgress: {'APPROVED', 'APPLYING'},
  ApplicationBucket.applied: {'APPLIED'},
  ApplicationBucket.interviewsOffers: {'INTERVIEW', 'OFFER'},
  ApplicationBucket.closed: {'REJECTED', 'WITHDRAWN'},
};

extension ApplicationBucketLabel on ApplicationBucket {
  String get label => switch (this) {
        ApplicationBucket.needsYou => 'Needs you',
        ApplicationBucket.pendingApproval => 'Pending approval',
        ApplicationBucket.inProgress => 'In progress',
        ApplicationBucket.applied => 'Applied',
        ApplicationBucket.interviewsOffers => 'Interviews & offers',
        ApplicationBucket.closed => 'Rejected/withdrawn',
        ApplicationBucket.all => 'All',
      };

  String get emptyMessage => switch (this) {
        ApplicationBucket.needsYou => 'Nothing needs your input right now.',
        ApplicationBucket.pendingApproval => 'No applications are waiting for your approval.',
        ApplicationBucket.inProgress => 'Nothing is being applied to right now.',
        ApplicationBucket.applied => 'No applications have been sent yet.',
        ApplicationBucket.interviewsOffers => 'No interviews or offers yet.',
        ApplicationBucket.closed => 'Nothing rejected or withdrawn.',
        ApplicationBucket.all => 'Applications your desktop agent prepares will show up here.',
      };
}

/// The bucket a status belongs to, or null for statuses only "All" shows.
ApplicationBucket? bucketOf(String status) {
  for (final entry in _bucketStatuses.entries) {
    if (entry.value.contains(status)) return entry.key;
  }
  return null;
}

bool inBucket(String status, ApplicationBucket bucket) => bucket == ApplicationBucket.all || bucketOf(status) == bucket;

/// How many applications each chip would show.
Map<ApplicationBucket, int> bucketCounts(Iterable<ApplicationListItem> items) {
  final counts = {for (final b in ApplicationBucket.values) b: 0};
  for (final item in items) {
    counts[ApplicationBucket.all] = counts[ApplicationBucket.all]! + 1;
    final b = bucketOf(item.application.status);
    if (b != null) counts[b] = counts[b]! + 1;
  }
  return counts;
}

bool _matches(String query, Iterable<String> fields) {
  final q = query.trim().toLowerCase();
  if (q.isEmpty) return true;
  return fields.any((f) => f.toLowerCase().contains(q));
}

/// The rows for one chip, search text and place, most recently updated first.
List<ApplicationListItem> filterApplications(Iterable<ApplicationListItem> items, ApplicationBucket bucket, String query, {String location = ''}) {
  final list = items
      .where((i) => inBucket(i.application.status, bucket))
      .where((i) => matchesLocation(i.job, location))
      .where((i) => _matches(query, [i.job.title, i.job.company, i.job.location, i.job.source]))
      .toList();
  list.sort((a, b) => b.application.updatedAt.compareTo(a.application.updatedAt));
  return list;
}

/// Counts for the Home tab, computed from the applications themselves so
/// they always agree with the Applications tab's chips.
class HomeCounts {
  final int pendingApproval;
  final int needsInput;
  final int manualAction;
  final int applied;
  final int interviews;
  final int offers;

  const HomeCounts({required this.pendingApproval, required this.needsInput, required this.manualAction, required this.applied, required this.interviews, required this.offers});

  factory HomeCounts.from(Iterable<ApplicationListItem> items) {
    int count(String status) => items.where((i) => i.application.status == status).length;
    return HomeCounts(
      pendingApproval: count('READY_FOR_REVIEW'),
      needsInput: count('WAITING_FOR_USER'),
      manualAction: count('MANUAL_ACTION_REQUIRED'),
      applied: count('APPLIED'),
      interviews: count('INTERVIEW'),
      offers: count('OFFER'),
    );
  }
}

/// The applications that are waiting on the user, most urgent first:
/// questions and manual steps before approvals.
List<ApplicationListItem> waitingOnYou(Iterable<ApplicationListItem> items) {
  final needs = filterApplications(items, ApplicationBucket.needsYou, '');
  final approvals = filterApplications(items, ApplicationBucket.pendingApproval, '');
  return [...needs, ...approvals];
}

// ---- jobs --------------------------------------------------------------------

enum JobFilter { all, shortlisted, notRelevant, hasApplication, emailPosts }

extension JobFilterLabel on JobFilter {
  String get label => switch (this) {
        JobFilter.all => 'All',
        JobFilter.shortlisted => 'Shortlisted',
        JobFilter.notRelevant => 'Not relevant',
        JobFilter.hasApplication => 'Has application',
        JobFilter.emailPosts => 'Email posts',
      };
}

bool jobMatchesFilter(JobListItem item, JobFilter filter) => switch (filter) {
      JobFilter.all => true,
      JobFilter.shortlisted => item.job.status == 'SHORTLISTED',
      JobFilter.notRelevant => item.job.status == 'NOT_RELEVANT' || item.analysis?.relevant == false,
      JobFilter.hasApplication => item.hasApplication,
      JobFilter.emailPosts => item.job.isEmailPost,
    };

Map<JobFilter, int> jobFilterCounts(Iterable<JobListItem> items) => {for (final f in JobFilter.values) f: items.where((i) => jobMatchesFilter(i, f)).length};

/// Newest discovered first.
List<JobListItem> filterJobs(Iterable<JobListItem> items, JobFilter filter, String query, {String location = ''}) {
  final list = items.where((i) => jobMatchesFilter(i, filter)).where((i) => matchesLocation(i.job, location)).where((i) => _matches(query, [i.job.title, i.job.company, i.job.location, i.job.source])).toList();
  DateTime when(JobListItem i) => i.job.discoveredAt ?? i.job.updatedAt ?? DateTime.fromMillisecondsSinceEpoch(0);
  list.sort((a, b) => when(b).compareTo(when(a)));
  return list;
}

// ---- location filter: searches what's already on the phone, never the web ----

class LocationOption {
  final String label;
  final int count;
  const LocationOption(this.label, this.count);
}

const _locationNoise = {'', 'india', 'in', 'hybrid', 'onsite', 'on-site', 'on site', 'wfh', 'work from home', 'full time', 'full-time', 'part time', 'part-time', 'contract', 'permanent', 'anywhere'};
final _remoteWords = RegExp(r'remote|work from home|wfh', caseSensitive: false);
final _locationSplit = RegExp(r'[,/|•·()\[\]\-–—]|\band\b|\bor\b', caseSensitive: false);

bool _isRemote(Job job) => _remoteWords.hasMatch(job.location) || RegExp('remote', caseSensitive: false).hasMatch(job.remote ?? '');

/// The places in the jobs' locations with how many jobs each ("Mumbai,
/// Maharashtra" and "Mumbai (Hybrid)" both count for Mumbai), most common
/// first. "Remote" is added for remote jobs.
List<LocationOption> locationOptions(Iterable<Job> jobs) {
  final counts = <String, (String, int)>{};
  void bump(String label) {
    final key = label.toLowerCase();
    final existing = counts[key];
    counts[key] = (existing?.$1 ?? label, (existing?.$2 ?? 0) + 1);
  }

  for (final job in jobs) {
    final seen = <String>{};
    for (final raw in job.location.split(_locationSplit)) {
      final part = raw.replaceAll(RegExp(r'\s+'), ' ').trim();
      if (part.length < 2 || int.tryParse(part) != null || _locationNoise.contains(part.toLowerCase()) || _remoteWords.hasMatch(part)) continue;
      if (seen.add(part.toLowerCase())) bump(part);
    }
    if (_isRemote(job)) bump('Remote');
  }
  final list = counts.values.map((e) => LocationOption(e.$1, e.$2)).toList();
  list.sort((a, b) => b.count != a.count ? b.count.compareTo(a.count) : a.label.compareTo(b.label));
  return list;
}

/// Whether a job is in the wanted place ("mumbai" matches "Mumbai,
/// Maharashtra"; "remote" matches remote jobs; empty matches everything).
bool matchesLocation(Job job, String wanted) {
  final w = wanted.trim().toLowerCase();
  if (w.isEmpty) return true;
  if (w == 'remote') return _isRemote(job);
  return job.location.toLowerCase().contains(w);
}
