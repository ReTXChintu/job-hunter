import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/filters.dart';
import '../models/application.dart';
import '../state/applications_controller.dart';
import '../state/nav_controller.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/match_score_bar.dart';
import '../widgets/presence_banner.dart';
import '../widgets/status_badge.dart';
import 'application_detail_screen.dart';

/// Every application, filtered by what it needs (chips with counts) and
/// searchable, most recently updated first. Approve/reject happen in the
/// detail screen, never on a stray swipe here.
class ApplicationsListScreen extends StatefulWidget {
  const ApplicationsListScreen({super.key});

  @override
  State<ApplicationsListScreen> createState() => _ApplicationsListScreenState();
}

class _ApplicationsListScreenState extends State<ApplicationsListScreen> {
  ApplicationBucket _bucket = ApplicationBucket.all;
  final _search = TextEditingController();
  late final NavController _nav;

  @override
  void initState() {
    super.initState();
    _nav = context.read<NavController>();
    _nav.addListener(_onNav);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _onNav();
      context.read<ApplicationsController>().refresh();
    });
  }

  void _onNav() {
    final requested = _nav.takeBucket();
    if (requested != null && mounted) setState(() => _bucket = requested);
  }

  @override
  void dispose() {
    _nav.removeListener(_onNav);
    _search.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();
    final counts = bucketCounts(apps.items);
    final rows = filterApplications(apps.items, _bucket, _search.text);

    Widget body;
    if (apps.items.isEmpty && apps.loading) {
      body = const Center(child: CircularProgressIndicator());
    } else if (apps.items.isEmpty) {
      body = _Scrollable(
        onRefresh: apps.refresh,
        child: EmptyState(
          icon: apps.error != null ? Icons.error_outline : Icons.inbox_outlined,
          title: apps.error != null ? 'Could not load applications' : 'No applications yet',
          message: apps.error ?? ApplicationBucket.all.emptyMessage,
        ),
      );
    } else if (rows.isEmpty) {
      body = _Scrollable(
        onRefresh: apps.refresh,
        child: EmptyState(icon: Icons.filter_alt_off_outlined, title: 'Nothing here', message: _search.text.trim().isNotEmpty ? 'No applications match "${_search.text.trim()}".' : _bucket.emptyMessage),
      );
    } else {
      body = RefreshIndicator(
        onRefresh: apps.refresh,
        child: ListView.separated(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.only(bottom: 16),
          itemCount: rows.length,
          separatorBuilder: (context, index) => const Divider(height: 1),
          itemBuilder: (context, i) => ApplicationRow(item: rows[i]),
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(title: const Text('Applications')),
      body: Column(
        children: [
          const PresenceBanner(),
          DataSourceBanner(apps.source),
          if (apps.error != null && apps.items.isNotEmpty)
            Container(
              width: double.infinity,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              color: Theme.of(context).colorScheme.errorContainer,
              child: Text(apps.error!, style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer, fontSize: 13)),
            ),
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 10, 12, 4),
            child: TextField(
              controller: _search,
              onChanged: (_) => setState(() {}),
              decoration: InputDecoration(
                isDense: true,
                prefixIcon: const Icon(Icons.search),
                hintText: 'Search title or company',
                suffixIcon: _search.text.isEmpty ? null : IconButton(icon: const Icon(Icons.clear), onPressed: () => setState(_search.clear)),
              ),
            ),
          ),
          SizedBox(
            height: 48,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              children: [
                for (final b in ApplicationBucket.values)
                  Padding(
                    padding: const EdgeInsets.only(right: 6),
                    child: ChoiceChip(
                      label: Text('${b.label} (${counts[b]})'),
                      selected: _bucket == b,
                      onSelected: (_) => setState(() => _bucket = b),
                      visualDensity: VisualDensity.compact,
                    ),
                  ),
              ],
            ),
          ),
          if (apps.loading && apps.items.isNotEmpty) const LinearProgressIndicator(minHeight: 2),
          Expanded(child: body),
        ],
      ),
    );
  }
}

/// Makes a non-list body pullable to refresh.
class _Scrollable extends StatelessWidget {
  final Future<void> Function() onRefresh;
  final Widget child;
  const _Scrollable({required this.onRefresh, required this.child});

  @override
  Widget build(BuildContext context) => RefreshIndicator(
        onRefresh: onRefresh,
        child: LayoutBuilder(
          builder: (context, constraints) => ListView(
            physics: const AlwaysScrollableScrollPhysics(),
            children: [SizedBox(height: constraints.maxHeight, child: child)],
          ),
        ),
      );
}

class ApplicationRow extends StatelessWidget {
  final ApplicationListItem item;
  const ApplicationRow({super.key, required this.item});

  @override
  Widget build(BuildContext context) {
    final job = item.job;
    final app = item.application;
    final theme = Theme.of(context);
    return InkWell(
      onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => ApplicationDetailScreen(applicationId: app.id))),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(job.title.isEmpty ? 'Untitled job' : job.title, maxLines: 2, overflow: TextOverflow.ellipsis, style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w600)),
            const SizedBox(height: 2),
            Text(job.companyLine, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodyMedium?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
            const SizedBox(height: 8),
            Row(children: [
              Expanded(
                child: Wrap(spacing: 6, runSpacing: 4, crossAxisAlignment: WrapCrossAlignment.center, children: [
                  StatusBadge(status: app.status),
                  if (job.isEmailPost) const Tag('email', color: Colors.indigo, icon: Icons.mail_outline),
                  if (app.replies.isNotEmpty) Tag('${app.replies.length} ${app.replies.length == 1 ? 'reply' : 'replies'}', color: Colors.teal, icon: Icons.forward_to_inbox),
                  if (app.pendingQuestions.isNotEmpty) Tag('${app.pendingQuestions.length} question${app.pendingQuestions.length == 1 ? '' : 's'}', color: Colors.orange),
                ]),
              ),
              const SizedBox(width: 8),
              MatchScoreBar(score: item.analysis?.matchScore, relevant: item.analysis?.relevant),
            ]),
            if (app.updatedAt.year > 1970) ...[
              const SizedBox(height: 4),
              Text('Updated ${timeAgo(app.updatedAt)}', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
            ],
          ],
        ),
      ),
    );
  }
}
