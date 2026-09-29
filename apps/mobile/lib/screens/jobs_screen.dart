import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/filters.dart';
import '../logic/links.dart';
import '../models/job.dart';
import '../state/data_source.dart';
import '../state/jobs_controller.dart';
import '../state/nav_controller.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/match_score_bar.dart';
import '../widgets/presence_banner.dart';
import '../widgets/status_badge.dart';
import 'job_detail_screen.dart';

/// Every job the desktop found, filterable and searchable, newest first.
class JobsScreen extends StatefulWidget {
  const JobsScreen({super.key});

  @override
  State<JobsScreen> createState() => _JobsScreenState();
}

class _JobsScreenState extends State<JobsScreen> {
  JobFilter _filter = JobFilter.all;
  final _search = TextEditingController();
  late final NavController _nav;

  @override
  void initState() {
    super.initState();
    _nav = context.read<NavController>();
    _nav.addListener(_onNav);
    WidgetsBinding.instance.addPostFrameCallback((_) => _onNav());
  }

  void _onNav() {
    if (!mounted) return;
    if (_nav.tab == tabJobs) context.read<JobsController>().ensureLoaded();
    final requested = _nav.takeJobFilter();
    if (requested != null) setState(() => _filter = requested);
  }

  @override
  void dispose() {
    _nav.removeListener(_onNav);
    _search.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final jobs = context.watch<JobsController>();
    final counts = jobFilterCounts(jobs.items);
    final rows = filterJobs(jobs.items, _filter, _search.text);

    Widget pullable(Widget child) => RefreshIndicator(
          onRefresh: jobs.refresh,
          child: LayoutBuilder(builder: (context, c) => ListView(physics: const AlwaysScrollableScrollPhysics(), children: [SizedBox(height: c.maxHeight, child: child)])),
        );

    Widget body;
    if (jobs.items.isEmpty && (jobs.loading || (jobs.source == DataSource.none && jobs.error == null))) {
      body = const Center(child: CircularProgressIndicator());
    } else if (jobs.items.isEmpty) {
      body = pullable(EmptyState(
        icon: jobs.error != null ? Icons.error_outline : Icons.work_off_outlined,
        title: jobs.error != null ? 'Could not load jobs' : 'No jobs yet',
        message: jobs.error ?? 'Start a job hunt from Home and the jobs your desktop finds show up here.',
      ));
    } else if (rows.isEmpty) {
      body = pullable(EmptyState(icon: Icons.filter_alt_off_outlined, title: 'Nothing here', message: _search.text.trim().isNotEmpty ? 'No jobs match "${_search.text.trim()}".' : 'No jobs in "${_filter.label}".'));
    } else {
      body = RefreshIndicator(
        onRefresh: jobs.refresh,
        child: ListView.separated(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.only(bottom: 16),
          itemCount: rows.length,
          separatorBuilder: (context, index) => const Divider(height: 1),
          itemBuilder: (context, i) => _JobRow(item: rows[i]),
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(title: const Text('Jobs')),
      body: Column(
        children: [
          const PresenceBanner(),
          DataSourceBanner(jobs.source),
          if (jobs.error != null && jobs.items.isNotEmpty)
            Container(
              width: double.infinity,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              color: Theme.of(context).colorScheme.errorContainer,
              child: Text(jobs.error!, style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer, fontSize: 13)),
            ),
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 10, 12, 4),
            child: TextField(
              controller: _search,
              onChanged: (_) => setState(() {}),
              decoration: InputDecoration(
                isDense: true,
                prefixIcon: const Icon(Icons.search),
                hintText: 'Search title, company or site',
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
                for (final f in JobFilter.values)
                  Padding(
                    padding: const EdgeInsets.only(right: 6),
                    child: ChoiceChip(label: Text('${f.label} (${counts[f]})'), selected: _filter == f, onSelected: (_) => setState(() => _filter = f), visualDensity: VisualDensity.compact),
                  ),
              ],
            ),
          ),
          if (jobs.loading && jobs.items.isNotEmpty) const LinearProgressIndicator(minHeight: 2),
          Expanded(child: body),
        ],
      ),
    );
  }
}

class _JobRow extends StatelessWidget {
  final JobListItem item;
  const _JobRow({required this.item});

  @override
  Widget build(BuildContext context) {
    final job = item.job;
    final theme = Theme.of(context);
    return InkWell(
      onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => JobDetailScreen(jobId: job.id))),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(job.title.isEmpty ? 'Untitled job' : job.title, maxLines: 2, overflow: TextOverflow.ellipsis, style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w600)),
          const SizedBox(height: 2),
          Text(job.companyLine, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodyMedium?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          const SizedBox(height: 8),
          Row(children: [
            Expanded(
              child: Wrap(spacing: 6, runSpacing: 4, crossAxisAlignment: WrapCrossAlignment.center, children: [
                StatusBadge(status: item.applicationStatus ?? job.status),
                if (job.isEmailPost) const Tag('email', color: Colors.indigo, icon: Icons.mail_outline),
                if (job.source.isNotEmpty) Tag(job.source, color: theme.colorScheme.secondary),
              ]),
            ),
            const SizedBox(width: 8),
            MatchScoreBar(score: item.analysis?.matchScore, relevant: item.analysis?.relevant),
          ]),
          if (job.discoveredAt != null) ...[
            const SizedBox(height: 4),
            Text('Found ${timeAgo(job.discoveredAt)}', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          ],
        ]),
      ),
    );
  }
}
