import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/models/application.dart';
import 'package:job_hunter_mobile/models/dashboard.dart';
import 'package:job_hunter_mobile/models/job.dart';
import 'package:job_hunter_mobile/models/profiles.dart';

void main() {
  test('an application with replies, questions and history parses fully', () {
    final a = Application.fromJson({
      'id': 'app-1',
      'jobId': 'job-1',
      'status': 'WAITING_FOR_USER',
      'pendingQuestions': [
        {'id': 'q1', 'question': 'Notice period?', 'fieldType': 'SELECT', 'options': ['Immediate', '30 days'], 'required': true, 'context': 'Screening'},
        {'question': 'Anything else?'},
      ],
      'replies': [
        {'id': 'r1', 'from': 'hr@acme.test', 'subject': 'Interview', 'receivedAt': '2026-09-28T09:00:00Z', 'folder': 'SPAM', 'kind': 'INTERVIEW', 'summary': 'They want to talk.'},
      ],
      'statusHistory': [
        {'status': 'READY_FOR_REVIEW', 'at': '2026-09-27T09:00:00Z', 'reason': ''},
      ],
      'answers': [
        {'question': 'CTC', 'answer': '12 LPA', 'source': 'KNOWN'},
      ],
      'updatedAt': '2026-09-28T10:00:00Z',
    });
    expect(a.pendingQuestions, hasLength(2));
    expect(a.pendingQuestions.first.fieldType, 'select');
    expect(a.pendingQuestions.first.required, isTrue);
    expect(a.pendingQuestions.first.options, ['Immediate', '30 days']);
    expect(a.pendingQuestions.last.fieldType, 'text');
    expect(a.pendingQuestions.last.key, 'Anything else?', reason: 'falls back to the question text as a form key');
    expect(a.replies.single.inSpam, isTrue);
    expect(replyKindLabel(a.replies.single.kind), 'Interview');
    expect(a.statusHistory.single.at, DateTime.utc(2026, 9, 27, 9));
    expect(a.answers.single.source, 'KNOWN');
    expect(a.updatedAt, DateTime.utc(2026, 9, 28, 10));
  });

  test('missing or mistyped fields fall back to defaults instead of throwing', () {
    final a = Application.fromJson({'id': 'x', 'replies': null, 'pendingQuestions': 'nope', 'answers': [1, 'two', null], 'potentialIssues': [null, 'Gap']});
    expect(a.status, 'DISCOVERED');
    expect(a.replies, isEmpty);
    expect(a.pendingQuestions, isEmpty);
    expect(a.answers, isEmpty);
    expect(a.potentialIssues, ['Gap']);
    expect(a.failureReason, isNull);

    final item = ApplicationListItem.fromJson({'application': {'id': 'x'}});
    expect(item.job.title, '');
    expect(item.analysis, isNull);

    expect(parseApplicationList([{'application': {'id': 'ok'}}, 'garbage', {'application': {}}]).map((i) => i.application.id), ['ok']);
  });

  test('jobs carry apply-by-email details', () {
    final j = Job.fromJson({'id': 'j', 'title': 'Hiring', 'company': 'Acme', 'location': '', 'applyEmail': 'hr@acme.test', 'contactName': 'Priya', 'matchScore': 'ignored', 'skills': ['Dart']});
    expect(j.isEmailPost, isTrue);
    expect(j.contactName, 'Priya');
    expect(j.companyLine, 'Acme');
    expect(Job.fromJson({'id': 'k', 'applyEmail': ''}).isEmailPost, isFalse);

    final item = JobListItem.fromJson({'job': {'id': 'j', 'applicationId': 'app-1'}, 'analysis': {'matchScore': 91.6, 'relevant': true}, 'applicationStatus': null});
    expect(item.analysis!.matchScore, 91);
    expect(item.hasApplication, isTrue);

    final detail = JobDetail.fromJson({'job': {'id': 'j'}, 'application': {'id': 'a', 'status': 'APPLIED'}, 'resume': {'id': 'r', 'label': 'v2'}});
    expect(detail.application?.status, 'APPLIED');
    expect(detail.resume?.label, 'v2');
  });

  test('job-site profiles and saved answers', () {
    final v = PlatformProfileView.fromJson({
      'profile': {
        'platform': 'LinkedIn',
        'status': 'SYNCED',
        'lastSyncedAt': '2026-09-20T00:00:00Z',
        'resumeSessionId': 'sess-1',
        'pendingQuestions': [{'id': 'q', 'question': 'Headline?', 'fieldType': 'textarea'}],
        'changes': ['Updated headline'],
      },
      'outOfDate': true,
    });
    expect(v.profile.platform, 'LinkedIn');
    expect(v.profile.canResume, isTrue);
    expect(v.profile.pendingQuestions.single.fieldType, 'textarea');
    expect(platformStatusLabel(v), 'Out of date');
    expect(platformStatusLabel(PlatformProfileView.fromJson({'profile': {'platform': 'Naukri', 'status': 'NEEDS_INPUT'}})), 'Needs your answers');

    final a = AnswerRecord.fromJson({'id': 'a1', 'question': 'Current CTC?', 'answer': '12 LPA', 'timesUsed': 3});
    expect(a.timesUsed, 3);
  });

  test('agent status includes progress when the desktop sends it', () {
    final s = AgentStatusLite.fromJson({'state': 'APPLYING', 'paused': false, 'runKind': 'APPLICATION', 'progress': 0.5});
    expect(s.progress, 0.5);
    expect(AgentStatusLite.fromJson(const {}).state, 'IDLE');
  });
}
