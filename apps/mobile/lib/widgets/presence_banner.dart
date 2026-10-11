import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/relay_client.dart';
import '../state/connection_controller.dart';
import '../util/format.dart';
import 'common.dart';

/// Says when the desktop can't be reached right now, from the relay's own
/// `presence` push (never a guess made on the phone). Shows nothing while
/// everything is fine.
class PresenceBanner extends StatelessWidget {
  const PresenceBanner({super.key});

  @override
  Widget build(BuildContext context) {
    final connection = context.watch<ConnectionController>();
    if (connection.socketState != SocketState.connected) {
      return InlineNotice(
        color: Colors.orange,
        icon: Icons.cloud_off,
        text: connection.socketState == SocketState.connecting ? 'Connecting to your Job Hunter server…' : 'Not connected to your Job Hunter server',
      );
    }
    if (!connection.desktopOnline) {
      final seen = connection.desktopLastSeenAt;
      return InlineNotice(
        color: Colors.orange,
        icon: Icons.desktop_access_disabled,
        text: seen == null ? 'Desktop offline — actions need it running' : 'Desktop offline · last seen ${timeAgo(seen)}',
      );
    }
    return const SizedBox.shrink();
  }
}

/// The connection in one small pill: online, offline or connecting.
class ConnectionPill extends StatelessWidget {
  /// Draw for a dark (hero) background.
  final bool onDark;
  const ConnectionPill({super.key, this.onDark = false});

  @override
  Widget build(BuildContext context) {
    final connection = context.watch<ConnectionController>();
    final (label, color, icon) = switch (connection.socketState) {
      SocketState.connected when connection.desktopOnline => ('Desktop online', Colors.greenAccent.shade400, Icons.desktop_windows),
      SocketState.connected => ('Desktop offline', Colors.orangeAccent, Icons.desktop_access_disabled),
      SocketState.connecting => ('Connecting…', Colors.orangeAccent, Icons.sync),
      SocketState.disconnected => ('No connection', Colors.orangeAccent, Icons.cloud_off),
    };
    final fg = onDark ? Colors.white : Theme.of(context).colorScheme.onSurface;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: onDark ? Colors.white.withValues(alpha: 0.16) : Theme.of(context).colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(mainAxisSize: MainAxisSize.min, children: [
        Container(width: 8, height: 8, decoration: BoxDecoration(color: color, shape: BoxShape.circle)),
        const SizedBox(width: 6),
        Icon(icon, size: 13, color: fg),
        const SizedBox(width: 4),
        Text(label, style: TextStyle(color: fg, fontSize: 12, fontWeight: FontWeight.w700)),
      ]),
    );
  }
}
