import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/logic/filters.dart';
import 'package:job_hunter_mobile/models/envelope.dart';
import 'package:job_hunter_mobile/screens/applications_list_screen.dart';
import 'package:job_hunter_mobile/state/applications_controller.dart';
import 'package:job_hunter_mobile/state/auth_controller.dart';
import 'package:job_hunter_mobile/state/connection_controller.dart';
import 'package:job_hunter_mobile/state/nav_controller.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

Map<String, dynamic> _row(String id, String status, String title, {String? applyEmail, int replies = 0}) => {
      'application': {
        'id': id,
        'status': status,
        'updatedAt': '2026-09-2${id.length}T00:00:00Z',
        'replies': [for (var i = 0; i < replies; i++) {'subject': 'Re $i'}],
      },
      'job': {'id': 'job-$id', 'title': title, 'company': 'Acme', 'location': 'Remote', 'applyEmail': applyEmail},
      'analysis': {'matchScore': 80, 'relevant': true},
    };

void main() {
  testWidgets('chips show counts and filter the list; Home can request a chip', (tester) async {
    SharedPreferences.setMockInitialValues({});
    // Wide enough that every filter chip is laid out.
    tester.view.physicalSize = const Size(2000, 1400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final auth = AuthController();
    final fake = FakeConnectionController(auth);
    fake.stubbed['list_applications'] = RelayResponse(ok: true, data: [
      _row('a', 'READY_FOR_REVIEW', 'Flutter Engineer'),
      _row('bb', 'APPLIED', 'Backend Engineer', applyEmail: 'hr@acme.test', replies: 2),
      _row('ccc', 'WAITING_FOR_USER', 'Platform Engineer'),
    ]);
    final apps = ApplicationsController(fake);
    final nav = NavController();

    await tester.pumpWidget(MultiProvider(
      providers: [
        ChangeNotifierProvider<AuthController>.value(value: auth),
        ChangeNotifierProvider<ConnectionController>.value(value: fake),
        ChangeNotifierProvider<ApplicationsController>.value(value: apps),
        ChangeNotifierProvider<NavController>.value(value: nav),
      ],
      child: const MaterialApp(home: ApplicationsListScreen()),
    ));
    await tester.pumpAndSettle();

    expect(find.text('All (3)'), findsOneWidget);
    expect(find.text('Needs you (1)'), findsOneWidget);
    expect(find.text('Pending approval (1)'), findsOneWidget);
    expect(find.text('Flutter Engineer'), findsOneWidget);
    expect(find.text('email'), findsOneWidget);
    expect(find.text('2 replies'), findsOneWidget);

    await tester.tap(find.text('Applied (1)'));
    await tester.pumpAndSettle();
    expect(find.text('Backend Engineer'), findsOneWidget);
    expect(find.text('Flutter Engineer'), findsNothing);

    nav.openApplications(ApplicationBucket.needsYou);
    await tester.pumpAndSettle();
    expect(find.text('Platform Engineer'), findsOneWidget);
    expect(find.text('Backend Engineer'), findsNothing);

    await tester.tap(find.text('All (3)'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'backend');
    await tester.pumpAndSettle();
    expect(find.text('Backend Engineer'), findsOneWidget);
    expect(find.text('Platform Engineer'), findsNothing);
  });
}
