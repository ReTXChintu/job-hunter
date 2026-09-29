import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/auth_controller.dart';
import '../state/update_controller.dart';
import '../widgets/common.dart';
import 'about_screen.dart';
import 'answers_screen.dart';
import 'job_sites_screen.dart';
import 'settings_screen.dart';

/// Everything else: job-site profiles, saved answers, settings, about.
class MoreScreen extends StatelessWidget {
  const MoreScreen({super.key});

  void _open(BuildContext context, Widget screen) => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => screen));

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthController>();
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('More')),
      body: ListView(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
            child: Row(children: [
              const BrandLogo(size: 48),
              const SizedBox(width: 14),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text('Job Hunter', style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700)),
                  Text(
                    (auth.accountEmail?.isNotEmpty ?? false) ? auth.accountEmail! : (auth.deviceName ?? ''),
                    style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant),
                  ),
                ]),
              ),
            ]),
          ),
          const Divider(),
          ListTile(
            leading: const Icon(Icons.language),
            title: const Text('Job sites'),
            subtitle: const Text('Keep your LinkedIn, Naukri and other profiles up to date'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _open(context, const JobSitesScreen()),
          ),
          ListTile(
            leading: const Icon(Icons.question_answer_outlined),
            title: const Text('Additional details'),
            subtitle: const Text('Saved answers the agent reuses on application forms'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _open(context, const AnswersScreen()),
          ),
          const Divider(),
          ListTile(
            leading: const Icon(Icons.settings_outlined),
            title: const Text('Settings'),
            subtitle: const Text('Connection, this device, sign out'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _open(context, const SettingsScreen()),
          ),
          ListTile(
            leading: const Icon(Icons.info_outline),
            title: const Text('About'),
            subtitle: Text(context.watch<UpdateController>().showBanner ? 'Update available' : 'Version and updates'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _open(context, const AboutScreen()),
          ),
        ],
      ),
    );
  }
}
