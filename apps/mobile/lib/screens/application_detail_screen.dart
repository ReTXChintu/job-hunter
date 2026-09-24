import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../models/application.dart';
import '../state/applications_controller.dart';
import '../widgets/match_score_bar.dart';
import '../widgets/status_badge.dart';

/// The review screen: everything the phone needs to decide approve/reject,
/// answer a pending question, or confirm a manual step -- all of it going
/// through `ApplicationsController`, which forwards to the exact same
/// `orchestrator` calls the desktop UI makes. Nothing here decides an
/// outcome locally.
class ApplicationDetailScreen extends StatefulWidget {
  final String applicationId;
  const ApplicationDetailScreen({super.key, required this.applicationId});

  @override
  State<ApplicationDetailScreen> createState() => _ApplicationDetailScreenState();
}

class _ApplicationDetailScreenState extends State<ApplicationDetailScreen> {
  ApplicationDetail? _detail;
  bool _loading = true;
  String? _error;
  bool _acting = false;
  final Map<String, TextEditingController> _answerControllers = {};

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final c in _answerControllers.values) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    final detail = await context.read<ApplicationsController>().loadDetail(widget.applicationId);
    if (!mounted) return;
    setState(() {
      _detail = detail;
      _loading = false;
      _error = detail == null ? 'Could not load this application.' : null;
      for (final q in detail?.application.pendingQuestions ?? const <PendingQuestion>[]) {
        _answerControllers.putIfAbsent(q.id, () => TextEditingController());
      }
    });
  }

  Future<void> _act(Future<dynamic> Function() action, {String? successMessage}) async {
    setState(() => _acting = true);
    final resp = await action();
    if (!mounted) return;
    setState(() => _acting = false);
    final ok = resp is Object && (resp as dynamic).ok == true;
    if (ok) {
      if (successMessage != null) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(successMessage)));
      }
      await _load();
    } else {
      final message = (resp as dynamic).errorMessage as String? ?? 'That did not go through.';
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
    }
  }

  Future<bool> _confirm(String title, String message) => showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text(title),
          content: Text(message),
          actions: [
            TextButton(onPressed: () => Navigator.of(ctx).pop(false), child: const Text('Cancel')),
            FilledButton(onPressed: () => Navigator.of(ctx).pop(true), child: const Text('Confirm')),
          ],
        ),
      ).then((v) => v ?? false);

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();

    if (_loading) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    if (_detail == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Application')),
        body: Center(child: Padding(padding: const EdgeInsets.all(24), child: Text(_error ?? 'Not found'))),
      );
    }

    final detail = _detail!;
    final app = detail.application;
    final job = detail.job;
    final analysis = detail.analysis;

    return Scaffold(
      appBar: AppBar(title: Text(job.title, maxLines: 1, overflow: TextOverflow.ellipsis)),
      body: RefreshIndicator(
        onRefresh: _load,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 100),
          children: [
            Row(children: [StatusBadge(status: app.status), const SizedBox(width: 10), MatchScoreBar(score: analysis?.matchScore, relevant: analysis?.relevant)]),
            const SizedBox(height: 12),
            Text('${job.company} · ${job.location}', style: Theme.of(context).textTheme.titleMedium),
            if (job.salary != null) Text(job.salary!, style: Theme.of(context).textTheme.bodyMedium),
            const SizedBox(height: 4),
            if (job.url.isNotEmpty)
              TextButton.icon(
                onPressed: () => launchUrl(Uri.parse(job.url), mode: LaunchMode.externalApplication),
                icon: const Icon(Icons.open_in_new, size: 16),
                label: const Text('View posting'),
              ),
            if (analysis != null) ...[
              const SizedBox(height: 8),
              _Section(title: 'Why this match', child: Text(analysis.summary.isEmpty ? 'No summary provided.' : analysis.summary)),
              if (analysis.concerns.isNotEmpty) _Section(title: 'Concerns', child: _BulletList(analysis.concerns)),
              if (analysis.matchedSkills.isNotEmpty) _Section(title: 'Matched skills', child: _ChipRow(analysis.matchedSkills, color: Colors.green)),
              if (analysis.missingSkills.isNotEmpty) _Section(title: 'Missing skills', child: _ChipRow(analysis.missingSkills, color: Colors.orange)),
            ],
            if (app.potentialIssues.isNotEmpty) _Section(title: 'Potential issues', child: _BulletList(app.potentialIssues)),
            if (detail.resume != null) _Section(title: 'Resume', child: Text(detail.resume!.label.isEmpty ? 'Tailored resume prepared.' : detail.resume!.label)),
            if (detail.coverLetter != null && detail.coverLetter!.text.isNotEmpty) _Section(title: 'Cover letter', child: Text(detail.coverLetter!.text, maxLines: 8, overflow: TextOverflow.ellipsis)),
            if (app.answers.isNotEmpty)
              _Section(
                title: 'Answers',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: app.answers.map((a) => Padding(padding: const EdgeInsets.only(bottom: 8), child: Text('${a.question}\n${a.answer}'))).toList(),
                ),
              ),
            if (app.pendingQuestions.isNotEmpty)
              _Section(
                title: 'Needs your input',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    ...app.pendingQuestions.map((q) => Padding(
                          padding: const EdgeInsets.only(bottom: 10),
                          child: TextField(
                            controller: _answerControllers[q.id],
                            decoration: InputDecoration(labelText: q.question, helperText: q.context.isEmpty ? null : q.context),
                            maxLines: q.fieldType == 'textarea' ? 3 : 1,
                          ),
                        )),
                    FilledButton(
                      onPressed: _acting
                          ? null
                          : () => _act(
                                () => apps.answerQuestions(app.id, app.pendingQuestions.map((q) => ApplicationAnswer(question: q.question, answer: _answerControllers[q.id]?.text.trim() ?? '', source: 'USER')).toList()),
                                successMessage: 'Answers sent.',
                              ),
                      child: const Text('Submit answers'),
                    ),
                  ],
                ),
              ),
            if (app.failureReason != null) _Section(title: 'Issue', child: Text(app.failureReason!, style: TextStyle(color: Theme.of(context).colorScheme.error))),
          ],
        ),
      ),
      bottomNavigationBar: _ActionBar(status: app.status, acting: _acting, onApprove: () => _act(() => apps.approve(app.id), successMessage: 'Approved.'), onReject: () async {
        if (!await _confirm('Reject application?', 'This tells your desktop agent to stop working on ${job.title} at ${job.company}.')) return;
        await _act(() => apps.reject(app.id), successMessage: 'Rejected.');
      }, onApplyNow: () => _act(() => apps.applyNow(app.id), successMessage: 'Applying now.'), onMarkComplete: () async {
        if (!await _confirm('Mark as done?', 'This confirms you finished the manual step for this application.')) return;
        await _act(() => apps.markManualComplete(app.id), successMessage: 'Marked complete.');
      }),
    );
  }
}

class _ActionBar extends StatelessWidget {
  final String status;
  final bool acting;
  final VoidCallback onApprove;
  final VoidCallback onReject;
  final VoidCallback onApplyNow;
  final VoidCallback onMarkComplete;
  const _ActionBar({required this.status, required this.acting, required this.onApprove, required this.onReject, required this.onApplyNow, required this.onMarkComplete});

  @override
  Widget build(BuildContext context) {
    List<Widget> buttons;
    switch (status) {
      case 'READY_FOR_REVIEW':
        buttons = [
          Expanded(child: OutlinedButton(onPressed: acting ? null : onReject, child: const Text('Reject'))),
          const SizedBox(width: 12),
          Expanded(child: FilledButton(onPressed: acting ? null : onApprove, child: const Text('Approve'))),
        ];
      case 'APPROVED':
        buttons = [Expanded(child: FilledButton(onPressed: acting ? null : onApplyNow, child: const Text('Apply now')))];
      case 'MANUAL_ACTION_REQUIRED':
        buttons = [Expanded(child: FilledButton(onPressed: acting ? null : onMarkComplete, child: const Text('Mark manual step done')))];
      default:
        return const SizedBox.shrink();
    }
    return SafeArea(
      minimum: const EdgeInsets.fromLTRB(16, 8, 16, 12),
      child: Row(children: buttons),
    );
  }
}

class _Section extends StatelessWidget {
  final String title;
  final Widget child;
  const _Section({required this.title, required this.child});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700)),
          const SizedBox(height: 6),
          child,
        ],
      ),
    );
  }
}

class _BulletList extends StatelessWidget {
  final List<String> items;
  const _BulletList(this.items);

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: items.map((i) => Padding(padding: const EdgeInsets.only(bottom: 4), child: Text('•  $i'))).toList(),
    );
  }
}

class _ChipRow extends StatelessWidget {
  final List<String> items;
  final Color color;
  const _ChipRow(this.items, {required this.color});

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 6,
      runSpacing: 6,
      children: items.map((i) => Chip(label: Text(i, style: const TextStyle(fontSize: 12)), backgroundColor: color.withValues(alpha: 0.12), side: BorderSide.none, visualDensity: VisualDensity.compact)).toList(),
    );
  }
}
