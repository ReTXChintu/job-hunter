import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../logic/labels.dart';
import '../models/application.dart';
import '../models/envelope.dart';
import '../models/job.dart';
import '../state/applications_controller.dart';
import '../state/connection_controller.dart';
import '../state/data_source.dart';
import '../theme/theme.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/match_score_bar.dart';
import '../widgets/question_form.dart';
import '../widgets/status_badge.dart';

Future<void> openExternal(BuildContext context, String url) async {
  final uri = Uri.tryParse(url);
  final ok = uri != null && await launchUrl(uri, mode: LaunchMode.externalApplication);
  if (!ok && context.mounted) ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("Couldn't open that link.")));
}

/// One application, in full: where it stands and why, the posting, the
/// match, the employer's replies, and the one thing it needs from you right
/// now (approve, answer questions, finish a manual step, track its outcome).
/// Every action is a request to the desktop, which applies its own approval
/// rules; nothing is decided here.
class ApplicationDetailScreen extends StatefulWidget {
  final String applicationId;
  const ApplicationDetailScreen({super.key, required this.applicationId});

  @override
  State<ApplicationDetailScreen> createState() => _ApplicationDetailScreenState();
}

class _ApplicationDetailScreenState extends State<ApplicationDetailScreen> {
  ApplicationDetail? _detail;
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
    final apps = context.read<ApplicationsController>();
    final detail = await apps.loadDetail(widget.applicationId);
    if (!mounted) return;
    setState(() {
      if (detail != null) _detail = detail;
      _fromServer = apps.lastDetailFromServer;
      _loading = false;
      _error = detail == null ? (apps.lastDetailError ?? 'Could not load this application.') : null;
    });
  }

  Future<bool> _act(Future<RelayResponse> Function() action, String success) async {
    setState(() => _acting = true);
    final resp = await action();
    if (!mounted) return resp.ok;
    setState(() => _acting = false);
    final ok = showOutcome(context, resp, success: success);
    if (ok) await _load();
    return ok;
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) return Scaffold(appBar: AppBar(), body: const Center(child: CircularProgressIndicator()));
    final detail = _detail;
    if (detail == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Application')),
        body: EmptyState(icon: Icons.error_outline, title: 'Could not load this application', message: _error, action: FilledButton(onPressed: _load, child: const Text('Try again'))),
      );
    }

    final connection = context.watch<ConnectionController>();
    final disabledReason = connection.desktopReachable ? null : (connection.connected ? desktopOfflineMessage : notConnectedMessage);
    final app = detail.application;
    final job = detail.job;
    final analysis = detail.analysis;
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(title: Text(job.company.isEmpty ? 'Application' : job.company, maxLines: 1, overflow: TextOverflow.ellipsis)),
      body: Column(
        children: [
          if (_fromServer) const DataSourceBanner(DataSource.server),
          Expanded(
            child: RefreshIndicator(
              onRefresh: _load,
              child: ListView(
                physics: const AlwaysScrollableScrollPhysics(),
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 32),
                children: [
                  _Header(job: job, app: app, analysis: analysis),
                  ..._actionArea(app, job, disabledReason),
                  if (app.failureReason != null && app.status != 'MANUAL_ACTION_REQUIRED')
                    Callout(icon: Icons.error_outline, color: theme.colorScheme.error, title: 'What went wrong', message: app.failureReason),
                  if (app.replies.isNotEmpty) _Replies(app.replies),
                  if (analysis != null) _Analysis(analysis),
                  if (job.description.isNotEmpty) Section(title: 'Job description', child: ExpandableText(job.description)),
                  if (job.requirements.isNotEmpty) Section(title: 'Requirements', child: BulletList(job.requirements)),
                  if (job.responsibilities.isNotEmpty) Section(title: 'Responsibilities', child: BulletList(job.responsibilities)),
                  if (app.answers.isNotEmpty) _AnswersUsed(app.answers),
                  if (detail.resume != null) _ResumeSection(detail.resume!),
                  if (detail.coverLetter != null && detail.coverLetter!.text.isNotEmpty) Section(title: 'Cover letter', child: ExpandableText(detail.coverLetter!.text, collapsedLines: 6)),
                  if (app.potentialIssues.isNotEmpty) Section(title: 'Potential issues', child: BulletList(app.potentialIssues)),
                  if (app.notes.isNotEmpty) Section(title: 'Notes', child: Text(app.notes)),
                  if (app.evidence != null) Section(title: 'Proof of submission', child: Text(app.evidence!)),
                  if (app.statusHistory.isNotEmpty) _History(app.statusHistory),
                ],
              ),
            ),
          ),
        ],
      ),
      bottomNavigationBar: app.status == 'READY_FOR_REVIEW' ? _approvalBar(app, job, disabledReason) : null,
    );
  }

  Widget _approvalBar(Application app, Job job, String? disabledReason) {
    final enabled = !_acting && disabledReason == null;
    return SafeArea(
      minimum: const EdgeInsets.fromLTRB(16, 8, 16, 12),
      child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        if (disabledReason != null) Padding(padding: const EdgeInsets.only(bottom: 8), child: Text(disabledReason, textAlign: TextAlign.center, style: TextStyle(color: Theme.of(context).colorScheme.error))),
        Row(children: [
          Expanded(
            child: OutlinedButton(
              onPressed: enabled
                  ? () async {
                      final reason = await textConfirmDialog(context, title: 'Reject this application?', message: 'Your desktop agent will stop working on ${job.title} at ${job.company}.', label: 'Reason (optional)', confirm: 'Reject');
                      if (reason == null || !mounted) return;
                      await _act(() => context.read<ApplicationsController>().reject(app.id, reason: reason), 'Rejected.');
                    }
                  : null,
              child: const Text('Reject'),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            flex: 2,
            child: FilledButton.icon(
              onPressed: enabled
                  ? () async {
                      final ok = await confirmDialog(context,
                          title: 'Approve and apply?', message: 'Your desktop agent will submit this application to ${job.company}${job.isEmailPost ? ' by email (${job.applyEmail})' : ''}.', confirm: 'Approve & apply');
                      if (!ok || !mounted) return;
                      await _act(() => context.read<ApplicationsController>().approve(app.id), 'Approved. Your desktop is applying.');
                    }
                  : null,
              icon: const Icon(Icons.check),
              label: const Text('Approve & apply'),
            ),
          ),
        ]),
      ]),
    );
  }

  List<Widget> _actionArea(Application app, Job job, String? disabledReason) {
    final apps = context.read<ApplicationsController>();
    final enabled = !_acting && disabledReason == null;
    Widget? disabledNote() => disabledReason == null ? null : Text(disabledReason, style: TextStyle(color: Theme.of(context).colorScheme.error));

    switch (app.status) {
      case 'READY_FOR_REVIEW':
        return [
          Callout(
            icon: Icons.rule,
            color: Theme.of(context).colorScheme.primary,
            title: 'Waiting for your approval',
            message: 'Review the match and the prepared resume below. Nothing is sent until you approve.',
          ),
        ];
      case 'WAITING_FOR_USER':
        if (app.pendingQuestions.isEmpty) {
          return [
            Callout(
              icon: Icons.help_outline,
              color: Colors.orange,
              title: 'The agent needs your input',
              message: app.failureReason ?? 'It stopped to ask something. Try again to let it continue.',
              child: FilledButton(onPressed: enabled ? () => _act(() => apps.applyNow(app.id), 'Retrying on your desktop.') : null, child: const Text('Try again')),
            ),
          ];
        }
        return [
          Callout(
            icon: Icons.help_outline,
            color: Colors.orange,
            title: 'Answer ${app.pendingQuestions.length == 1 ? 'this question' : 'these ${app.pendingQuestions.length} questions'} to continue',
            message: app.failureReason,
            child: QuestionForm(
              key: ValueKey(app.pendingQuestions.map((q) => q.key).join('|')),
              questions: app.pendingQuestions,
              disabledReason: disabledReason,
              submitLabel: 'Send answers & continue',
              onSubmit: (answers) => _act(() => apps.answerQuestions(app.id, answers), 'Answers sent. Your desktop is continuing the application.'),
            ),
          ),
        ];
      case 'MANUAL_ACTION_REQUIRED':
        return [
          Callout(
            icon: Icons.back_hand_outlined,
            color: Colors.deepOrange,
            title: 'Needs a manual step',
            message: app.failureReason ?? 'The agent could not finish this application on its own.',
            child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              if (app.applicationUrl.isNotEmpty)
                TextButton.icon(onPressed: () => openExternal(context, app.applicationUrl), icon: const Icon(Icons.open_in_new, size: 18), label: const Text('Open the application page')),
              Row(children: [
                Expanded(child: OutlinedButton(onPressed: enabled ? () => _act(() => apps.applyNow(app.id), 'Retrying on your desktop.') : null, child: const Text('Retry / resume'))),
                const SizedBox(width: 8),
                Expanded(
                  child: FilledButton(
                    onPressed: enabled
                        ? () async {
                            final note = await textConfirmDialog(context, title: 'Mark as applied?', message: 'Confirm you finished and submitted this application yourself.', label: 'Note (optional)', confirm: 'Mark applied');
                            if (note == null || !mounted) return;
                            await _act(() => apps.markManualComplete(app.id, note: note), 'Marked as applied.');
                          }
                        : null,
                    child: const Text('Mark as applied'),
                  ),
                ),
              ]),
              ?disabledNote(),
            ]),
          ),
        ];
      case 'APPROVED':
        return [
          Callout(
            icon: Icons.schedule_send,
            color: Theme.of(context).colorScheme.primary,
            title: 'Approved',
            message: 'Your desktop applies to approved jobs. Apply now if it hasn\'t started.',
            child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              FilledButton(onPressed: enabled ? () => _act(() => apps.applyNow(app.id), 'Applying now on your desktop.') : null, child: const Text('Apply now')),
              ?disabledNote(),
            ]),
          ),
        ];
      case 'APPLYING':
        return [Callout(icon: Icons.autorenew, color: Theme.of(context).colorScheme.primary, title: 'Applying now', message: 'Your desktop agent is filling in this application.')];
      case 'APPLIED':
      case 'INTERVIEW':
        final options = [if (app.status == 'APPLIED') 'INTERVIEW', 'OFFER', 'REJECTED', 'WITHDRAWN'];
        return [
          Section(
            title: 'Track this application',
            child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              Wrap(spacing: 8, runSpacing: 8, children: [
                for (final s in options)
                  OutlinedButton(
                    onPressed: enabled
                        ? () async {
                            final note = await textConfirmDialog(context, title: 'Mark as ${statusLabel(s).toLowerCase()}?', message: 'Updates this application\'s status on your desktop.', label: 'Note (optional)', confirm: 'Update');
                            if (note == null || !mounted) return;
                            await _act(() => apps.setStatus(app.id, s, note: note), 'Status updated.');
                          }
                        : null,
                    child: Text(statusLabel(s)),
                  ),
              ]),
              if (disabledReason != null) Padding(padding: const EdgeInsets.only(top: 6), child: disabledNote()),
            ]),
          ),
        ];
      default:
        return const [];
    }
  }
}

class _Header extends StatelessWidget {
  final Job job;
  final Application app;
  final JobAnalysis? analysis;
  const _Header({required this.job, required this.app, required this.analysis});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final facts = [job.employmentType, job.remote, job.seniority].whereType<String>().where((s) => s.isNotEmpty).map((s) => s.replaceAll('_', ' ').toLowerCase()).toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(job.title.isEmpty ? 'Untitled job' : job.title, style: theme.textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700)),
        const SizedBox(height: 4),
        Text(job.companyLine, style: theme.textTheme.titleMedium),
        if (job.salary != null) Text(job.salary!, style: theme.textTheme.bodyMedium),
        if (facts.isNotEmpty) Text(facts.join(' · '), style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
        const SizedBox(height: 10),
        Wrap(spacing: 8, runSpacing: 6, crossAxisAlignment: WrapCrossAlignment.center, children: [
          StatusBadge(status: app.status),
          MatchScoreBar(score: analysis?.matchScore, relevant: analysis?.relevant),
          if (job.source.isNotEmpty) Tag(job.source, color: theme.colorScheme.secondary),
        ]),
        if (job.isEmailPost) ...[
          const SizedBox(height: 10),
          Row(children: [
            const Icon(Icons.mail_outline, size: 18, color: Colors.indigo),
            const SizedBox(width: 6),
            Expanded(child: Text('Applies by email to ${job.contactName != null ? '${job.contactName} · ' : ''}${job.applyEmail}', style: theme.textTheme.bodyMedium)),
          ]),
        ],
        const SizedBox(height: 4),
        Wrap(spacing: 4, children: [
          if (job.url.isNotEmpty) TextButton.icon(onPressed: () => openExternal(context, job.url), icon: const Icon(Icons.open_in_new, size: 16), label: const Text('View posting')),
          if (app.applicationUrl.isNotEmpty && app.applicationUrl != job.url)
            TextButton.icon(onPressed: () => openExternal(context, app.applicationUrl), icon: const Icon(Icons.link, size: 16), label: const Text('Application page')),
        ]),
        if (app.appliedAt != null) Text('Applied ${formatDateTime(context, app.appliedAt)}', style: theme.textTheme.bodySmall),
      ],
    );
  }
}

class _Replies extends StatelessWidget {
  final List<EmailReply> replies;
  const _Replies(this.replies);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sorted = [...replies]..sort((a, b) => (b.receivedAt ?? DateTime(0)).compareTo(a.receivedAt ?? DateTime(0)));
    return Section(
      title: 'Replies from the employer (${replies.length})',
      child: Column(children: [
        for (final r in sorted)
          Card(
            margin: const EdgeInsets.only(bottom: 8),
            color: theme.colorScheme.surfaceContainerLow,
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
                Wrap(spacing: 6, runSpacing: 4, children: [
                  Tag(replyKindLabel(r.kind), color: switch (r.kind.toUpperCase()) { 'INTERVIEW' || 'OFFER' => Colors.green, 'REJECTION' => Colors.redAccent, 'ASSESSMENT' || 'QUESTION' => Colors.orange, _ => Colors.blueGrey }),
                  if (r.inSpam) const Tag('In Spam', color: Colors.red, icon: Icons.report_outlined),
                ]),
                const SizedBox(height: 6),
                Text(r.subject.isEmpty ? '(no subject)' : r.subject, style: theme.textTheme.titleSmall),
                Text([r.from, formatDateTime(context, r.receivedAt)].where((s) => s.isNotEmpty).join(' · '), style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                if (r.summary.isNotEmpty) ...[const SizedBox(height: 6), Text(r.summary)],
              ]),
            ),
          ),
      ]),
    );
  }
}

class _Analysis extends StatelessWidget {
  final JobAnalysis analysis;
  const _Analysis(this.analysis);

  @override
  Widget build(BuildContext context) {
    Widget check(String label, bool ok) => Row(mainAxisSize: MainAxisSize.min, children: [
          Icon(ok ? Icons.check_circle : Icons.cancel, size: 16, color: ok ? Colors.green : Colors.orange),
          const SizedBox(width: 4),
          Text(label),
        ]);
    return Section(
      title: 'Match analysis',
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text(analysis.summary.isEmpty ? 'No summary provided.' : analysis.summary),
        const SizedBox(height: 8),
        Wrap(spacing: 14, runSpacing: 6, children: [
          check('Experience', analysis.requiredExperienceMet),
          check('Seniority', analysis.seniorityMatch),
          check('Location', analysis.locationMatch),
        ]),
        if (analysis.salaryAssessment.isNotEmpty) ...[const SizedBox(height: 8), Text('Salary: ${analysis.salaryAssessment}')],
        if (analysis.matchedSkills.isNotEmpty) ...[const SizedBox(height: 12), const Text('Matched skills'), const SizedBox(height: 4), ChipRow(analysis.matchedSkills, color: Colors.green)],
        if (analysis.missingSkills.isNotEmpty) ...[const SizedBox(height: 12), const Text('Missing skills'), const SizedBox(height: 4), ChipRow(analysis.missingSkills, color: Colors.orange)],
        if (analysis.concerns.isNotEmpty) ...[const SizedBox(height: 12), const Text('Concerns'), const SizedBox(height: 4), BulletList(analysis.concerns)],
      ]),
    );
  }
}

class _AnswersUsed extends StatelessWidget {
  final List<ApplicationAnswer> answers;
  const _AnswersUsed(this.answers);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Section(
      title: 'Answers used',
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        for (final a in answers)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(a.question, style: theme.textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600)),
              Text(a.answer.isEmpty ? '(skipped)' : a.answer),
              if (a.source.isNotEmpty) Text(switch (a.source) { 'USER' => 'You answered', 'PROFILE' => 'From your profile', 'KNOWN' => 'Saved answer', 'SKIPPED' => 'Skipped', _ => a.source }, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
            ]),
          ),
      ]),
    );
  }
}

class _ResumeSection extends StatelessWidget {
  final ResumeInfo resume;
  const _ResumeSection(this.resume);

  @override
  Widget build(BuildContext context) {
    final v = resume.validation;
    final theme = Theme.of(context);
    return Section(
      title: 'Tailored resume',
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text(resume.label.isEmpty ? 'Prepared for this job.' : resume.label),
        if (v != null) ...[
          const SizedBox(height: 6),
          Text('ATS check: ${v.status} · ${v.keywordCoverage}% keyword coverage'),
          if (v.missingKeywords.isNotEmpty) ...[const SizedBox(height: 6), const Text('Missing keywords'), const SizedBox(height: 4), ChipRow(v.missingKeywords, color: Colors.orange)],
        ],
        if (resume.filesDeletedAt != null)
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Text('Its files were deleted after applying; the content is kept on your desktop.', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          ),
      ]),
    );
  }
}

class _History extends StatelessWidget {
  final List<StatusChange> history;
  const _History(this.history);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sorted = [...history]..sort((a, b) => (b.at ?? DateTime(0)).compareTo(a.at ?? DateTime(0)));
    return Section(
      title: 'History',
      child: Column(children: [
        for (final h in sorted)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Padding(padding: const EdgeInsets.only(top: 5), child: Icon(Icons.circle, size: 8, color: statusColor(context, h.status))),
              const SizedBox(width: 10),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text(statusLabel(h.status), style: theme.textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600)),
                  if (h.at != null) Text(formatDateTime(context, h.at), style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                  if (h.reason.isNotEmpty) Text(h.reason, style: theme.textTheme.bodySmall),
                ]),
              ),
            ]),
          ),
      ]),
    );
  }
}
