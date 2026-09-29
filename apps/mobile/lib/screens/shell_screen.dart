import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/filters.dart';
import '../logic/links.dart';
import '../services/link_bus.dart';
import '../state/applications_controller.dart';
import '../state/nav_controller.dart';
import '../state/notifications_controller.dart';
import 'about_screen.dart';
import 'application_detail_screen.dart';
import 'applications_list_screen.dart';
import 'home_screen.dart';
import 'job_sites_screen.dart';
import 'jobs_screen.dart';
import 'more_screen.dart';
import 'notifications_screen.dart';

/// Opens the screen a notification links to: switches tab, then pushes a
/// detail screen when the link names one.
void openAppLink(BuildContext context, AppLink link) {
  final nav = context.read<NavController>();
  switch (link.kind) {
    case LinkKind.application:
      nav.select(tabApplications);
      Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => ApplicationDetailScreen(applicationId: link.id!)));
    case LinkKind.applications:
      nav.openApplications();
    case LinkKind.jobs:
      nav.openJobs();
    case LinkKind.jobSites:
      nav.select(tabMore);
      Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => const JobSitesScreen()));
    case LinkKind.home:
      nav.select(tabHome);
  }
}

/// The signed-in shell: Home, Applications, Jobs, Alerts, More.
class AppShell extends StatefulWidget {
  const AppShell({super.key});

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  StreamSubscription<AppLink>? _linkSub;

  @override
  void initState() {
    super.initState();
    // Notification taps: now, and any that launched the app before this existed.
    _linkSub = LinkBus.instance.links.listen((_) => _takeLink());
    WidgetsBinding.instance.addPostFrameCallback((_) => _takeLink());
  }

  void _takeLink() {
    if (!mounted) return;
    final link = LinkBus.instance.take();
    if (link == null) return;
    // Close any detail screen first so the link opens from the shell.
    Navigator.of(context).popUntil((route) => route.isFirst);
    openAppLink(context, link);
  }

  @override
  void dispose() {
    _linkSub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final nav = context.watch<NavController>();
    final unread = context.watch<NotificationsController>().unread;
    final counts = bucketCounts(context.watch<ApplicationsController>().items);
    final waiting = counts[ApplicationBucket.needsYou]! + counts[ApplicationBucket.pendingApproval]!;

    Widget badged(IconData icon, int count) => Badge(isLabelVisible: count > 0, label: Text(count > 99 ? '99+' : '$count'), child: Icon(icon));

    return Scaffold(
      body: Column(
        children: [
          const UpdateBanner(),
          Expanded(
            child: IndexedStack(
              index: nav.tab,
              children: const [HomeScreen(), ApplicationsListScreen(), JobsScreen(), NotificationsScreen(), MoreScreen()],
            ),
          ),
        ],
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: nav.tab,
        onDestinationSelected: (i) {
          // Leaving the Alerts tab counts as having seen them.
          if (nav.tab == tabAlerts && i != tabAlerts) context.read<NotificationsController>().markAllSeen();
          nav.select(i);
        },
        destinations: [
          const NavigationDestination(icon: Icon(Icons.home_outlined), selectedIcon: Icon(Icons.home), label: 'Home'),
          NavigationDestination(icon: badged(Icons.fact_check_outlined, waiting), selectedIcon: badged(Icons.fact_check, waiting), label: 'Applications'),
          const NavigationDestination(icon: Icon(Icons.work_outline), selectedIcon: Icon(Icons.work), label: 'Jobs'),
          NavigationDestination(icon: badged(Icons.notifications_outlined, unread), selectedIcon: badged(Icons.notifications, unread), label: 'Alerts'),
          const NavigationDestination(icon: Icon(Icons.more_horiz), selectedIcon: Icon(Icons.more_horiz), label: 'More'),
        ],
      ),
    );
  }
}
