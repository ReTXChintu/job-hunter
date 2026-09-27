import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/app_notification.dart';
import '../state/notifications_controller.dart';
import '../widgets/empty_state.dart';
import 'application_detail_screen.dart';

/// Everything the desktop wanted you to know: finished job hunts,
/// applications and job-site updates that need you, failures.
class NotificationsScreen extends StatelessWidget {
  const NotificationsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final c = context.watch<NotificationsController>();
    return Scaffold(
      appBar: AppBar(
        title: const Text('Alerts'),
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
              child: ListView.separated(
                itemCount: c.items.length,
                separatorBuilder: (context, index) => const Divider(height: 1),
                itemBuilder: (context, i) => _NotificationTile(n: c.items[i], unread: c.isUnread(c.items[i])),
              ),
            ),
    );
  }
}

class _NotificationTile extends StatelessWidget {
  final AppNotification n;
  final bool unread;
  const _NotificationTile({required this.n, required this.unread});

  Color _color(ColorScheme scheme) => switch (n.level) {
        'ERROR' => scheme.error,
        'WARN' => Colors.orange,
        'SUCCESS' => Colors.green,
        _ => scheme.primary,
      };

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final when = DateTime.tryParse(n.createdAt)?.toLocal();
    return ListTile(
      leading: Icon(
        switch (n.level) { 'ERROR' => Icons.error_outline, 'WARN' => Icons.warning_amber, 'SUCCESS' => Icons.check_circle_outline, _ => Icons.info_outline },
        color: _color(scheme),
      ),
      title: Text(n.title, style: TextStyle(fontWeight: unread ? FontWeight.w600 : FontWeight.normal)),
      subtitle: Text(
        [if (n.body.isNotEmpty) n.body, if (when != null) '${MaterialLocalizations.of(context).formatShortDate(when)} ${TimeOfDay.fromDateTime(when).format(context)}'].join('\n'),
        maxLines: 3,
        overflow: TextOverflow.ellipsis,
      ),
      isThreeLine: n.body.isNotEmpty,
      onTap: () {
        final id = n.applicationId;
        if (id != null) {
          Navigator.of(context).push(MaterialPageRoute(builder: (_) => ApplicationDetailScreen(applicationId: id)));
        } else {
          ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Open the desktop app to act on this.')));
        }
      },
    );
  }
}
