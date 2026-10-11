import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/filters.dart';
import '../models/application.dart';
import '../state/applications_controller.dart';
import '../state/nav_controller.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/job_tile.dart';
import '../widgets/presence_banner.dart';
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
      body = PullableEmpty(
        onRefresh: apps.refresh,
        child: EmptyState(
          icon: apps.error != null ? Icons.error_outline : Icons.inbox_outlined,
          title: apps.error != null ? 'Could not load applications' : 'No applications yet',
          message: apps.error ?? ApplicationBucket.all.emptyMessage,
        ),
      );
    } else if (rows.isEmpty) {
      body = PullableEmpty(
        onRefresh: apps.refresh,
        child: EmptyState(icon: Icons.filter_alt_off_outlined, title: 'Nothing here', message: _search.text.trim().isNotEmpty ? 'No applications match "${_search.text.trim()}".' : _bucket.emptyMessage),
      );
    } else {
      body = RefreshIndicator(
        onRefresh: apps.refresh,
        child: ListView.builder(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.only(top: 4, bottom: 24),
          itemCount: rows.length,
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
          if (apps.error != null && apps.items.isNotEmpty) InlineNotice(icon: Icons.error_outline, text: apps.error!, color: Theme.of(context).colorScheme.error),
          SearchField(controller: _search, hint: 'Search title or company', onChanged: () => setState(() {})),
          FilterChips<ApplicationBucket>(
            values: ApplicationBucket.values,
            selected: _bucket,
            label: (b) => '${b.label} (${counts[b]})',
            onSelected: (b) => setState(() => _bucket = b),
          ),
          if (apps.loading && apps.items.isNotEmpty) const LinearProgressIndicator(minHeight: 2),
          Expanded(child: body),
        ],
      ),
    );
  }
}

/// The search box above a list.
class SearchField extends StatelessWidget {
  final TextEditingController controller;
  final String hint;
  final VoidCallback onChanged;
  const SearchField({super.key, required this.controller, required this.hint, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
      child: TextField(
        controller: controller,
        onChanged: (_) => onChanged(),
        decoration: InputDecoration(
          prefixIcon: const Icon(Icons.search, size: 20),
          hintText: hint,
          suffixIcon: controller.text.isEmpty
              ? null
              : IconButton(
                  icon: const Icon(Icons.clear, size: 18),
                  onPressed: () {
                    controller.clear();
                    onChanged();
                  },
                ),
        ),
      ),
    );
  }
}

/// A horizontal row of filter chips with counts.
class FilterChips<T> extends StatelessWidget {
  final List<T> values;
  final T selected;
  final String Function(T) label;
  final ValueChanged<T> onSelected;
  const FilterChips({super.key, required this.values, required this.selected, required this.label, required this.onSelected});

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 50,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
        children: [
          for (final v in values)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: ChoiceChip(label: Text(label(v)), selected: selected == v, onSelected: (_) => onSelected(v)),
            ),
        ],
      ),
    );
  }
}

/// Makes a non-list body pullable to refresh.
class PullableEmpty extends StatelessWidget {
  final Future<void> Function() onRefresh;
  final Widget child;
  const PullableEmpty({super.key, required this.onRefresh, required this.child});

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
    return JobTile(
      title: job.title,
      subtitle: job.companyLine,
      status: app.status,
      score: item.analysis?.matchScore,
      relevant: item.analysis?.relevant,
      tags: [
        if (job.isEmailPost) const Tag('email', color: Colors.indigo, icon: Icons.mail_outline),
        if (app.replies.isNotEmpty) Tag('${app.replies.length} ${app.replies.length == 1 ? 'reply' : 'replies'}', color: Colors.teal, icon: Icons.forward_to_inbox),
        if (app.pendingQuestions.isNotEmpty) Tag('${app.pendingQuestions.length} question${app.pendingQuestions.length == 1 ? '' : 's'}', color: Colors.orange, icon: Icons.help_outline),
      ],
      footnote: app.updatedAt.year > 1970 ? 'Updated ${timeAgo(app.updatedAt)}' : null,
      onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => ApplicationDetailScreen(applicationId: app.id))),
    );
  }
}
