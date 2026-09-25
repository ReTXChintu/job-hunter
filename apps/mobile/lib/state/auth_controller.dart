import 'dart:io';

import 'package:flutter/foundation.dart';

import '../config.dart';
import '../services/api_client.dart';
import '../services/secure_store.dart';

enum AuthStatus { unknown, signedOut, signedIn }

/// Owns sign-in/pairing/sign-out. Holds no job or application data -- see
/// `ApplicationsController` for that. Mirrors the desktop's `RemoteClient`
/// register/login/logout flow (`crates/job-hunter-core/src/remote/client.rs`):
/// after the one register-or-pair call, only the device token is kept.
class AuthController extends ChangeNotifier {
  final LocalStore _store;
  AuthController({LocalStore? store}) : _store = store ?? LocalStore();

  AuthStatus status = AuthStatus.unknown;
  bool isBusy = false;
  String? lastError;

  String? relayUrl;
  String? accountEmail;
  String? deviceName;
  String? deviceToken;

  Future<void> loadPersisted() async {
    final token = await _store.deviceToken;
    relayUrl = await _store.relayUrl;
    // The server address is fixed per build: if this phone signed in under
    // an older build's address, follow the current one.
    if (hasBuiltInServer && relayUrl != null && relayUrl!.isNotEmpty) {
      relayUrl = ApiClient.normalizeRelayUrl(kBuiltInServerUrl);
    }
    accountEmail = await _store.accountEmail;
    deviceName = await _store.deviceName;
    deviceToken = token;
    status = (token != null && token.isNotEmpty && relayUrl != null && relayUrl!.isNotEmpty) ? AuthStatus.signedIn : AuthStatus.signedOut;
    notifyListeners();
  }

  Future<bool> _run(Future<void> Function() body) async {
    isBusy = true;
    lastError = null;
    notifyListeners();
    try {
      await body();
      isBusy = false;
      notifyListeners();
      return true;
    } catch (e) {
      isBusy = false;
      lastError = e is RelayApiException ? e.message : e.toString();
      notifyListeners();
      return false;
    }
  }

  Future<bool> signUp({required String relayUrl, required String email, required String password, required String deviceName}) => _authenticate(relayUrl: relayUrl, email: email, password: password, deviceName: deviceName, isRegister: true);

  Future<bool> signIn({required String relayUrl, required String email, required String password, required String deviceName}) => _authenticate(relayUrl: relayUrl, email: email, password: password, deviceName: deviceName, isRegister: false);

  Future<bool> _authenticate({required String relayUrl, required String email, required String password, required String deviceName, required bool isRegister}) {
    return _run(() async {
      final url = ApiClient.normalizeRelayUrl(relayUrl);
      final api = ApiClient(url);
      final accessToken = isRegister ? await api.register(email.trim(), password) : await api.login(email.trim(), password);
      final device = await api.registerDevice(accessToken: accessToken, name: deviceName.trim().isEmpty ? _defaultDeviceName() : deviceName.trim(), platform: _platform());
      await _store.setDeviceToken(device.deviceToken);
      await _store.savePairing(relayUrl: url, accountEmail: email.trim(), deviceName: deviceName.trim().isEmpty ? _defaultDeviceName() : deviceName.trim());
      this.relayUrl = url;
      accountEmail = email.trim();
      this.deviceName = deviceName.trim().isEmpty ? _defaultDeviceName() : deviceName.trim();
      deviceToken = device.deviceToken;
      status = AuthStatus.signedIn;
    });
  }

  /// Pairs with a one-time code minted on the desktop -- no password typed
  /// here at all.
  Future<bool> pairWithCode({required String relayUrl, required String code, required String deviceName}) {
    return _run(() async {
      final url = ApiClient.normalizeRelayUrl(relayUrl);
      final api = ApiClient(url);
      final name = deviceName.trim().isEmpty ? _defaultDeviceName() : deviceName.trim();
      final device = await api.redeemPairingCode(code: code.trim().toUpperCase(), name: name, platform: _platform());
      await _store.setDeviceToken(device.deviceToken);
      await _store.savePairing(relayUrl: url, accountEmail: '', deviceName: name);
      this.relayUrl = url;
      accountEmail = '';
      this.deviceName = name;
      deviceToken = device.deviceToken;
      status = AuthStatus.signedIn;
    });
  }

  /// Forgets this phone's credentials locally. The device entry itself
  /// stays valid on the relay until revoked from the desktop's Paired
  /// Devices list (or re-paired over); this matches "sign out" rather than
  /// "revoke" -- the same distinction the desktop draws.
  Future<void> signOut() async {
    isBusy = true;
    notifyListeners();
    await _store.clearAll();
    relayUrl = null;
    accountEmail = null;
    deviceName = null;
    deviceToken = null;
    status = AuthStatus.signedOut;
    isBusy = false;
    notifyListeners();
  }

  String _platform() {
    if (kIsWeb) return 'web';
    if (Platform.isAndroid) return 'android';
    if (Platform.isIOS) return 'ios';
    return Platform.operatingSystem;
  }

  String _defaultDeviceName() => '${_platform()} phone';
}
