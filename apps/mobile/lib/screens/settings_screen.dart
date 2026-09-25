import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/relay_client.dart';
import '../state/auth_controller.dart';
import '../state/connection_controller.dart';
import '../state/update_controller.dart';
import 'about_screen.dart';

/// Connection status and sign-out. This phone never manages other paired
/// devices or account settings -- that stays on the desktop's own Settings
/// > Mobile app tab, which is the source of truth for the account.
class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key});

  String _socketLabel(SocketState s) => switch (s) {
        SocketState.connected => 'Connected',
        SocketState.connecting => 'Connecting…',
        SocketState.disconnected => 'Disconnected',
      };

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthController>();
    final connection = context.watch<ConnectionController>();

    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        children: [
          const SizedBox(height: 8),
          ListTile(
            leading: const Icon(Icons.cloud_outlined),
            title: const Text('Server'),
            subtitle: Text(auth.relayUrl ?? 'Not connected'),
          ),
          if (auth.accountEmail != null && auth.accountEmail!.isNotEmpty)
            ListTile(leading: const Icon(Icons.person_outline), title: const Text('Account'), subtitle: Text(auth.accountEmail!)),
          ListTile(leading: const Icon(Icons.phone_iphone), title: const Text('This device'), subtitle: Text(auth.deviceName ?? '')),
          ListTile(
            leading: Icon(connection.socketState == SocketState.connected ? Icons.wifi : Icons.wifi_off, color: connection.socketState == SocketState.connected ? Colors.green : Colors.orange),
            title: const Text('Connection'),
            subtitle: Text(_socketLabel(connection.socketState)),
          ),
          ListTile(
            leading: Icon(connection.desktopOnline ? Icons.desktop_windows : Icons.desktop_access_disabled, color: connection.desktopOnline ? Colors.green : Colors.orange),
            title: const Text('Desktop'),
            subtitle: Text(connection.desktopOnline ? 'Online' : 'Offline'),
          ),
          ListTile(
            leading: const Icon(Icons.info_outline),
            title: const Text('About'),
            subtitle: Text(context.watch<UpdateController>().showBanner ? 'Update available' : 'Version and updates'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => const AboutScreen())),
          ),
          const Divider(height: 32),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: OutlinedButton.icon(
              onPressed: () async {
                final authController = context.read<AuthController>();
                final confirmed = await showDialog<bool>(
                  context: context,
                  builder: (ctx) => AlertDialog(
                    title: const Text('Sign out?'),
                    content: const Text('This forgets this phone\'s sign-in locally. Your account and its data are unaffected.'),
                    actions: [
                      TextButton(onPressed: () => Navigator.of(ctx).pop(false), child: const Text('Cancel')),
                      FilledButton(onPressed: () => Navigator.of(ctx).pop(true), child: const Text('Sign out')),
                    ],
                  ),
                );
                if (confirmed == true) {
                  await authController.signOut();
                }
              },
              icon: const Icon(Icons.logout),
              label: const Text('Sign out'),
            ),
          ),
          const SizedBox(height: 24),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Text(
              'This app only reviews and approves work your desktop agent already prepared. No AI runs here and no data is stored anywhere except your own server and this phone.',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant),
            ),
          ),
        ],
      ),
    );
  }
}
