import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../config.dart';
import '../state/auth_controller.dart';
import 'pairing_scan_screen.dart';

/// Sign-in/sign-up with an email+password account on a relay the user
/// hosts themselves. This is a fallback path -- pairing by code (see
/// [PairingScanScreen]) is the ergonomic default -- but it has to exist
/// because a phone needs *some* way to become the first paired device.
class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> with SingleTickerProviderStateMixin {
  late final TabController _tab = TabController(length: 2, vsync: this);
  final _relayCtrl = TextEditingController();
  final _emailCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  final _deviceNameCtrl = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _isRegister = false;

  @override
  void dispose() {
    _tab.dispose();
    _relayCtrl.dispose();
    _emailCtrl.dispose();
    _passwordCtrl.dispose();
    _deviceNameCtrl.dispose();
    super.dispose();
  }

  String get _serverUrl => hasBuiltInServer ? kBuiltInServerUrl : _relayCtrl.text;

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    final auth = context.read<AuthController>();
    final ok = _isRegister
        ? await auth.signUp(relayUrl: _serverUrl, email: _emailCtrl.text, password: _passwordCtrl.text, deviceName: _deviceNameCtrl.text)
        : await auth.signIn(relayUrl: _serverUrl, email: _emailCtrl.text, password: _passwordCtrl.text, deviceName: _deviceNameCtrl.text);
    if (ok && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Signed in.')));
    }
  }

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthController>();
    return Scaffold(
      appBar: AppBar(
        title: const Text('Job Hunter'),
        bottom: TabBar(controller: _tab, tabs: const [Tab(text: 'Pair with code'), Tab(text: 'Email + password')]),
      ),
      body: TabBarView(
        controller: _tab,
        children: [
          const PairingScanScreen(),
          SingleChildScrollView(
            padding: const EdgeInsets.all(20),
            child: Form(
              key: _formKey,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const SizedBox(height: 8),
                  Text('This connects to the Job Hunter server you host yourself, not a service we run.', style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant)),
                  const SizedBox(height: 20),
                  if (!hasBuiltInServer) ...[
                    TextFormField(
                      controller: _relayCtrl,
                      decoration: const InputDecoration(labelText: 'Server URL', hintText: 'http://192.168.1.10:8788'),
                      keyboardType: TextInputType.url,
                      autocorrect: false,
                      validator: (v) => (v == null || v.trim().isEmpty) ? 'Required' : null,
                    ),
                    const SizedBox(height: 12),
                  ],
                  TextFormField(
                    controller: _emailCtrl,
                    decoration: const InputDecoration(labelText: 'Email'),
                    keyboardType: TextInputType.emailAddress,
                    autocorrect: false,
                    validator: (v) => (v == null || v.trim().isEmpty) ? 'Required' : null,
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    controller: _passwordCtrl,
                    decoration: const InputDecoration(labelText: 'Password'),
                    obscureText: true,
                    validator: (v) => (v == null || v.length < 8) ? 'At least 8 characters' : null,
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    controller: _deviceNameCtrl,
                    decoration: const InputDecoration(labelText: 'Device name (optional)', hintText: 'My phone'),
                  ),
                  const SizedBox(height: 8),
                  SwitchListTile(
                    contentPadding: EdgeInsets.zero,
                    title: const Text('Create a new account'),
                    subtitle: const Text('Off signs in to an existing account'),
                    value: _isRegister,
                    onChanged: (v) => setState(() => _isRegister = v),
                  ),
                  if (auth.lastError != null) ...[
                    const SizedBox(height: 8),
                    Text(auth.lastError!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
                  ],
                  const SizedBox(height: 16),
                  FilledButton(
                    onPressed: auth.isBusy ? null : _submit,
                    child: auth.isBusy ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2)) : Text(_isRegister ? 'Create account' : 'Sign in'),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
