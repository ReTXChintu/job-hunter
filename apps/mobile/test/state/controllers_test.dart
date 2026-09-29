import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:job_hunter_mobile/logic/labels.dart';
import 'package:job_hunter_mobile/models/application.dart';
import 'package:job_hunter_mobile/models/envelope.dart';
import 'package:job_hunter_mobile/state/agent_controller.dart';
import 'package:job_hunter_mobile/state/applications_controller.dart';
import 'package:job_hunter_mobile/state/auth_controller.dart';
import 'package:job_hunter_mobile/state/data_source.dart';
import 'package:job_hunter_mobile/state/jobs_controller.dart';
import 'package:job_hunter_mobile/state/profiles_controller.dart';

import '../support/fakes.dart';

const _offline = RelayResponse(ok: false, errorCode: 'DESKTOP_OFFLINE', errorMessage: 'The desktop is not connected.');

void main() {
  setUp(() => SharedPreferences.setMockInitialValues({}));

  test('applications fall back to the server when the desktop is offline', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_applications'] = _offline;
    fake.serverData['/v1/applications'] = [
      {
        'application': {'id': 'a1', 'status': 'APPLIED', 'replies': [{'subject': 'Thanks'}]},
        'job': {'id': 'j1', 'title': 'Engineer'},
      },
    ];
    final c = ApplicationsController(fake);
    await c.refresh();
    expect(c.source, DataSource.server);
    expect(c.items.single.application.replies.single.subject, 'Thanks');
    expect(c.error, isNull);
    expect(fake.calls.where((x) => x.type == 'list_dashboard'), isEmpty, reason: 'no dashboard request while offline');
  });

  test('answering questions sends {question, answer} pairs to the desktop', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['answer_application_questions'] = const RelayResponse(ok: true, data: {});
    final c = ApplicationsController(fake);
    final resp = await c.answerQuestions('a1', const [ApplicationAnswer(question: 'Notice period?', answer: '30 days')]);
    expect(resp.ok, isTrue);
    expect(fake.calls.first.payload, {
      'id': 'a1',
      'answers': [
        {'question': 'Notice period?', 'answer': '30 days'},
      ],
    });
  });

  test('status tracking sends set_application_status with the status and note', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['set_application_status'] = const RelayResponse(ok: true, data: {});
    await ApplicationsController(fake).setStatus('a1', 'INTERVIEW', note: 'Call on Monday');
    expect(fake.calls.first.type, 'set_application_status');
    expect(fake.calls.first.payload, {'id': 'a1', 'status': 'INTERVIEW', 'note': 'Call on Monday'});
  });

  test('jobs load from the desktop, else from the server collections', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_jobs'] = const RelayResponse(ok: true, data: [
      {'job': {'id': 'j1', 'title': 'A', 'applyEmail': 'hr@x.test'}, 'analysis': null, 'applicationStatus': null},
    ]);
    final jobs = JobsController(fake);
    await jobs.refresh();
    expect(jobs.source, DataSource.desktop);
    expect(jobs.items.single.job.isEmailPost, isTrue);

    fake.stubbed['list_jobs'] = _offline;
    fake.serverData['/v1/data/jobs'] = {
      'documents': [
        {'id': 'j2', 'title': 'B'},
      ],
    };
    await jobs.refresh();
    expect(jobs.source, DataSource.server);
    expect(jobs.items.single.job.id, 'j2');

    fake.serverData.clear();
    await jobs.refresh();
    expect(jobs.error, desktopOfflineMessage);
    expect(jobs.items.single.job.id, 'j2', reason: 'keeps the last list on failure');
  });

  test('job actions use the desktop request types', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['generate_resume'] = const RelayResponse(ok: true, data: {'runId': 'r'});
    fake.stubbed['reject_job'] = _offline;
    final jobs = JobsController(fake);
    expect((await jobs.prepareApplication('j1')).ok, isTrue);
    expect(fake.calls.first.payload, {'jobId': 'j1'});
    final rejected = await jobs.rejectJob('j1');
    expect(rejected.ok, isFalse);
    expect(friendlyError(rejected), desktopOfflineMessage);
  });

  test('quick actions: hiring posts is a LinkedIn Posts job hunt; offline explains itself', () async {
    final fake = FakeConnectionController(AuthController());
    final agent = AgentController(fake);
    expect(agent.unavailableReason, notConnectedMessage);

    fake.stubbed['start_job_hunt'] = const RelayResponse(ok: true, data: {'runId': 'r1'});
    fake.stubbed['get_agent_status'] = const RelayResponse(ok: true, data: {'state': 'DISCOVERING', 'paused': false, 'runKind': 'JOB_HUNT'});
    final resp = await agent.findHiringPosts();
    expect(resp.ok, isTrue);
    expect(fake.calls.first.type, 'start_job_hunt');
    expect(fake.calls.first.payload, {
      'sources': ['LinkedIn Posts'],
    });
    expect(agent.status?.state, 'DISCOVERING', reason: 'refreshes the agent state after starting');
    expect(agent.busy, isTrue);

    fake.stubbed['check_inbox'] = const RelayResponse(ok: false, errorCode: 'VALIDATION', errorMessage: 'Nothing has been sent yet.');
    expect(friendlyError(await agent.checkInbox()), 'Nothing has been sent yet.');
  });

  test('job sites and saved answers', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_platform_profiles'] = const RelayResponse(ok: true, data: [
      {'profile': {'platform': 'LinkedIn', 'status': 'NEEDS_INPUT'}, 'outOfDate': false},
    ]);
    fake.stubbed['sync_platform_profile'] = const RelayResponse(ok: true, data: {'runId': 'r'});
    final profiles = ProfilesController(fake);
    await profiles.refresh();
    expect(profiles.items.single.profile.status, 'NEEDS_INPUT');
    await profiles.sync('LinkedIn', resume: true);
    expect(fake.calls.firstWhere((c) => c.type == 'sync_platform_profile').payload, {'platform': 'LinkedIn', 'resume': true});

    fake.stubbed['list_answers'] = _offline;
    fake.serverData['/v1/data/application_answers'] = {
      'documents': [
        {'id': 'b', 'question': 'Notice period?', 'answer': '30 days'},
        {'id': 'a', 'question': 'current CTC?', 'answer': '12 LPA'},
      ],
    };
    final answers = AnswersController(fake);
    await answers.refresh();
    expect(answers.source, DataSource.server);
    expect(answers.items.map((a) => a.id), ['a', 'b'], reason: 'sorted by question, case-insensitively');

    fake.stubbed['save_answer'] = const RelayResponse(ok: true, data: {});
    await answers.save(question: 'Q', answer: 'A');
    expect(fake.calls.firstWhere((c) => c.type == 'save_answer').payload, {'question': 'Q', 'answer': 'A'}, reason: 'no id means a new answer');
  });
}
