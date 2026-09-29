import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/filters.dart';
import '../logic/labels.dart';
import '../models/envelope.dart';
import '../state/agent_controller.dart';
import '../state/applications_controller.dart';
import '../state/connection_controller.dart';
import '../state/nav_controller.dart';
import '../widgets/common.dart';
import '../widgets/presence_banner.dart';
import '../widgets/status_badge.dart';
import 'application_detail_screen.dart';

/// The Home tab: is the desktop there, what is the agent doing, what's
/// waiting on you, and the buttons to set the agent going.
class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  Future<void> _refresh(BuildContext context) async {
    final apps = context.read<ApplicationsController>();
    final agent = context.read<AgentController>();
    await Future.wait([apps.refresh(), agent.refreshStatus()]);
  }

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();
    return Scaffold(
      appBar: AppBar(
        title: const Row(children: [BrandLogo(size: 30), SizedBox(width: 10), Text('Job Hunter')]),
      ),
      body: Column(
        children: [
          const PresenceBanner(),
          DataSourceBanner(apps.source),
          Expanded(
            child: RefreshIndicator(
              onRefresh: () => _refresh(context),
              child: ListView(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
                children: const [
                  _AgentCard(),
                  SizedBox(height: 16),
                  _Counts(),
                  SizedBox(height: 16),
                  _QuickActions(),
                  SizedBox(height: 8),
                  _WaitingOnYou(),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _AgentCard extends StatelessWidget {
  const _AgentCard();

  @override
  Widget build(BuildContext context) {
    final connection = context.watch<ConnectionController>();
    final status = connection.agentStatus;
    final theme = Theme.of(context);
    final String title;
    String? subtitle;
    var color = theme.colorScheme.onSurfaceVariant;
    var icon = Icons.smart_toy_outlined;
    if (!connection.desktopReachable) {
      title = 'Agent status unknown';
      subtitle = 'Shown when your desktop is online.';
    } else if (status == null) {
      title = 'Checking the agent…';
    } else {
      title = agentStateLabel(status.state);
      final parts = [
        if (status.runKind != null && agentBusy(status.state)) runKindLabel(status.runKind!),
        if (status.paused) 'Paused',
      ];
      subtitle = parts.isEmpty ? null : parts.join(' · ');
      if (agentBusy(status.state)) {
        color = theme.colorScheme.primary;
        icon = Icons.autorenew;
      } else if (status.state == 'FAILED') {
        color = theme.colorScheme.error;
        icon = Icons.error_outline;
      } else if (status.state == 'COMPLETED') {
        color = Colors.green;
        icon = Icons.check_circle_outline;
      }
    }
    final progress = status?.progress;
    return Card(
      color: theme.colorScheme.surfaceContainerLow,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(children: [
              Icon(icon, color: color),
              const SizedBox(width: 12),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text('Agent', style: theme.textTheme.labelMedium?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                  Text(title, style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w600)),
                  if (subtitle != null) Text(subtitle, style: theme.textTheme.bodySmall),
                ]),
              ),
            ]),
            if (connection.desktopReachable && status != null && agentBusy(status.state)) ...[
              const SizedBox(height: 12),
              LinearProgressIndicator(value: progress == null ? null : (progress > 1 ? progress / 100 : progress).clamp(0.0, 1.0)),
            ],
          ],
        ),
      ),
    );
  }
}

class _Counts extends StatelessWidget {
  const _Counts();

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();
    final nav = context.read<NavController>();
    final c = HomeCounts.from(apps.items);
    final tiles = [
      ('Pending approval', c.pendingApproval, Theme.of(context).colorScheme.primary, ApplicationBucket.pendingApproval),
      ('Needs your input', c.needsInput, Colors.orange, ApplicationBucket.needsYou),
      ('Manual action', c.manualAction, Colors.deepOrange, ApplicationBucket.needsYou),
      ('Applied', c.applied, Colors.green, ApplicationBucket.applied),
      ('Interviews', c.interviews, Colors.teal, ApplicationBucket.interviewsOffers),
      ('Offers', c.offers, Colors.purple, ApplicationBucket.interviewsOffers),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Applications', style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700)),
        const SizedBox(height: 8),
        if (apps.items.isEmpty && apps.loading)
          const Padding(padding: EdgeInsets.all(24), child: Center(child: CircularProgressIndicator()))
        else if (apps.items.isEmpty && apps.error != null)
          Text(apps.error!, style: TextStyle(color: Theme.of(context).colorScheme.error))
        else
          GridView.count(
            crossAxisCount: 3,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            mainAxisSpacing: 8,
            crossAxisSpacing: 8,
            childAspectRatio: 1.15,
            children: [
              for (final (label, count, color, bucket) in tiles)
                Material(
                  color: color.withValues(alpha: count > 0 ? 0.12 : 0.05),
                  borderRadius: BorderRadius.circular(12),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(12),
                    onTap: () => nav.openApplications(bucket),
                    child: Padding(
                      padding: const EdgeInsets.all(10),
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Text('$count', style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w700, color: count > 0 ? color : null)),
                          const SizedBox(height: 2),
                          Text(label, textAlign: TextAlign.center, maxLines: 2, style: Theme.of(context).textTheme.bodySmall),
                        ],
                      ),
                    ),
                  ),
                ),
            ],
          ),
      ],
    );
  }
}

class _QuickActions extends StatelessWidget {
  const _QuickActions();

  Future<void> _go(BuildContext context, Future<RelayResponse> Function() action, String success) async {
    final resp = await action();
    if (context.mounted) showOutcome(context, resp, success: success);
  }

  @override
  Widget build(BuildContext context) {
    final agent = context.watch<AgentController>();
    final theme = Theme.of(context);
    final unavailable = agent.unavailableReason;
    final busy = agent.busy;
    final running = agent.running;
    final canStart = unavailable == null && !busy && running == null;
    final canStop = unavailable == null && busy && agent.status?.state != 'STOPPING' && running == null;

    Widget spinnerOr(String name, IconData icon) => running == name ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2)) : Icon(icon);

    String? note = unavailable;
    if (note == null && busy) note = 'The agent is busy (${agentStateLabel(agent.status!.state)}). New runs start when it finishes, or stop it first.';

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text('Run on your desktop', style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700)),
            const SizedBox(height: 12),
            Row(children: [
              Expanded(
                child: FilledButton.icon(
                  onPressed: canStart ? () => _go(context, agent.startJobHunt, 'Job hunt started on your desktop.') : null,
                  icon: spinnerOr('start', Icons.play_arrow),
                  label: const Text('Start job hunt'),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: canStop ? () => _go(context, agent.stop, 'Asked the agent to stop.') : null,
                  icon: spinnerOr('stop', Icons.stop),
                  label: const Text('Stop'),
                ),
              ),
            ]),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed: canStart ? () => _go(context, agent.findHiringPosts, 'Looking for LinkedIn hiring posts.') : null,
              icon: spinnerOr('posts', Icons.campaign_outlined),
              label: const Text('Find hiring posts'),
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed: canStart ? () => _go(context, agent.checkInbox, 'Checking Gmail for replies.') : null,
              icon: spinnerOr('inbox', Icons.mark_email_unread_outlined),
              label: const Text('Check Gmail for replies'),
            ),
            if (note != null) ...[
              const SizedBox(height: 10),
              Text(note, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
            ],
          ],
        ),
      ),
    );
  }
}

class _WaitingOnYou extends StatelessWidget {
  const _WaitingOnYou();

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();
    final waiting = waitingOnYou(apps.items);
    if (waiting.isEmpty) return const SizedBox.shrink();
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SizedBox(height: 8),
        Row(children: [
          Expanded(child: Text('Waiting on you (${waiting.length})', style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700))),
          TextButton(onPressed: () => context.read<NavController>().openApplications(ApplicationBucket.needsYou), child: const Text('See all')),
        ]),
        Card(
          clipBehavior: Clip.antiAlias,
          child: Column(children: [
            for (final item in waiting.take(5))
              ListTile(
                title: Text(item.job.title, maxLines: 1, overflow: TextOverflow.ellipsis),
                subtitle: Text(item.job.companyLine, maxLines: 1, overflow: TextOverflow.ellipsis),
                trailing: StatusBadge(status: item.application.status),
                onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => ApplicationDetailScreen(applicationId: item.application.id))),
              ),
          ]),
        ),
      ],
    );
  }
}
