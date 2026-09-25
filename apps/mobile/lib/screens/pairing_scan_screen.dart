import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:provider/provider.dart';

import '../config.dart';
import '../state/auth_controller.dart';

/// Pairs this phone using a one-time code minted on the desktop (Settings
/// > Mobile app > Generate pairing code). The QR just encodes
/// `{"code": "..."}` -- see `apps/desktop/src/components/MobileAppSettings.tsx`
/// -- and the server URL is built into the app (see `lib/config.dart`),
/// never taken from the QR.
class PairingScanScreen extends StatefulWidget {
  const PairingScanScreen({super.key});

  @override
  State<PairingScanScreen> createState() => _PairingScanScreenState();
}

class _PairingScanScreenState extends State<PairingScanScreen> {
  final _relayCtrl = TextEditingController();
  final _codeCtrl = TextEditingController();
  final _deviceNameCtrl = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  MobileScannerController? _scanner;
  bool _scanning = false;
  bool _handledScan = false;

  @override
  void dispose() {
    _relayCtrl.dispose();
    _codeCtrl.dispose();
    _deviceNameCtrl.dispose();
    _scanner?.dispose();
    super.dispose();
  }

  void _toggleScanner() {
    setState(() {
      _scanning = !_scanning;
      if (_scanning) {
        _handledScan = false;
        _scanner = MobileScannerController();
      } else {
        _scanner?.dispose();
        _scanner = null;
      }
    });
  }

  void _onDetect(BarcodeCapture capture) {
    if (_handledScan) return;
    final raw = capture.barcodes.firstOrNull?.rawValue;
    if (raw == null) return;
    String? code;
    try {
      final decoded = jsonDecode(raw);
      if (decoded is Map && decoded['code'] is String) code = decoded['code'] as String;
    } catch (_) {
      // Not JSON: fall back to treating the whole payload as the code.
      code = raw;
    }
    if (code == null || code.isEmpty) return;
    _handledScan = true;
    _codeCtrl.text = code;
    _toggleScanner();
    _submit();
  }

  String get _serverUrl => hasBuiltInServer ? kBuiltInServerUrl : _relayCtrl.text;

  Future<void> _submit() async {
    if (_serverUrl.trim().isEmpty || _codeCtrl.text.trim().isEmpty) {
      if (!_formKey.currentState!.validate()) return;
    }
    final auth = context.read<AuthController>();
    final ok = await auth.pairWithCode(relayUrl: _serverUrl, code: _codeCtrl.text, deviceName: _deviceNameCtrl.text);
    if (ok && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Paired.')));
    }
  }

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthController>();
    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: 8),
            Text('Scan the QR code, or type the code shown in Settings > Mobile app on your desktop.', style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant)),
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
            if (_scanning)
              ClipRRect(
                borderRadius: BorderRadius.circular(10),
                child: SizedBox(height: 260, child: MobileScanner(controller: _scanner, onDetect: _onDetect)),
              )
            else
              OutlinedButton.icon(onPressed: _toggleScanner, icon: const Icon(Icons.qr_code_scanner), label: const Text('Scan QR code')),
            if (_scanning) ...[
              const SizedBox(height: 8),
              TextButton(onPressed: _toggleScanner, child: const Text('Cancel scan')),
            ],
            const SizedBox(height: 12),
            TextFormField(
              controller: _codeCtrl,
              decoration: const InputDecoration(labelText: 'Pairing code'),
              textCapitalization: TextCapitalization.characters,
              validator: (v) => (v == null || v.trim().isEmpty) ? 'Required' : null,
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _deviceNameCtrl,
              decoration: const InputDecoration(labelText: 'Device name (optional)', hintText: 'My phone'),
            ),
            if (auth.lastError != null) ...[
              const SizedBox(height: 8),
              Text(auth.lastError!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
            ],
            const SizedBox(height: 16),
            FilledButton(
              onPressed: auth.isBusy ? null : _submit,
              child: auth.isBusy ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2)) : const Text('Pair this phone'),
            ),
          ],
        ),
      ),
    );
  }
}

extension _FirstOrNull<T> on List<T> {
  T? get firstOrNull => isEmpty ? null : first;
}
