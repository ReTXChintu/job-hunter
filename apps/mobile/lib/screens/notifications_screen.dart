import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/app_notification.dart';
import '../state/notifications_controller.dart';
import '../util/format.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import 'shell_screen.dart';

/// Everything the desktop wanted you to know: finished job hunts,
/// applications and job-site updates that need you, failures.
class NotificationsScreen extends StatelessWidget {
  const NotificationsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final c = context.watch<NotificationsController>();
    final groups = _groupByDay(c.items);
    return Scaffold(
      appBar: AppBar(
        title: Text(c.unread > 0 ? 'Alerts (${c.unread})' : 'Alerts'),
        actions: [
          if (c.unread > 0) TextButton(onPressed: c.markAllSeen, child: const Text('Mark all seen')),
        ],
      ),
      body: c.items.isEmpty
          ? (c.loading
              ? const Center(child: CircularProgressIndicator())
              : RefreshIndicator(
                  onRefresh: c.refresh,
                  child: ListView(children: [
                    SizedBox(
                      height: 360,
                      child: EmptyState(
                        icon: c.error != null ? Icons.error_outline : Icons.notifications_none,
                        title: c.error != null ? 'Could not load alerts' : 'No alerts yet',
                        message: c.error ?? 'You\'ll be alerted here when a job hunt finishes or something needs you, even when the app is closed.',
                      ),
                    ),
                  ]),
                ))
          : RefreshIndicator(
              onRefresh: c.refresh,
              child: ListView(
                padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
                children: [
                  for (final (day, list) in groups) ...[
                    Padding(
                      padding: const EdgeInsets.fromLTRB(4, 14, 4, 8),
                      child: Text(day, style: Theme.of(context).textTheme.labelLarge?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant, fontWeight: FontWeight.w700)),
                    ),
                    for (final n in list)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: _NotificationCard(n: n, unread: c.isUnread(n)),
                      ),
                  ],
                ],
              ),
            ),
    );
  }

  List<(String, List<AppNotification>)> _groupByDay(List<AppNotification> items) {
    final now = DateTime.now();
    final groups = <String, List<AppNotification>>{};
    for (final n in items) {
      final at = DateTime.tryParse(n.createdAt)?.toLocal();
      final String day;
      if (at == null) {
        day = 'Earlier';
      } else if (at.year == now.year && at.month == now.month && at.day == now.day) {
        day = 'Today';
      } else if (now.difference(at).inDays < 1 || (now.difference(at).inDays == 1 && at.day == now.day - 1)) {
        day = 'Yesterday';
      } else if (now.difference(at).inDays < 7) {
        day = 'This week';
      } else {
        day = 'Earlier';
      }
      groups.putIfAbsent(day, () => []).add(n);
    }
    return [for (final e in groups.entries) (e.key, e.value)];
  }
}

class _NotificationCard extends StatelessWidget {
  final AppNotification n;
  final bool unread;
  const _NotificationCard({required this.n, required this.unread});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final color = switch (n.level) { 'ERROR' => scheme.error, 'WARN' => Colors.orange, 'SUCCESS' => Colors.green, _ => scheme.primary };
    final icon = switch (n.level) { 'ERROR' => Icons.error_outline, 'WARN' => Icons.warning_amber_rounded, 'SUCCESS' => Icons.check_circle_outline, _ => Icons.info_outline };
    final when = DateTime.tryParse(n.createdAt);
    return AppCard(
      onTap: () => openAppLink(context, n.link),
      padding: const EdgeInsets.fromLTRB(14, 12, 12, 12),
      color: unread ? null : scheme.surfaceContainerLow,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          IconBubble(icon, color: color, size: 36),
          const SizedBox(width: 12),
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Row(children: [
                Expanded(child: Text(n.title, style: theme.textTheme.titleSmall?.copyWith(fontWeight: unread ? FontWeight.w700 : FontWeight.w600))),
                if (unread) Container(width: 8, height: 8, margin: const EdgeInsets.only(left: 8), decoration: BoxDecoration(color: scheme.primary, shape: BoxShape.circle)),
              ]),
              if (n.body.isNotEmpty) ...[
                const SizedBox(height: 3),
                Text(n.body, maxLines: 3, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodySmall?.copyWith(color: scheme.onSurface.withValues(alpha: 0.85))),
              ],
              if (when != null) ...[
                const SizedBox(height: 6),
                Text('${timeAgo(when)} · ${formatDateTime(context, when)}', style: theme.textTheme.bodySmall?.copyWith(color: scheme.onSurfaceVariant, fontSize: 11.5)),
              ],
            ]),
          ),
        ],
      ),
    );
  }
}
