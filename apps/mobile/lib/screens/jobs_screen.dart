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
import '../widgets/job_tile.dart';
import '../widgets/location_filter.dart';
import '../widgets/presence_banner.dart';
import 'applications_list_screen.dart' show FilterChips, PullableEmpty, SearchField;
import 'job_detail_screen.dart';

/// Every job the desktop found, filterable and searchable, newest first.
class JobsScreen extends StatefulWidget {
  const JobsScreen({super.key});

  @override
  State<JobsScreen> createState() => _JobsScreenState();
}

class _JobsScreenState extends State<JobsScreen> {
  JobFilter _filter = JobFilter.all;
  String _location = '';
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
    final counts = jobFilterCounts(jobs.items.where((i) => matchesLocation(i.job, _location)));
    final rows = filterJobs(jobs.items, _filter, _search.text, location: _location);

    Widget body;
    if (jobs.items.isEmpty && (jobs.loading || (jobs.source == DataSource.none && jobs.error == null))) {
      body = const Center(child: CircularProgressIndicator());
    } else if (jobs.items.isEmpty) {
      body = PullableEmpty(
        onRefresh: jobs.refresh,
        child: EmptyState(
          icon: jobs.error != null ? Icons.error_outline : Icons.work_off_outlined,
          title: jobs.error != null ? 'Could not load jobs' : 'No jobs yet',
          message: jobs.error ?? 'Start a job hunt from Home and the jobs your desktop finds show up here.',
        ),
      );
    } else if (rows.isEmpty) {
      body = PullableEmpty(
        onRefresh: jobs.refresh,
        child: EmptyState(
          icon: Icons.filter_alt_off_outlined,
          title: 'Nothing here',
          message: _location.isNotEmpty
              ? 'No jobs in "$_location" here. This only looks through the jobs already found, never the web.'
              : _search.text.trim().isNotEmpty
                  ? 'No jobs match "${_search.text.trim()}".'
                  : 'No jobs in "${_filter.label}".',
        ),
      );
    } else {
      body = RefreshIndicator(
        onRefresh: jobs.refresh,
        child: ListView.builder(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.only(top: 4, bottom: 24),
          itemCount: rows.length,
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
          if (jobs.error != null && jobs.items.isNotEmpty) InlineNotice(icon: Icons.error_outline, text: jobs.error!, color: Theme.of(context).colorScheme.error),
          SearchField(controller: _search, hint: 'Search title, company or site', onChanged: () => setState(() {})),
          FilterChips<JobFilter>(
            values: JobFilter.values,
            selected: _filter,
            label: (f) => '${f.label} (${counts[f]})',
            onSelected: (f) => setState(() => _filter = f),
            leading: LocationChip(value: _location, options: locationOptions(jobs.items.map((i) => i.job)), onChanged: (v) => setState(() => _location = v)),
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
    return JobTile(
      title: job.title,
      subtitle: job.companyLine,
      status: item.applicationStatus ?? job.status,
      score: item.analysis?.matchScore,
      relevant: item.analysis?.relevant,
      tags: [
        if (job.isEmailPost) const Tag('email', color: Colors.indigo, icon: Icons.mail_outline),
        if (job.source.isNotEmpty) Tag(job.source, color: Theme.of(context).colorScheme.secondary),
      ],
      footnote: job.discoveredAt != null ? 'Found ${timeAgo(job.discoveredAt)}' : null,
      onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => JobDetailScreen(jobId: job.id))),
    );
  }
}
