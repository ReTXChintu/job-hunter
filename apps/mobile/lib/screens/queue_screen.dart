import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/queue.dart';
import '../state/connection_controller.dart';
import '../state/queue_controller.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/presence_banner.dart';

/// Everything waiting for the desktop's agent, first to run first, with
/// remove and clear. The desktop starts them by itself as it frees up.
class QueueScreen extends StatelessWidget {
  const QueueScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final queue = context.watch<QueueController>();
    final connection = context.watch<ConnectionController>();
    final items = queue.items;
    final theme = Theme.of(context);

    Widget body;
    if (items.isEmpty && queue.loading) {
      body = const Center(child: CircularProgressIndicator());
    } else if (items.isEmpty) {
      body = RefreshIndicator(
        onRefresh: queue.refresh,
        child: LayoutBuilder(
          builder: (context, c) => ListView(physics: const AlwaysScrollableScrollPhysics(), children: [
            SizedBox(
              height: c.maxHeight,
              child: EmptyState(
                icon: queue.error != null ? Icons.error_outline : Icons.playlist_add_check_outlined,
                title: queue.error != null ? 'Could not read the queue' : 'Nothing queued',
                message: queue.error ??
                    (connection.desktopReachable
                        ? 'Start anything while the agent is busy and it lines up here, then runs by itself as soon as the agent is free.'
                        : 'The queue lives on your desktop; it shows here while the desktop is online.'),
              ),
            ),
          ]),
        ),
      );
    } else {
      body = RefreshIndicator(
        onRefresh: queue.refresh,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
          children: [
            AppCard(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Column(children: [
                for (final (i, item) in items.indexed) QueueRow(index: i + 1, item: item),
              ]),
            ),
            const SizedBox(height: 12),
            Text('Runs in this order. A task that can no longer run when its turn comes (an application rejected meanwhile, …) is skipped with an alert.', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          ],
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: Text(items.isEmpty ? 'Queue' : 'Queue (${items.length})'),
        actions: [
          if (items.length > 1)
            TextButton(
              onPressed: () async {
                final ok = await confirmDialog(context, title: 'Clear the queue?', message: 'Every waiting task is removed. Whatever is running now keeps going.', confirm: 'Clear');
                if (!ok || !context.mounted) return;
                final resp = await queue.clear();
                if (context.mounted) showOutcome(context, resp, success: 'Queue cleared.');
              },
              child: const Text('Clear all'),
            ),
        ],
      ),
      body: Column(children: [
        const PresenceBanner(),
        if (queue.loading && items.isNotEmpty) const LinearProgressIndicator(minHeight: 2),
        Expanded(child: body),
      ]),
    );
  }
}

IconData queueKindIcon(String kind) => switch (kind) {
      'APPLICATION' => Icons.send_outlined,
      'ANALYZE_JOB' => Icons.analytics_outlined,
      'GENERATE_RESUME' => Icons.description_outlined,
      'PROFILE_SYNC' => Icons.language,
      'JOB_LINK' => Icons.link,
      'SHARED_JOB' => Icons.post_add_outlined,
      'INBOX_CHECK' => Icons.mark_email_unread_outlined,
      _ => Icons.travel_explore_outlined,
    };

/// One queued task with its place and a remove button.
class QueueRow extends StatefulWidget {
  final int index;
  final QueuedTaskItem item;
  final bool compact;
  const QueueRow({super.key, required this.index, required this.item, this.compact = false});

  @override
  State<QueueRow> createState() => _QueueRowState();
}

class _QueueRowState extends State<QueueRow> {
  bool _removing = false;

  Future<void> _remove() async {
    final queue = context.read<QueueController>();
    setState(() => _removing = true);
    final resp = await queue.remove(widget.item.id);
    if (!mounted) return;
    setState(() => _removing = false);
    showOutcome(context, resp, success: 'Removed from the queue.');
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final item = widget.item;
    final reachable = context.watch<ConnectionController>().desktopReachable;
    return ListTile(
      dense: widget.compact,
      leading: Stack(alignment: Alignment.bottomRight, children: [
        IconBubble(queueKindIcon(item.kind), color: theme.colorScheme.primary, size: 36),
        Container(
          width: 16,
          height: 16,
          decoration: BoxDecoration(color: theme.colorScheme.primary, shape: BoxShape.circle, border: Border.all(color: theme.colorScheme.surface, width: 1.5)),
          child: Center(child: Text('${widget.index}', style: const TextStyle(color: Colors.white, fontSize: 9, fontWeight: FontWeight.w800))),
        ),
      ]),
      title: Text(item.title, maxLines: 2, overflow: TextOverflow.ellipsis, style: const TextStyle(fontWeight: FontWeight.w600)),
      subtitle: item.requestedAt == null ? null : Text('Added ${timeAgo(item.requestedAt)}'),
      trailing: _removing
          ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
          : IconButton(tooltip: 'Remove', onPressed: reachable ? _remove : null, icon: const Icon(Icons.close, size: 20)),
    );
  }
}
