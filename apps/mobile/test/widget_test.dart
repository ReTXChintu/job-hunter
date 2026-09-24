// Smoke tests for the small, pure-presentation widgets under lib/widgets.
// These render each widget in isolation -- no controllers, no network --
// to catch obvious layout/null-handling regressions without a device.
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:job_hunter_mobile/widgets/empty_state.dart';
import 'package:job_hunter_mobile/widgets/match_score_bar.dart';
import 'package:job_hunter_mobile/widgets/status_badge.dart';

Widget _wrap(Widget child) => MaterialApp(home: Scaffold(body: Center(child: child)));

void main() {
  group('StatusBadge', () {
    testWidgets('shows the friendly label for a known status', (tester) async {
      await tester.pumpWidget(_wrap(const StatusBadge(status: 'READY_FOR_REVIEW')));
      expect(find.text('Awaiting approval'), findsOneWidget);
    });

    testWidgets('falls back to the raw status string for an unknown one', (tester) async {
      await tester.pumpWidget(_wrap(const StatusBadge(status: 'SOMETHING_NEW')));
      expect(find.text('SOMETHING_NEW'), findsOneWidget);
    });
  });

  group('MatchScoreBar', () {
    testWidgets('shows "Not analyzed" when there is no score', (tester) async {
      await tester.pumpWidget(_wrap(const MatchScoreBar()));
      expect(find.text('Not analyzed'), findsOneWidget);
    });

    testWidgets('shows the percentage when a score is present', (tester) async {
      await tester.pumpWidget(_wrap(const MatchScoreBar(score: 92, relevant: true)));
      expect(find.text('92%'), findsOneWidget);
    });
  });

  group('EmptyState', () {
    testWidgets('renders title and optional message', (tester) async {
      await tester.pumpWidget(_wrap(const EmptyState(icon: Icons.inbox_outlined, title: 'No applications yet', message: 'Check back later.')));
      expect(find.text('No applications yet'), findsOneWidget);
      expect(find.text('Check back later.'), findsOneWidget);
    });

    testWidgets('renders without a message when none is given', (tester) async {
      await tester.pumpWidget(_wrap(const EmptyState(icon: Icons.inbox_outlined, title: 'No applications yet')));
      expect(find.text('No applications yet'), findsOneWidget);
    });
  });
}
