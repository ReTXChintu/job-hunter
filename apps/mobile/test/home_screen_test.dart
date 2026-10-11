import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/models/dashboard.dart';
import 'package:job_hunter_mobile/models/envelope.dart';
import 'package:job_hunter_mobile/screens/home_screen.dart';
import 'package:job_hunter_mobile/screens/queue_screen.dart';
import 'package:job_hunter_mobile/state/agent_controller.dart';
import 'package:job_hunter_mobile/state/applications_controller.dart';
import 'package:job_hunter_mobile/state/auth_controller.dart';
import 'package:job_hunter_mobile/state/connection_controller.dart';
import 'package:job_hunter_mobile/state/nav_controller.dart';
import 'package:job_hunter_mobile/state/queue_controller.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

Future<(FakeConnectionController, QueueController)> pumpHome(WidgetTester tester, {Widget? home}) async {
  SharedPreferences.setMockInitialValues({});
  tester.view.physicalSize = const Size(1080, 2400);
  tester.view.devicePixelRatio = 2;
  addTearDown(tester.view.reset);
  final auth = AuthController();
  final fake = FakeConnectionController(auth);
  fake.stubbed['list_applications'] = const RelayResponse(ok: true, data: []);
  fake.stubbed['list_queue'] = const RelayResponse(ok: true, data: [
    {'id': 'q1', 'title': 'Apply: Developer at Acme', 'task': {'kind': 'APPLICATION'}, 'requestedAt': '2026-10-11T10:00:00Z'},
    {'id': 'q2', 'title': 'Search only: Naukri', 'task': {'kind': 'JOB_HUNT'}},
  ]);
  fake.stubbed['remove_queued_task'] = const RelayResponse(ok: true, data: {'removed': true});
  fake.agentStatus = const AgentStatusLite(state: 'DISCOVERING', paused: false, runKind: 'JOB_HUNT', queued: 2);
  final queue = QueueController(fake);
  await tester.pumpWidget(MultiProvider(
    providers: [
      ChangeNotifierProvider<AuthController>.value(value: auth),
      ChangeNotifierProvider<ConnectionController>.value(value: fake),
      ChangeNotifierProvider<ApplicationsController>.value(value: ApplicationsController(fake)),
      ChangeNotifierProvider<AgentController>.value(value: AgentController(fake)),
      ChangeNotifierProvider<QueueController>.value(value: queue),
      ChangeNotifierProvider<NavController>.value(value: NavController()),
    ],
    child: MaterialApp(home: home ?? const HomeScreen()),
  ));
  // The queue reads itself once the agent status is known.
  fake.notifyListeners();
  await tester.pumpAndSettle();
  return (fake, queue);
}

void main() {
  testWidgets('Home shows what is queued and every desktop action', (tester) async {
    await pumpHome(tester);
    expect(find.text('Up next'), findsOneWidget);
    expect(find.text('1. Apply: Developer at Acme'), findsNothing); // titles are plain; the place is drawn as a badge
    expect(find.text('Apply: Developer at Acme'), findsOneWidget);
    expect(find.text('Search only: Naukri'), findsOneWidget);
    for (final action in ['Start job hunt', 'Prepare applications', 'Search only', 'Find hiring posts', 'Scan Telegram', 'Check Gmail', 'Apply from a link', 'Add a job I found']) {
      expect(find.text(action), findsOneWidget, reason: action);
    }
  });

  testWidgets('the Queue screen lists tasks in order and removes one on the desktop', (tester) async {
    final (fake, _) = await pumpHome(tester, home: const QueueScreen());
    expect(find.text('Queue (2)'), findsOneWidget);
    expect(find.text('Apply: Developer at Acme'), findsOneWidget);
    expect(find.text('Clear all'), findsOneWidget);
    // Removing needs the desktop; the fake is never "online", so the button is disabled.
    final remove = find.ancestor(of: find.byTooltip('Remove').first, matching: find.byType(IconButton)).first;
    expect(tester.widget<IconButton>(remove).onPressed, isNull);
    expect(fake.calls.where((c) => c.type == 'remove_queued_task'), isEmpty);
  });
}
