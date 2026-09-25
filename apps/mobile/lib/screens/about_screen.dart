import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../config.dart';
import '../state/update_controller.dart';

String formatMb(int bytes) => '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';

/// App name, version and server, plus a manual "Check for updates".
class AboutScreen extends StatelessWidget {
  const AboutScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final updates = context.watch<UpdateController>();
    final theme = Theme.of(context);
    final info = updates.info;
    final pkg = updates.package;

    Widget updateStatus() {
      if (updates.isChecking) {
        return const Row(children: [SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)), SizedBox(width: 12), Text('Checking for updates…')]);
      }
      if (updates.lastError != null) return Text(updates.lastError!, style: TextStyle(color: theme.colorScheme.error));
      if (info == null) return Text('Not checked yet.', style: TextStyle(color: theme.colorScheme.onSurfaceVariant));
      if (!info.available) {
        return Row(children: [
          const Icon(Icons.check_circle, color: Colors.green, size: 20),
          const SizedBox(width: 8),
          Expanded(child: Text("You're up to date${info.latest?.version != null ? ' (latest is ${info.latest!.version})' : ''}.")),
        ]);
      }
      final latest = info.latest!;
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Version ${latest.version}${latest.build != null ? ' (build ${latest.build})' : ''} is available.', style: theme.textTheme.titleSmall),
          const SizedBox(height: 4),
          Text('Tap Download, then open the downloaded file to install it over this app. Your sign-in is kept.', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          const SizedBox(height: 12),
          FilledButton.icon(
            onPressed: () async {
              final messenger = ScaffoldMessenger.of(context);
              if (!await updates.download()) messenger.showSnackBar(const SnackBar(content: Text("Couldn't open the download.")));
            },
            icon: const Icon(Icons.download),
            label: Text('Download${latest.sizeBytes > 0 ? ' (${formatMb(latest.sizeBytes)})' : ''}'),
          ),
        ],
      );
    }

    return Scaffold(
      appBar: AppBar(title: const Text('About')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(color: theme.colorScheme.primary, borderRadius: BorderRadius.circular(12)),
                child: Icon(Icons.work_outline, color: theme.colorScheme.onPrimary, size: 28),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Job Hunter', style: theme.textTheme.titleLarge),
                    Text(pkg == null ? '' : 'Version ${pkg.version} (build ${pkg.buildNumber})', style: TextStyle(color: theme.colorScheme.onSurfaceVariant)),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      Expanded(child: Text('Updates', style: theme.textTheme.titleMedium)),
                      TextButton.icon(
                        onPressed: updates.isChecking || updates.serverUrl == null ? null : updates.check,
                        icon: const Icon(Icons.refresh, size: 18),
                        label: const Text('Check for updates'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  updateStatus(),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          Card(
            child: Column(
              children: [
                ListTile(title: const Text('Version'), subtitle: Text(pkg == null ? '…' : '${pkg.version} (${pkg.buildNumber})')),
                ListTile(title: const Text('Package'), subtitle: Text(pkg?.packageName ?? '…')),
                ListTile(
                  title: const Text('Server'),
                  subtitle: Text(updates.serverUrl ?? 'Not signed in yet${hasBuiltInServer ? '' : ' (development build)'}'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Shown above the home screen when a newer APK is on the server.
class UpdateBanner extends StatelessWidget {
  const UpdateBanner({super.key});

  @override
  Widget build(BuildContext context) {
    final updates = context.watch<UpdateController>();
    if (!updates.showBanner) return const SizedBox.shrink();
    final latest = updates.info!.latest!;
    final scheme = Theme.of(context).colorScheme;
    return Material(
      color: scheme.primaryContainer,
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 4, 8),
          child: Row(
            children: [
              Icon(Icons.system_update, color: scheme.onPrimaryContainer),
              const SizedBox(width: 12),
              Expanded(
                child: Text('Job Hunter ${latest.version} is available', style: TextStyle(color: scheme.onPrimaryContainer, fontWeight: FontWeight.w600)),
              ),
              TextButton(
                onPressed: () async {
                  final messenger = ScaffoldMessenger.of(context);
                  if (!await updates.download()) messenger.showSnackBar(const SnackBar(content: Text("Couldn't open the download.")));
                },
                child: const Text('Download'),
              ),
              IconButton(tooltip: 'Later', onPressed: updates.dismiss, icon: Icon(Icons.close, color: scheme.onPrimaryContainer, size: 20)),
            ],
          ),
        ),
      ),
    );
  }
}
