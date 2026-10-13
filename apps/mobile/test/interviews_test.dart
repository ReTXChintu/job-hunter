import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/models/application.dart';
import 'package:job_hunter_mobile/models/envelope.dart';
import 'package:job_hunter_mobile/screens/home_screen.dart';
import 'package:job_hunter_mobile/state/applications_controller.dart';
import 'package:job_hunter_mobile/state/auth_controller.dart';

import 'support/fakes.dart';

ApplicationListItem _item(String status, List<Map<String, dynamic>> interviews) => ApplicationListItem.fromJson({
      'application': {'id': 'a-$status', 'status': status, 'interviews': interviews, 'updatedAt': '2026-10-13T00:00:00Z'},
      'job': {'id': 'j', 'title': 'Developer', 'company': 'Acme'},
    });

void main() {
  test('interviews parse with their mode label line', () {
    final app = Application.fromJson({
      'id': 'a1',
      'status': 'INTERVIEW',
      'interviews': [
        {'id': 'i1', 'scheduledAt': '2026-10-20T09:30:00Z', 'mode': 'video', 'round': 'HR screening', 'contact': 'Priya', 'notes': 'Meet link to follow', 'recordedAt': '2026-10-13T08:00:00Z'},
        {'id': 'i2'},
      ],
    });
    expect(app.interviews.length, 2);
    expect(app.interviews.first.mode, 'VIDEO');
    expect(app.interviews.first.line, 'Video call · HR screening · with Priya');
    expect(app.interviews.last.scheduledAt, isNull);
    expect(app.interviews.last.line, '');
  });

  test('upcoming interviews are the dated ones from a few hours ago on, soonest first', () {
    final now = DateTime(2026, 10, 13, 12);
    final items = [
      _item('INTERVIEW', [
        {'id': 'late', 'scheduledAt': '2026-10-20T09:00:00Z'},
        {'id': 'past', 'scheduledAt': '2026-10-01T09:00:00Z'},
        {'id': 'undated'},
      ]),
      _item('OFFER', [
        {'id': 'soon', 'scheduledAt': '2026-10-14T09:00:00Z'},
      ]),
      _item('APPLIED', [
        {'id': 'wrong-status', 'scheduledAt': '2026-10-15T09:00:00Z'},
      ]),
    ];
    expect(upcomingInterviews(items, now: now).map((e) => e.$2.id), ['soon', 'late']);
  });

  test('recording an interview sends the payload with the application id', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['record_interview'] = const RelayResponse(ok: true, data: {'id': 'a1', 'status': 'INTERVIEW'});
    fake.stubbed['list_applications'] = const RelayResponse(ok: true, data: []);
    final apps = ApplicationsController(fake);
    final resp = await apps.recordInterview('a1', {'scheduledAt': '2026-10-20T09:30:00Z', 'mode': 'VIDEO', 'round': 'HR', 'contact': '', 'notes': ''});
    expect(resp.ok, isTrue);
    final call = fake.calls.firstWhere((c) => c.type == 'record_interview');
    expect(call.payload['id'], 'a1');
    expect(call.payload['mode'], 'VIDEO');
    expect(call.payload['scheduledAt'], '2026-10-20T09:30:00Z');
  });
}
