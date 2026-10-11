import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/auth_controller.dart';
import '../state/queue_controller.dart';
import '../state/update_controller.dart';
import '../widgets/common.dart';
import 'about_screen.dart';
import 'answers_screen.dart';
import 'job_sites_screen.dart';
import 'queue_screen.dart';
import 'settings_screen.dart';

/// Everything else: the queue, job-site profiles, saved answers, settings,
/// about.
class MoreScreen extends StatelessWidget {
  const MoreScreen({super.key});

  void _open(BuildContext context, Widget screen) => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => screen));

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthController>();
    final queued = context.watch<QueueController>().count;
    final updateAvailable = context.watch<UpdateController>().showBanner;
    final theme = Theme.of(context);

    Widget tile(IconData icon, Color color, String title, String subtitle, Widget screen, {Widget? trailing}) => ListTile(
          leading: IconBubble(icon, color: color, size: 38),
          title: Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
          subtitle: Text(subtitle, maxLines: 2, overflow: TextOverflow.ellipsis),
          trailing: trailing ?? const Icon(Icons.chevron_right),
          onTap: () => _open(context, screen),
        );

    return Scaffold(
      appBar: AppBar(title: const Text('More')),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
        children: [
          AppCard(
            child: Row(children: [
              const BrandLogo(size: 52),
              const SizedBox(width: 14),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text('Job Hunter', style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w800)),
                  Text(
                    (auth.accountEmail?.isNotEmpty ?? false) ? auth.accountEmail! : (auth.deviceName ?? ''),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant),
                  ),
                ]),
              ),
            ]),
          ),
          const SectionHeader('Agent'),
          AppCard(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Column(children: [
              tile(
                Icons.playlist_play,
                theme.colorScheme.primary,
                'Queue',
                queued > 0 ? '$queued task${queued == 1 ? '' : 's'} waiting for the agent' : 'What runs next, in order',
                const QueueScreen(),
                trailing: queued > 0 ? Badge(label: Text('$queued')) : const Icon(Icons.chevron_right),
              ),
              const Divider(indent: 70),
              tile(Icons.language, Colors.indigo, 'Job sites', 'Keep your LinkedIn, Naukri and other profiles up to date', const JobSitesScreen()),
              const Divider(indent: 70),
              tile(Icons.question_answer_outlined, Colors.teal, 'Additional details', 'Saved answers the agent reuses on application forms', const AnswersScreen()),
            ]),
          ),
          const SectionHeader('App'),
          AppCard(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Column(children: [
              tile(Icons.settings_outlined, Colors.blueGrey, 'Settings', 'Connection, this device, sign out', const SettingsScreen()),
              const Divider(indent: 70),
              tile(
                Icons.info_outline,
                Colors.grey,
                'About',
                updateAvailable ? 'Update available' : 'Version and updates',
                const AboutScreen(),
                trailing: updateAvailable ? const Badge(label: Text('1')) : const Icon(Icons.chevron_right),
              ),
            ]),
          ),
        ],
      ),
    );
  }
}
