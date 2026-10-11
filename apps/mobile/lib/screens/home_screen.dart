import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/filters.dart';
import '../logic/labels.dart';
import '../models/application.dart';
import '../models/envelope.dart';
import '../state/agent_controller.dart';
import '../state/applications_controller.dart';
import '../state/connection_controller.dart';
import '../state/nav_controller.dart';
import '../state/queue_controller.dart';
import '../theme/theme.dart';
import '../widgets/common.dart';
import '../widgets/presence_banner.dart';
import 'application_detail_screen.dart';
import 'queue_screen.dart';

/// The Home tab: what the agent is doing and what's queued behind it, what's
/// waiting on you, the numbers, and everything the desktop can be asked to
/// start from here.
class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  Future<void> _refresh(BuildContext context) async {
    final apps = context.read<ApplicationsController>();
    final agent = context.read<AgentController>();
    final queue = context.read<QueueController>();
    await Future.wait([apps.refresh(), agent.refreshStatus(), queue.refresh()]);
  }

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();
    return Scaffold(
      body: RefreshIndicator(
        onRefresh: () => _refresh(context),
        edgeOffset: 80,
        child: CustomScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          slivers: [
            const SliverToBoxAdapter(child: _Hero()),
            SliverToBoxAdapter(child: DataSourceBanner(apps.source)),
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 4, 16, 28),
              sliver: SliverList.list(children: const [
                _QueueCard(),
                _WaitingOnYou(),
                _Counts(),
                _Actions(),
              ]),
            ),
          ],
        ),
      ),
    );
  }
}

/// The brand header: greeting, connection, and the agent's state.
class _Hero extends StatelessWidget {
  const _Hero();

  String _greeting() {
    final h = DateTime.now().hour;
    if (h < 5) return 'Still up?';
    if (h < 12) return 'Good morning';
    if (h < 17) return 'Good afternoon';
    return 'Good evening';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final top = MediaQuery.paddingOf(context).top;
    return Container(
      decoration: BoxDecoration(gradient: brandGradient(theme.brightness)),
      padding: EdgeInsets.fromLTRB(20, top + 14, 20, 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(children: [
            const BrandLogo(size: 28),
            const SizedBox(width: 10),
            const Expanded(child: Text('Job Hunter', style: TextStyle(color: Colors.white, fontWeight: FontWeight.w800, fontSize: 17, letterSpacing: -0.2))),
            const ConnectionPill(onDark: true),
          ]),
          const SizedBox(height: 18),
          Text(_greeting(), style: theme.textTheme.headlineSmall?.copyWith(color: Colors.white, fontWeight: FontWeight.w800, letterSpacing: -0.5)),
          const SizedBox(height: 14),
          const _AgentPanel(),
        ],
      ),
    );
  }
}

/// The agent's state on the hero: label, run kind, progress, queued count,
/// and Stop while it's working.
class _AgentPanel extends StatelessWidget {
  const _AgentPanel();

  @override
  Widget build(BuildContext context) {
    final connection = context.watch<ConnectionController>();
    final agent = context.watch<AgentController>();
    final status = connection.agentStatus;
    final theme = Theme.of(context);
    final String title;
    String? subtitle;
    var icon = Icons.smart_toy_outlined;
    var busy = false;
    if (!connection.desktopReachable) {
      title = 'Agent status unknown';
      subtitle = 'Shown when your desktop is online.';
      icon = Icons.desktop_access_disabled;
    } else if (status == null) {
      title = 'Checking the agent…';
    } else {
      title = agentStateLabel(status.state);
      busy = agentBusy(status.state);
      final parts = [
        if (status.runKind != null && busy) runKindLabel(status.runKind!),
        if (status.paused) 'Paused',
        if (status.queued > 0) '${status.queued} queued',
      ];
      subtitle = parts.isEmpty ? null : parts.join(' · ');
      icon = switch (status.state) {
        'FAILED' => Icons.error_outline,
        'COMPLETED' => Icons.check_circle_outline,
        'WAITING_FOR_APPROVAL' || 'WAITING_FOR_USER' || 'MANUAL_ACTION_REQUIRED' => Icons.pan_tool_outlined,
        _ => busy ? Icons.autorenew : Icons.smart_toy_outlined,
      };
    }
    final progress = status?.progress;
    final canStop = busy && agent.unavailableReason == null && status?.state != 'STOPPING' && agent.running == null;
    final white = Colors.white;
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(color: white.withValues(alpha: 0.14), borderRadius: BorderRadius.circular(16), border: Border.all(color: white.withValues(alpha: 0.18))),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(children: [
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(color: white.withValues(alpha: 0.18), shape: BoxShape.circle),
              child: busy && agent.running == null
                  ? Padding(padding: const EdgeInsets.all(11), child: CircularProgressIndicator(strokeWidth: 2.2, color: white, value: progress == null ? null : (progress > 1 ? progress / 100 : progress).clamp(0.0, 1.0)))
                  : Icon(icon, color: white, size: 22),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text('Agent', style: theme.textTheme.labelSmall?.copyWith(color: white.withValues(alpha: 0.75), fontWeight: FontWeight.w700, letterSpacing: 0.6)),
                Text(title, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.titleMedium?.copyWith(color: white, fontWeight: FontWeight.w700)),
                if (subtitle != null) Text(subtitle, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodySmall?.copyWith(color: white.withValues(alpha: 0.85))),
              ]),
            ),
            if (canStop)
              IconButton.filledTonal(
                tooltip: 'Stop',
                style: IconButton.styleFrom(backgroundColor: white.withValues(alpha: 0.18), foregroundColor: white),
                onPressed: () async {
                  final resp = await agent.stop();
                  if (context.mounted) showOutcome(context, resp, success: 'Asked the agent to stop.');
                },
                icon: agent.running == 'stop' ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white)) : const Icon(Icons.stop),
              ),
          ]),
          if (busy && progress != null) ...[
            const SizedBox(height: 12),
            ClipRRect(
              borderRadius: BorderRadius.circular(4),
              child: LinearProgressIndicator(minHeight: 5, value: (progress > 1 ? progress / 100 : progress).clamp(0.0, 1.0), color: white, backgroundColor: white.withValues(alpha: 0.2)),
            ),
          ],
        ],
      ),
    );
  }
}

/// What's waiting for the agent, in order. Hidden when nothing is.
class _QueueCard extends StatelessWidget {
  const _QueueCard();

  @override
  Widget build(BuildContext context) {
    final queue = context.watch<QueueController>();
    final theme = Theme.of(context);
    final items = queue.items;
    if (items.isEmpty && queue.count == 0) return const SizedBox.shrink();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SectionHeader(
          'Up next',
          subtitle: 'Starts by itself when the agent is free',
          action: TextButton(onPressed: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => const QueueScreen())), child: const Text('See all')),
        ),
        AppCard(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: items.isEmpty
              ? Padding(
                  padding: const EdgeInsets.all(12),
                  child: Row(children: [
                    const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2)),
                    const SizedBox(width: 12),
                    Text('${queue.count} queued · loading…', style: theme.textTheme.bodyMedium),
                  ]),
                )
              : Column(children: [
                  for (final (i, item) in items.take(3).indexed) QueueRow(index: i + 1, item: item, compact: true),
                  if (items.length > 3)
                    Padding(
                      padding: const EdgeInsets.fromLTRB(16, 4, 16, 8),
                      child: Text('and ${items.length - 3} more', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                    ),
                ]),
        ),
      ],
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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SectionHeader(
          'Waiting on you',
          subtitle: '${waiting.length} application${waiting.length == 1 ? '' : 's'} need${waiting.length == 1 ? 's' : ''} a decision or an answer',
          action: TextButton(onPressed: () => context.read<NavController>().openApplications(ApplicationBucket.needsYou), child: const Text('See all')),
        ),
        for (final item in waiting.take(4))
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: _WaitingRow(item: item),
          ),
      ],
    );
  }
}

class _WaitingRow extends StatelessWidget {
  final ApplicationListItem item;
  const _WaitingRow({required this.item});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final status = item.application.status;
    final color = statusColor(context, status);
    final questions = item.application.pendingQuestions.length;
    final hint = switch (status) {
      'READY_FOR_REVIEW' => 'Review and approve',
      'WAITING_FOR_USER' => questions > 0 ? 'Answer $questions question${questions == 1 ? '' : 's'}' : 'Needs your input',
      _ => 'Finish a manual step',
    };
    return AppCard(
      onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => ApplicationDetailScreen(applicationId: item.application.id))),
      padding: const EdgeInsets.fromLTRB(14, 12, 10, 12),
      child: Row(children: [
        IconBubble(switch (status) { 'READY_FOR_REVIEW' => Icons.rule, 'WAITING_FOR_USER' => Icons.help_outline, _ => Icons.back_hand_outlined }, color: color, size: 38),
        const SizedBox(width: 12),
        Expanded(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(item.job.title, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700)),
            Text(item.job.companyLine, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
            const SizedBox(height: 2),
            Text(hint, style: theme.textTheme.bodySmall?.copyWith(color: color, fontWeight: FontWeight.w700)),
          ]),
        ),
        Icon(Icons.chevron_right, color: theme.colorScheme.onSurfaceVariant),
      ]),
    );
  }
}

class _Counts extends StatelessWidget {
  const _Counts();

  @override
  Widget build(BuildContext context) {
    final apps = context.watch<ApplicationsController>();
    final nav = context.read<NavController>();
    final theme = Theme.of(context);
    final c = HomeCounts.from(apps.items);
    final tiles = [
      ('Pending approval', c.pendingApproval, Icons.rule, theme.colorScheme.primary, ApplicationBucket.pendingApproval),
      ('Needs your input', c.needsInput, Icons.help_outline, Colors.orange, ApplicationBucket.needsYou),
      ('Manual action', c.manualAction, Icons.back_hand_outlined, Colors.deepOrange, ApplicationBucket.needsYou),
      ('Applied', c.applied, Icons.send_outlined, Colors.green, ApplicationBucket.applied),
      ('Interviews', c.interviews, Icons.event_available_outlined, Colors.teal, ApplicationBucket.interviewsOffers),
      ('Offers', c.offers, Icons.celebration_outlined, Colors.purple, ApplicationBucket.interviewsOffers),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SectionHeader('Applications', subtitle: '${apps.items.length} in total', action: TextButton(onPressed: () => nav.openApplications(), child: const Text('Open'))),
        if (apps.items.isEmpty && apps.loading)
          const Padding(padding: EdgeInsets.all(24), child: Center(child: CircularProgressIndicator()))
        else if (apps.items.isEmpty && apps.error != null)
          AppCard(child: Text(apps.error!, style: TextStyle(color: theme.colorScheme.error)))
        else
          GridView.count(
            crossAxisCount: 3,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            mainAxisSpacing: 10,
            crossAxisSpacing: 10,
            childAspectRatio: 0.95,
            children: [
              for (final (label, count, icon, color, bucket) in tiles) StatTile(label: label, count: count, icon: icon, color: color, onTap: () => nav.openApplications(bucket)),
            ],
          ),
      ],
    );
  }
}

/// Everything the desktop can be asked to start from the phone.
class _Actions extends StatelessWidget {
  const _Actions();

  Future<void> _go(BuildContext context, Future<RelayResponse> Function() action, String success) async {
    final resp = await action();
    if (context.mounted) showOutcome(context, resp, success: success);
  }

  Future<void> _link(BuildContext context, AgentController agent) async {
    final url = await textConfirmDialog(
      context,
      title: 'Apply from a link',
      message: 'Paste a job link from any site Job Hunter knows. Your desktop opens it, prepares the resume, then approves and applies.',
      label: 'Job link',
      hint: 'https://…',
      confirm: 'Start',
      required: true,
      maxLines: 2,
      keyboardType: TextInputType.url,
    );
    if (url == null || url.isEmpty || !context.mounted) return;
    await _go(context, () => agent.applyFromLink(url), 'Your desktop is opening the link and preparing the application.');
  }

  Future<void> _shared(BuildContext context, AgentController agent) async {
    final text = await textConfirmDialog(
      context,
      title: 'Add a job I found',
      message: 'Paste the job post. Your desktop reads it, tailors a resume and saves the application email as a Gmail draft for you to send.',
      label: 'Job post',
      hint: 'Paste the posting text…',
      confirm: 'Prepare',
      required: true,
      maxLines: 8,
    );
    if (text == null || text.isEmpty || !context.mounted) return;
    if (text.trim().length < 30) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Paste a bit more of the posting (at least a few lines).')));
      return;
    }
    await _go(context, () => agent.addSharedJob(text), 'Your desktop is reading the post and preparing the application.');
  }

  @override
  Widget build(BuildContext context) {
    final agent = context.watch<AgentController>();
    final theme = Theme.of(context);
    final unavailable = agent.unavailableReason;
    final running = agent.running;
    final can = unavailable == null && running == null;

    Widget tile(String key, String title, String subtitle, IconData icon, Color color, Future<RelayResponse> Function() action, String success, {bool primary = false}) => ActionTile(
          title: title,
          subtitle: subtitle,
          icon: icon,
          color: color,
          primary: primary,
          busy: running == key,
          onTap: can ? () => _go(context, action, success) : null,
        );

    final note = unavailable ?? (agent.busy ? 'The agent is busy${agent.queued > 0 ? ' with ${agent.queued} more queued' : ''}. Anything you start now waits its turn and runs by itself.' : null);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SectionHeader('Run on your desktop', subtitle: 'Each one runs on your computer, exactly like its own buttons'),
        GridView.count(
          crossAxisCount: 2,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          mainAxisSpacing: 10,
          crossAxisSpacing: 10,
          childAspectRatio: 1.25,
          children: [
            tile('start', 'Start job hunt', 'Search, score and prepare applications', Icons.play_arrow_rounded, theme.colorScheme.primary, agent.startJobHunt, 'Job hunt started on your desktop.', primary: true),
            tile('prepare', 'Prepare applications', 'Resumes for the best jobs already found', Icons.auto_awesome_outlined, Colors.purple, agent.prepareApplications, 'Preparing applications on your desktop.'),
            tile('search', 'Search only', 'Find and score jobs, nothing prepared', Icons.search, Colors.blue, agent.searchOnly, 'Searching your job sites.'),
            tile('posts', 'Find hiring posts', 'LinkedIn posts asking for resumes', Icons.campaign_outlined, Colors.indigo, agent.findHiringPosts, 'Looking for LinkedIn hiring posts.'),
            tile('telegram', 'Scan Telegram', 'Your channels since the last scan', Icons.send_outlined, Colors.lightBlue, agent.scanTelegram, 'Scanning your Telegram channels.'),
            tile('inbox', 'Check Gmail', 'Replies from employers, spam included', Icons.mark_email_unread_outlined, Colors.teal, agent.checkInbox, 'Checking Gmail for replies.'),
            ActionTile(title: 'Apply from a link', subtitle: 'Paste a job link and apply', icon: Icons.link, color: Colors.deepOrange, busy: running == 'link', onTap: can ? () => _link(context, agent) : null),
            ActionTile(title: 'Add a job I found', subtitle: 'Paste a post; get a resume and a draft', icon: Icons.post_add_outlined, color: Colors.green, busy: running == 'shared', onTap: can ? () => _shared(context, agent) : null),
          ],
        ),
        if (note != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(4, 12, 4, 0),
            child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Icon(unavailable != null ? Icons.info_outline : Icons.schedule, size: 16, color: theme.colorScheme.onSurfaceVariant),
              const SizedBox(width: 8),
              Expanded(child: Text(note, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant))),
            ]),
          ),
      ],
    );
  }
}
