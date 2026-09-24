import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Everything this phone remembers about its pairing, split the same way
/// the desktop splits it: the device token is the one secret (OS keychain /
/// Keystore via flutter_secure_storage); the relay address and display
/// details are not secret and live in shared_preferences.
class LocalStore {
  static const _deviceTokenKey = 'job_hunter.device_token';
  static const _relayUrlKey = 'job_hunter.relay_url';
  static const _accountEmailKey = 'job_hunter.account_email';
  static const _deviceNameKey = 'job_hunter.device_name';

  final FlutterSecureStorage _secure;
  LocalStore({FlutterSecureStorage? secure}) : _secure = secure ?? const FlutterSecureStorage();

  Future<String?> get deviceToken => _secure.read(key: _deviceTokenKey);
  Future<void> setDeviceToken(String token) => _secure.write(key: _deviceTokenKey, value: token);
  Future<void> clearDeviceToken() => _secure.delete(key: _deviceTokenKey);

  Future<void> savePairing({required String relayUrl, required String accountEmail, required String deviceName}) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_relayUrlKey, relayUrl);
    await prefs.setString(_accountEmailKey, accountEmail);
    await prefs.setString(_deviceNameKey, deviceName);
  }

  Future<String?> get relayUrl async => (await SharedPreferences.getInstance()).getString(_relayUrlKey);
  Future<String?> get accountEmail async => (await SharedPreferences.getInstance()).getString(_accountEmailKey);
  Future<String?> get deviceName async => (await SharedPreferences.getInstance()).getString(_deviceNameKey);

  Future<void> clearAll() async {
    await clearDeviceToken();
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_relayUrlKey);
    await prefs.remove(_accountEmailKey);
    await prefs.remove(_deviceNameKey);
  }
}
