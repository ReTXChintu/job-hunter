import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/labels.dart';
import '../models/envelope.dart';
import '../models/job.dart';
import '../state/connection_controller.dart';
import '../state/data_source.dart';
import '../state/jobs_controller.dart';
import '../theme/theme.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/match_score_bar.dart';
import '../widgets/status_badge.dart';
import 'application_detail_screen.dart';

/// One job: the posting, the desktop's analysis, and what to do with it --
/// prepare an application (the desktop tailors a resume, then asks for your
/// approval) or reject it.
class JobDetailScreen extends StatefulWidget {
  final String jobId;
  const JobDetailScreen({super.key, required this.jobId});

  @override
  State<JobDetailScreen> createState() => _JobDetailScreenState();
}

class _JobDetailScreenState extends State<JobDetailScreen> {
  JobDetail? _detail;
  bool _loading = true;
  bool _fromServer = false;
  String? _error;
  bool _acting = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = _detail == null;
      _error = null;
    });
    final jobs = context.read<JobsController>();
    final detail = await jobs.loadDetail(widget.jobId);
    if (!mounted) return;
    setState(() {
      if (detail != null) _detail = detail;
      _fromServer = jobs.lastDetailFromServer;
      _loading = false;
      _error = detail == null ? jobs.lastDetailError : null;
    });
  }

  Future<void> _act(Future<RelayResponse> Function() action, String success) async {
    setState(() => _acting = true);
    final resp = await action();
    if (!mounted) return;
    setState(() => _acting = false);
    if (showOutcome(context, resp, success: success)) await _load();
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) return Scaffold(appBar: AppBar(), body: const Center(child: CircularProgressIndicator()));
    final detail = _detail;
    if (detail == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Job')),
        body: EmptyState(icon: Icons.error_outline, title: 'Could not load this job', message: _error, action: FilledButton(onPressed: _load, child: const Text('Try again'))),
      );
    }
    final connection = context.watch<ConnectionController>();
    final disabledReason = connection.desktopReachable ? null : (connection.connected ? desktopOfflineMessage : notConnectedMessage);
    final job = detail.job;
    final analysis = detail.analysis;
    final application = detail.application;
    final theme = Theme.of(context);
    final closed = job.status == 'REJECTED' || job.status == 'NOT_RELEVANT';
    final facts = [job.employmentType, job.remote, job.seniority].whereType<String>().where((s) => s.isNotEmpty).map((s) => s.replaceAll('_', ' ').toLowerCase()).toList();

    return Scaffold(
      appBar: AppBar(title: Text(job.company.isEmpty ? 'Job' : job.company, maxLines: 1, overflow: TextOverflow.ellipsis)),
      body: Column(children: [
        if (_fromServer) const DataSourceBanner(DataSource.server),
        Expanded(
          child: RefreshIndicator(
            onRefresh: _load,
            child: ListView(
              physics: const AlwaysScrollableScrollPhysics(),
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 32),
              children: [
                Text(job.title.isEmpty ? 'Untitled job' : job.title, style: theme.textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700)),
                const SizedBox(height: 4),
                Text(job.companyLine, style: theme.textTheme.titleMedium),
                if (job.salary != null) Text(job.salary!),
                if (facts.isNotEmpty) Text(facts.join(' · '), style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                const SizedBox(height: 10),
                Wrap(spacing: 8, runSpacing: 6, crossAxisAlignment: WrapCrossAlignment.center, children: [
                  StatusBadge(status: job.status),
                  MatchScoreBar(score: analysis?.matchScore, relevant: analysis?.relevant),
                  if (job.source.isNotEmpty) Tag(job.source, color: theme.colorScheme.secondary),
                ]),
                if (job.discoveredAt != null) Padding(padding: const EdgeInsets.only(top: 6), child: Text('Found ${formatDateTime(context, job.discoveredAt)}', style: theme.textTheme.bodySmall)),
                if (job.isEmailPost) ...[
                  const SizedBox(height: 10),
                  Row(children: [
                    const Icon(Icons.mail_outline, size: 18, color: Colors.indigo),
                    const SizedBox(width: 6),
                    Expanded(child: Text('Hiring post: apply by email to ${job.contactName != null ? '${job.contactName} · ' : ''}${job.applyEmail}')),
                  ]),
                ],
                if (job.url.isNotEmpty)
                  Align(alignment: Alignment.centerLeft, child: TextButton.icon(onPressed: () => openExternal(context, job.url), icon: const Icon(Icons.open_in_new, size: 16), label: const Text('View source posting'))),
                if (application != null)
                  Callout(
                    icon: Icons.fact_check_outlined,
                    color: theme.colorScheme.primary,
                    title: 'Application: ${statusLabel(application.status)}',
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: FilledButton.tonal(
                        onPressed: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => ApplicationDetailScreen(applicationId: application.id))),
                        child: const Text('Open application'),
                      ),
                    ),
                  )
                else if (!closed)
                  Callout(
                    icon: Icons.auto_awesome_outlined,
                    color: theme.colorScheme.primary,
                    title: 'No application yet',
                    message: 'Prepare one: your desktop tailors a resume for this job, then waits for your approval before anything is sent.',
                    child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
                      Row(children: [
                        Expanded(
                          child: OutlinedButton(
                            onPressed: _acting || disabledReason != null
                                ? null
                                : () async {
                                    final ok = await confirmDialog(context, title: 'Reject this job?', message: 'It will be marked as rejected and no application will be prepared.', confirm: 'Reject job');
                                    if (!ok || !mounted) return;
                                    await _act(() => context.read<JobsController>().rejectJob(job.id), 'Job rejected.');
                                  },
                            child: const Text('Reject job'),
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          flex: 2,
                          child: FilledButton(
                            onPressed: _acting || disabledReason != null ? null : () => _act(() => context.read<JobsController>().prepareApplication(job.id), 'Preparing an application on your desktop.'),
                            child: const Text('Prepare application'),
                          ),
                        ),
                      ]),
                      if (disabledReason != null) Padding(padding: const EdgeInsets.only(top: 6), child: Text(disabledReason, style: TextStyle(color: theme.colorScheme.error))),
                    ]),
                  ),
                if (analysis != null)
                  Section(
                    title: 'Match analysis',
                    child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                      Text(analysis.summary.isEmpty ? 'No summary provided.' : analysis.summary),
                      if (analysis.matchedSkills.isNotEmpty) ...[const SizedBox(height: 12), const Text('Matched skills'), const SizedBox(height: 4), ChipRow(analysis.matchedSkills, color: Colors.green)],
                      if (analysis.missingSkills.isNotEmpty) ...[const SizedBox(height: 12), const Text('Missing skills'), const SizedBox(height: 4), ChipRow(analysis.missingSkills, color: Colors.orange)],
                      if (analysis.concerns.isNotEmpty) ...[const SizedBox(height: 12), const Text('Concerns'), const SizedBox(height: 4), BulletList(analysis.concerns)],
                    ]),
                  ),
                if (job.description.isNotEmpty) Section(title: 'Description', child: ExpandableText(job.description)),
                if (job.requirements.isNotEmpty) Section(title: 'Requirements', child: BulletList(job.requirements)),
                if (job.responsibilities.isNotEmpty) Section(title: 'Responsibilities', child: BulletList(job.responsibilities)),
                if (job.skills.isNotEmpty) Section(title: 'Skills', child: ChipRow(job.skills, color: theme.colorScheme.primary)),
              ],
            ),
          ),
        ),
      ]),
    );
  }
}
