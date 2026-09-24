import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/application.dart';
import '../state/applications_controller.dart';
import '../widgets/empty_state.dart';
import '../widgets/match_score_bar.dart';
import '../widgets/presence_banner.dart';
import '../widgets/status_badge.dart';
import 'application_detail_screen.dart';

/// The review list: every application the desktop's agent has produced,
/// newest activity first, read from the cache instantly and refreshed as
/// soon as the relay connects. Approve/reject happen in the detail screen,
/// not here, to avoid a stray swipe changing a real job application.
class ApplicationsListScreen extends StatefulWidget {
  const ApplicationsListScreen({super.key});

  @override
  State<ApplicationsListScreen> createState() => _ApplicationsListScreenState();
}

class _ApplicationsListScreenState extends State<ApplicationsListScreen> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => context.read<ApplicationsController>().refresh());
  }

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();
    return Scaffold(
      appBar: AppBar(title: const Text('Applications')),
      body: Column(
        children: [
          const PresenceBanner(),
          if (apps.isFromCache)
            Container(
              width: double.infinity,
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              child: Text('Showing cached data', style: Theme.of(context).textTheme.bodySmall),
            ),
          if (apps.error != null && apps.items.isEmpty)
            Expanded(child: EmptyState(icon: Icons.error_outline, title: 'Could not load applications', message: apps.error))
          else if (apps.items.isEmpty && apps.loading)
            const Expanded(child: Center(child: CircularProgressIndicator()))
          else if (apps.items.isEmpty)
            const Expanded(child: EmptyState(icon: Icons.inbox_outlined, title: 'No applications yet', message: 'Applications your desktop agent finds will show up here.'))
          else
            Expanded(
              child: RefreshIndicator(
                onRefresh: apps.refresh,
                child: ListView.separated(
                  padding: const EdgeInsets.only(bottom: 12),
                  itemCount: apps.items.length,
                  separatorBuilder: (context, index) => const Divider(height: 1),
                  itemBuilder: (context, i) => _ApplicationRow(item: apps.items[i]),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _ApplicationRow extends StatelessWidget {
  final ApplicationListItem item;
  const _ApplicationRow({required this.item});

  @override
  Widget build(BuildContext context) {
    final job = item.job;
    final app = item.application;
    return ListTile(
      onTap: () => Navigator.of(context).push(MaterialPageRoute(builder: (_) => ApplicationDetailScreen(applicationId: app.id))),
      title: Text(job.title, maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontWeight: FontWeight.w600)),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('${job.company} · ${job.location}', maxLines: 1, overflow: TextOverflow.ellipsis),
          const SizedBox(height: 6),
          Row(children: [StatusBadge(status: app.status), const Spacer(), MatchScoreBar(score: item.analysis?.matchScore, relevant: item.analysis?.relevant)]),
        ],
      ),
      isThreeLine: true,
    );
  }
}
