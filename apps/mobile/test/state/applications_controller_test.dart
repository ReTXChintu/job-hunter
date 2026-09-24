import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:job_hunter_mobile/models/envelope.dart';
import 'package:job_hunter_mobile/state/applications_controller.dart';
import 'package:job_hunter_mobile/state/auth_controller.dart';
import 'package:job_hunter_mobile/state/connection_controller.dart';

/// A `ConnectionController` stand-in that records every `request()` call
/// and returns whatever the test stubs, so `ApplicationsController` can be
/// exercised without a real relay or WebSocket. It never touches the
/// network -- `ConnectionController` itself is a no-op here because the
/// underlying `AuthController` is never marked signed in.
class FakeConnectionController extends ConnectionController {
  FakeConnectionController(super.auth);

  final List<({String type, Map<String, dynamic> payload})> calls = [];
  final Map<String, RelayResponse> stubbed = {};

  @override
  Future<RelayResponse> request(String type, [Map<String, dynamic> payload = const {}]) async {
    calls.add((type: type, payload: payload));
    return stubbed[type] ?? const RelayResponse(ok: false, errorCode: 'NOT_STUBBED', errorMessage: 'This test did not stub a response for this request type.');
  }
}

Map<String, dynamic> _applicationJson({String id = 'app-1', String status = 'READY_FOR_REVIEW'}) => {
      'id': id,
      'jobId': 'job-1',
      'status': status,
      'applicationUrl': '',
      'notes': '',
      'answers': [],
      'pendingQuestions': [],
      'statusHistory': [],
      'potentialIssues': [],
      'approvedAt': null,
      'appliedAt': null,
      'failureReason': null,
      'evidence': null,
      'manualCompleted': false,
      'updatedAt': '2026-01-01T00:00:00.000Z',
    };

Map<String, dynamic> _jobJson() => {
      'id': 'job-1',
      'title': 'Staff Engineer',
      'company': 'Acme',
      'location': 'Remote',
      'employmentType': 'full_time',
      'remote': 'remote',
      'salary': null,
      'seniority': 'staff',
      'postedAt': null,
      'source': 'greenhouse',
      'url': 'https://example.com/job',
      'description': '',
      'requirements': [],
      'responsibilities': [],
      'skills': [],
      'status': 'ANALYZED',
    };

Map<String, dynamic> _dashboardJson() => {
      'today': {'jobsDiscovered': 1, 'relevant': 1, 'awaitingApproval': 1, 'applied': 0, 'manualAction': 0, 'waitingForUser': 0, 'interviews': 0, 'rejected': 0, 'offers': 0},
      'total': {'jobsDiscovered': 10, 'relevant': 5, 'awaitingApproval': 1, 'applied': 3, 'manualAction': 0, 'waitingForUser': 0, 'interviews': 1, 'rejected': 1, 'offers': 0},
      'agent': {'state': 'IDLE', 'paused': false, 'runKind': null},
    };

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  test('refresh() loads applications and the dashboard from list_applications/list_dashboard', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_applications'] = RelayResponse(ok: true, data: [
      {'application': _applicationJson(), 'job': _jobJson()},
    ]);
    fake.stubbed['list_dashboard'] = RelayResponse(ok: true, data: _dashboardJson());

    final controller = ApplicationsController(fake);
    await controller.refresh();

    expect(controller.items, hasLength(1));
    expect(controller.items.single.job.title, 'Staff Engineer');
    expect(controller.items.single.application.status, 'READY_FOR_REVIEW');
    expect(controller.isFromCache, isFalse);
    expect(controller.dashboard?.today.awaitingApproval, 1);
    expect(controller.error, isNull);
    expect(controller.loading, isFalse);
  });

  test('refresh() surfaces the relay error and keeps loading=false on failure', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['list_applications'] = const RelayResponse(ok: false, errorCode: 'DESKTOP_OFFLINE', errorMessage: 'Desktop is not connected.');

    final controller = ApplicationsController(fake);
    await controller.refresh();

    expect(controller.items, isEmpty);
    expect(controller.error, 'Desktop is not connected.');
    expect(controller.loading, isFalse);
  });

  test('approve() dispatches approve_application with the application id and refreshes on success', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['approve_application'] = const RelayResponse(ok: true, data: {'status': 'APPROVED'});
    fake.stubbed['list_applications'] = const RelayResponse(ok: true, data: []);
    fake.stubbed['list_dashboard'] = RelayResponse(ok: true, data: _dashboardJson());

    final controller = ApplicationsController(fake);
    final resp = await controller.approve('app-1');

    expect(resp.ok, isTrue);
    expect(fake.calls.first.type, 'approve_application');
    expect(fake.calls.first.payload, {'id': 'app-1'});
    // approve() triggers an unawaited refresh(); give it a turn to run.
    await Future.delayed(Duration.zero);
    expect(fake.calls.any((c) => c.type == 'list_applications'), isTrue);
  });

  test('reject() does not refresh when the desktop rejects the request', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['reject_application'] = const RelayResponse(ok: false, errorCode: 'NOT_FOUND', errorMessage: 'Application not found.');

    final controller = ApplicationsController(fake);
    final resp = await controller.reject('missing-app', reason: 'no longer interested');

    expect(resp.ok, isFalse);
    expect(fake.calls.single.type, 'reject_application');
    expect(fake.calls.single.payload, {'id': 'missing-app', 'reason': 'no longer interested'});
  });

  test('loadDetail() returns null and sets error on failure instead of throwing', () async {
    final fake = FakeConnectionController(AuthController());
    fake.stubbed['get_application'] = const RelayResponse(ok: false, errorCode: 'NOT_FOUND', errorMessage: 'Gone.');

    final controller = ApplicationsController(fake);
    final detail = await controller.loadDetail('app-1');

    expect(detail, isNull);
    expect(controller.error, 'Gone.');
  });
}
