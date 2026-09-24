import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/relay_client.dart';
import '../state/connection_controller.dart';

String _timeAgo(DateTime? at) {
  if (at == null) return '';
  final diff = DateTime.now().difference(at);
  if (diff.inMinutes < 1) return 'just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes} min ago';
  if (diff.inHours < 24) return '${diff.inHours} h ago';
  return '${diff.inDays} d ago';
}

/// A thin banner showing whether the desktop is reachable right now. This
/// reflects the relay's own `presence` push, never a guess made on the
/// phone.
class PresenceBanner extends StatelessWidget {
  const PresenceBanner({super.key});

  @override
  Widget build(BuildContext context) {
    final connection = context.watch<ConnectionController>();
    final scheme = Theme.of(context).colorScheme;

    if (connection.socketState != SocketState.connected) {
      return _Bar(color: Colors.orange, icon: Icons.cloud_off, text: connection.socketState == SocketState.connecting ? 'Connecting to your relay…' : 'Not connected to your relay');
    }
    if (!connection.desktopOnline) {
      final seen = connection.desktopLastSeenAt;
      return _Bar(color: Colors.orange, icon: Icons.desktop_access_disabled, text: seen == null ? 'Desktop offline' : 'Desktop offline · last seen ${_timeAgo(seen)}');
    }
    return _Bar(color: Colors.green, icon: Icons.desktop_windows, text: 'Desktop online', background: scheme.surface);
  }
}

class _Bar extends StatelessWidget {
  final Color color;
  final IconData icon;
  final String text;
  final Color? background;
  const _Bar({required this.color, required this.icon, required this.text, this.background});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: background ?? color.withValues(alpha: 0.12),
      child: Row(
        children: [
          Icon(icon, size: 16, color: color),
          const SizedBox(width: 8),
          Expanded(child: Text(text, style: TextStyle(color: color, fontSize: 13, fontWeight: FontWeight.w600))),
        ],
      ),
    );
  }
}
