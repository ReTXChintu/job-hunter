import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/logic/filters.dart';
import 'package:job_hunter_mobile/models/application.dart';
import 'package:job_hunter_mobile/models/job.dart';

ApplicationListItem app(String id, String status, {String title = 'Engineer', String company = 'Acme', String updatedAt = '2026-09-01T00:00:00Z'}) => ApplicationListItem.fromJson({
      'application': {'id': id, 'jobId': 'job-$id', 'status': status, 'updatedAt': updatedAt},
      'job': {'id': 'job-$id', 'title': title, 'company': company, 'location': 'Remote'},
    });

JobListItem job(String id, {String status = 'ANALYZED', bool? relevant, String? applicationStatus, String? applyEmail, String? discoveredAt, String title = 'Engineer'}) => JobListItem.fromJson({
      'job': {'id': id, 'title': title, 'company': 'Acme', 'status': status, 'applyEmail': applyEmail, 'discoveredAt': discoveredAt},
      'analysis': relevant == null ? null : {'jobId': id, 'relevant': relevant, 'matchScore': 70},
      'applicationStatus': applicationStatus,
    });

void main() {
  group('application buckets', () {
    test('every status maps to the right chip', () {
      expect(bucketOf('WAITING_FOR_USER'), ApplicationBucket.needsYou);
      expect(bucketOf('MANUAL_ACTION_REQUIRED'), ApplicationBucket.needsYou);
      expect(bucketOf('READY_FOR_REVIEW'), ApplicationBucket.pendingApproval);
      expect(bucketOf('APPROVED'), ApplicationBucket.inProgress);
      expect(bucketOf('APPLYING'), ApplicationBucket.inProgress);
      expect(bucketOf('APPLIED'), ApplicationBucket.applied);
      expect(bucketOf('INTERVIEW'), ApplicationBucket.interviewsOffers);
      expect(bucketOf('OFFER'), ApplicationBucket.interviewsOffers);
      expect(bucketOf('REJECTED'), ApplicationBucket.closed);
      expect(bucketOf('WITHDRAWN'), ApplicationBucket.closed);
      // Early statuses only show under "All".
      expect(bucketOf('DISCOVERED'), isNull);
      expect(bucketOf('SHORTLISTED'), isNull);
      expect(inBucket('SHORTLISTED', ApplicationBucket.all), isTrue);
      expect(inBucket('SHORTLISTED', ApplicationBucket.applied), isFalse);
    });

    test('counts per chip add up, with All counting everything', () {
      final items = [
        app('1', 'WAITING_FOR_USER'),
        app('2', 'MANUAL_ACTION_REQUIRED'),
        app('3', 'READY_FOR_REVIEW'),
        app('4', 'APPLIED'),
        app('5', 'APPLIED'),
        app('6', 'OFFER'),
        app('7', 'SHORTLISTED'),
      ];
      final counts = bucketCounts(items);
      expect(counts[ApplicationBucket.needsYou], 2);
      expect(counts[ApplicationBucket.pendingApproval], 1);
      expect(counts[ApplicationBucket.inProgress], 0);
      expect(counts[ApplicationBucket.applied], 2);
      expect(counts[ApplicationBucket.interviewsOffers], 1);
      expect(counts[ApplicationBucket.closed], 0);
      expect(counts[ApplicationBucket.all], 7);
    });

    test('filtering keeps the chip, matches the search, and sorts newest first', () {
      final items = [
        app('old', 'APPLIED', title: 'Backend Engineer', updatedAt: '2026-09-01T00:00:00Z'),
        app('new', 'APPLIED', title: 'Frontend Engineer', company: 'Globex', updatedAt: '2026-09-20T00:00:00Z'),
        app('other', 'READY_FOR_REVIEW', updatedAt: '2026-09-25T00:00:00Z'),
      ];
      expect(filterApplications(items, ApplicationBucket.applied, '').map((i) => i.application.id), ['new', 'old']);
      expect(filterApplications(items, ApplicationBucket.all, '').map((i) => i.application.id), ['other', 'new', 'old']);
      expect(filterApplications(items, ApplicationBucket.all, 'globex').map((i) => i.application.id), ['new']);
      expect(filterApplications(items, ApplicationBucket.applied, '  BACKEND ').map((i) => i.application.id), ['old']);
    });

    test('home counts come from the applications themselves', () {
      final c = HomeCounts.from([app('1', 'READY_FOR_REVIEW'), app('2', 'READY_FOR_REVIEW'), app('3', 'WAITING_FOR_USER'), app('4', 'MANUAL_ACTION_REQUIRED'), app('5', 'APPLIED'), app('6', 'INTERVIEW'), app('7', 'OFFER')]);
      expect([c.pendingApproval, c.needsInput, c.manualAction, c.applied, c.interviews, c.offers], [2, 1, 1, 1, 1, 1]);
    });

    test('waiting on you lists questions and manual steps before approvals', () {
      final list = waitingOnYou([app('a', 'READY_FOR_REVIEW', updatedAt: '2026-09-30T00:00:00Z'), app('b', 'APPLIED'), app('c', 'WAITING_FOR_USER')]);
      expect(list.map((i) => i.application.id), ['c', 'a']);
    });
  });

  group('job filters', () {
    final items = [
      job('s', status: 'SHORTLISTED', relevant: true, discoveredAt: '2026-09-10T00:00:00Z'),
      job('n', status: 'NOT_RELEVANT', relevant: false, discoveredAt: '2026-09-12T00:00:00Z'),
      job('a', status: 'READY_FOR_REVIEW', relevant: true, applicationStatus: 'READY_FOR_REVIEW', discoveredAt: '2026-09-11T00:00:00Z'),
      job('e', status: 'ANALYZED', applyEmail: 'hr@acme.test', discoveredAt: '2026-09-13T00:00:00Z', title: 'Hiring: Flutter dev'),
    ];

    test('each filter picks the right jobs', () {
      List<String> ids(JobFilter f) => filterJobs(items, f, '').map((i) => i.job.id).toList();
      expect(ids(JobFilter.all), ['e', 'n', 'a', 's']);
      expect(ids(JobFilter.shortlisted), ['s']);
      expect(ids(JobFilter.notRelevant), ['n']);
      expect(ids(JobFilter.hasApplication), ['a']);
      expect(ids(JobFilter.emailPosts), ['e']);
      expect(filterJobs(items, JobFilter.all, 'flutter').map((i) => i.job.id), ['e']);
      final counts = jobFilterCounts(items);
      expect(counts[JobFilter.all], 4);
      expect(counts[JobFilter.emailPosts], 1);
    });

    test('offline job list is rebuilt from the server collections', () {
      final built = buildJobItemsFromServer(
        {
          'documents': [
            {'id': 'j1', 'title': 'A', 'status': 'ANALYZED'},
            {'id': 'j2', 'title': 'B', 'status': 'READY_FOR_REVIEW', 'applicationId': 'app-2'},
            {'title': 'no id, skipped'},
          ],
        },
        {
          'documents': [
            {'jobId': 'j1', 'matchScore': 40, 'relevant': false, 'updatedAt': '2026-09-01T00:00:00Z'},
            {'jobId': 'j1', 'matchScore': 80, 'relevant': true, 'updatedAt': '2026-09-02T00:00:00Z'},
          ],
        },
        [
          {
            'application': {'id': 'app-2', 'jobId': 'j2', 'status': 'READY_FOR_REVIEW'},
            'job': {'id': 'j2'},
          },
        ],
      );
      expect(built.map((i) => i.job.id), ['j1', 'j2']);
      expect(built.first.analysis?.matchScore, 80, reason: 'the newest analysis wins');
      expect(built.last.applicationStatus, 'READY_FOR_REVIEW');
      expect(built.last.hasApplication, isTrue);
    });
  });
}
