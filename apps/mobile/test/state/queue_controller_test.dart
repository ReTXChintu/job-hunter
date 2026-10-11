import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/models/dashboard.dart';
import 'package:job_hunter_mobile/models/envelope.dart';
import 'package:job_hunter_mobile/state/auth_controller.dart';
import 'package:job_hunter_mobile/state/queue_controller.dart';

import '../support/fakes.dart';

void main() {
  test('reads the queue when the desktop reports a different queued count', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_queue'] = const RelayResponse(ok: true, data: [
      {'id': 'q1', 'title': 'Apply: Developer at Acme', 'task': {'kind': 'APPLICATION', 'applicationId': 'a1'}, 'requestedAt': '2026-10-11T10:00:00Z'},
      {'id': 'q2', 'title': 'Search only: Naukri', 'task': {'kind': 'JOB_HUNT', 'discoverOnly': true}},
    ]);
    final queue = QueueController(fake);
    expect(queue.items, isEmpty);

    fake.agentStatus = const AgentStatusLite(state: 'APPLYING', paused: false, queued: 2);
    fake.notifyListeners();
    await Future<void>.delayed(Duration.zero);
    expect(queue.items.map((i) => i.id), ['q1', 'q2']);
    expect(queue.items.first.kind, 'APPLICATION');
    expect(queue.count, 2);
    expect(fake.calls.where((c) => c.type == 'list_queue').length, 1);

    // The same count again doesn't re-read; a different one does.
    fake.notifyListeners();
    await Future<void>.delayed(Duration.zero);
    expect(fake.calls.where((c) => c.type == 'list_queue').length, 1);
    fake.agentStatus = const AgentStatusLite(state: 'APPLYING', paused: false, queued: 1);
    fake.notifyListeners();
    await Future<void>.delayed(Duration.zero);
    expect(fake.calls.where((c) => c.type == 'list_queue').length, 2);

    // Desktop offline: nothing to show, no error.
    fake.agentStatus = null;
    fake.notifyListeners();
    expect(queue.items, isEmpty);
    expect(queue.error, isNull);
  });

  test('remove and clear go to the desktop and re-read the queue', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_queue'] = const RelayResponse(ok: true, data: []);
    fake.stubbed['remove_queued_task'] = const RelayResponse(ok: true, data: {'removed': true});
    fake.stubbed['clear_queue'] = const RelayResponse(ok: true, data: {'removed': 2});
    final queue = QueueController(fake);

    expect((await queue.remove('q1')).ok, isTrue);
    expect(fake.calls.last.type, 'list_queue');
    expect(fake.calls.where((c) => c.type == 'remove_queued_task').single.payload, {'id': 'q1'});
    expect((await queue.clear()).ok, isTrue);
    expect(fake.calls.where((c) => c.type == 'clear_queue').length, 1);
  });

  test('an offline desktop leaves the queue empty without an error', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_queue'] = const RelayResponse(ok: false, errorCode: 'DESKTOP_OFFLINE', errorMessage: 'offline');
    final queue = QueueController(fake);
    await queue.refresh();
    expect(queue.items, isEmpty);
    expect(queue.error, isNull);
  });
}
