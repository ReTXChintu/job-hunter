import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/labels.dart';
import '../models/envelope.dart';
import '../models/profiles.dart';
import '../state/connection_controller.dart';
import '../state/profiles_controller.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/presence_banner.dart';
import '../widgets/question_form.dart';
import 'application_detail_screen.dart' show openExternal;

Color platformStatusColor(BuildContext context, PlatformProfileView v) => switch (v.profile.status) {
      'SYNCED' => v.outOfDate ? Colors.orange : Colors.green,
      'SYNCING' => Theme.of(context).colorScheme.primary,
      'NEEDS_INPUT' || 'MANUAL_ACTION_REQUIRED' => Colors.orange,
      'FAILED' => Theme.of(context).colorScheme.error,
      _ => Theme.of(context).colorScheme.onSurfaceVariant,
    };

/// More > Job sites: each site's profile state, with Update / Resume /
/// Start over, and the questions the agent stopped on. Runs on the desktop,
/// like its own Job sites page.
class JobSitesScreen extends StatefulWidget {
  const JobSitesScreen({super.key});

  @override
  State<JobSitesScreen> createState() => _JobSitesScreenState();
}

class _JobSitesScreenState extends State<JobSitesScreen> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => context.read<ProfilesController>().refresh());
  }

  @override
  Widget build(BuildContext context) {
    final profiles = context.watch<ProfilesController>();
    Widget body;
    if (profiles.items.isEmpty && profiles.loading) {
      body = const Center(child: CircularProgressIndicator());
    } else if (profiles.items.isEmpty) {
      body = RefreshIndicator(
        onRefresh: profiles.refresh,
        child: LayoutBuilder(
          builder: (context, c) => ListView(physics: const AlwaysScrollableScrollPhysics(), children: [
            SizedBox(
              height: c.maxHeight,
              child: EmptyState(
                icon: profiles.error != null ? Icons.desktop_access_disabled : Icons.language,
                title: profiles.error != null ? 'Job sites need your desktop' : 'No job sites',
                message: profiles.error != null ? '${profiles.error}\n\nJob-site profiles are read from your desktop.' : null,
              ),
            ),
          ]),
        ),
      );
    } else {
      body = RefreshIndicator(
        onRefresh: profiles.refresh,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 24),
          physics: const AlwaysScrollableScrollPhysics(),
          children: [for (final v in profiles.items) _PlatformCard(view: v)],
        ),
      );
    }
    return Scaffold(
      appBar: AppBar(title: const Text('Job sites')),
      body: Column(children: [
        const PresenceBanner(),
        if (profiles.loading && profiles.items.isNotEmpty) const LinearProgressIndicator(minHeight: 2),
        if (profiles.error != null && profiles.items.isNotEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
            color: Theme.of(context).colorScheme.errorContainer,
            child: Text('${profiles.error} Showing the last loaded state.', style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer, fontSize: 13)),
          ),
        Expanded(child: body),
      ]),
    );
  }
}

class _PlatformCard extends StatefulWidget {
  final PlatformProfileView view;
  const _PlatformCard({required this.view});

  @override
  State<_PlatformCard> createState() => _PlatformCardState();
}

class _PlatformCardState extends State<_PlatformCard> {
  bool _acting = false;

  Future<void> _run(Future<RelayResponse> Function() action, String success) async {
    setState(() => _acting = true);
    final resp = await action();
    if (!mounted) return;
    setState(() => _acting = false);
    showOutcome(context, resp, success: success);
  }

  @override
  Widget build(BuildContext context) {
    final v = widget.view;
    final p = v.profile;
    final theme = Theme.of(context);
    final connection = context.watch<ConnectionController>();
    final ctrl = context.read<ProfilesController>();
    final disabledReason = connection.desktopReachable ? null : (connection.connected ? desktopOfflineMessage : notConnectedMessage);
    final enabled = !_acting && disabledReason == null && !p.isSyncing;
    final color = platformStatusColor(context, v);

    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          Row(children: [
            Expanded(child: Text(p.platform, style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700))),
            Tag(platformStatusLabel(v), color: color),
          ]),
          const SizedBox(height: 4),
          Text(
            p.lastSyncedAt != null ? 'Last updated ${formatDateTime(context, p.lastSyncedAt)} (${timeAgo(p.lastSyncedAt)})' : 'Never updated from Job Hunter',
            style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant),
          ),
          if (p.autoSync) Text('Updates automatically when your profile changes', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          if (p.message.isNotEmpty) ...[const SizedBox(height: 8), Text(p.message)],
          if (p.isSyncing) const Padding(padding: EdgeInsets.only(top: 10), child: LinearProgressIndicator()),
          if (p.changes.isNotEmpty)
            Theme(
              data: theme.copyWith(dividerColor: Colors.transparent),
              child: ExpansionTile(
                tilePadding: EdgeInsets.zero,
                childrenPadding: EdgeInsets.zero,
                title: Text('What was changed (${p.changes.length})', style: theme.textTheme.bodyMedium),
                children: [BulletList(p.changes)],
              ),
            ),
          if (p.status == 'NEEDS_INPUT' && p.pendingQuestions.isNotEmpty) ...[
            const SizedBox(height: 10),
            FilledButton.icon(
              onPressed: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => PlatformQuestionsScreen(platform: p.platform))),
              icon: const Icon(Icons.question_answer_outlined),
              label: Text('Answer ${p.pendingQuestions.length} question${p.pendingQuestions.length == 1 ? '' : 's'}'),
            ),
          ],
          const SizedBox(height: 10),
          Wrap(spacing: 8, runSpacing: 8, children: [
            if (p.canResume) ...[
              FilledButton.tonal(onPressed: enabled ? () => _run(() => ctrl.sync(p.platform, resume: true), 'Resuming the ${p.platform} update.') : null, child: const Text('Resume')),
              OutlinedButton(
                onPressed: enabled
                    ? () async {
                        final ok = await confirmDialog(context, title: 'Start over?', message: 'The ${p.platform} update that stopped part-way is discarded and a new one starts.', confirm: 'Start over');
                        if (ok && mounted) await _run(() => ctrl.sync(p.platform), 'Started a fresh ${p.platform} update.');
                      }
                    : null,
                child: const Text('Start over'),
              ),
            ] else
              FilledButton.tonal(onPressed: enabled ? () => _run(() => ctrl.sync(p.platform), 'Updating ${p.platform} on your desktop.') : null, child: const Text('Update')),
            if (p.profileUrl.isNotEmpty) TextButton.icon(onPressed: () => openExternal(context, p.profileUrl), icon: const Icon(Icons.open_in_new, size: 16), label: const Text('Open profile')),
          ]),
          if (disabledReason != null) Padding(padding: const EdgeInsets.only(top: 6), child: Text(disabledReason, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.error))),
        ]),
      ),
    );
  }
}

/// The questions a job-site update stopped on, answered on the phone.
class PlatformQuestionsScreen extends StatelessWidget {
  final String platform;
  const PlatformQuestionsScreen({super.key, required this.platform});

  @override
  Widget build(BuildContext context) {
    final profiles = context.watch<ProfilesController>();
    final connection = context.watch<ConnectionController>();
    final view = profiles.items.where((v) => v.profile.platform == platform).firstOrNull;
    final questions = view?.profile.pendingQuestions ?? const [];
    final disabledReason = connection.desktopReachable ? null : (connection.connected ? desktopOfflineMessage : notConnectedMessage);
    return Scaffold(
      appBar: AppBar(title: Text('$platform questions')),
      body: questions.isEmpty
          ? const EmptyState(icon: Icons.check_circle_outline, title: 'No questions waiting', message: 'The update has everything it needs.')
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                if (view!.profile.message.isNotEmpty) Padding(padding: const EdgeInsets.only(bottom: 12), child: Text(view.profile.message)),
                QuestionForm(
                  questions: questions,
                  disabledReason: disabledReason,
                  submitLabel: 'Send answers & continue',
                  onSubmit: (answers) async {
                    final resp = await profiles.answer(platform, answers);
                    if (!context.mounted) return resp.ok;
                    final ok = showOutcome(context, resp, success: 'Answers sent. Your desktop is continuing the $platform update.');
                    if (ok) Navigator.of(context).pop();
                    return ok;
                  },
                ),
              ],
            ),
    );
  }
}
