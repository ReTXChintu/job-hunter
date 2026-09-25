import 'package:flutter/widgets.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:url_launcher/url_launcher.dart';

import '../config.dart';
import '../services/update_service.dart';
import 'auth_controller.dart';

/// Checks the Job Hunter server for a newer APK when the app starts and
/// whenever it comes back to the foreground (at most every 6 hours unless
/// asked). Installing is Android's own flow: "Download" opens the APK URL in
/// the browser, and tapping the finished download installs it over this
/// app, keeping its data (as long as both builds share a signing key).
class UpdateController extends ChangeNotifier with WidgetsBindingObserver {
  static const _autoCheckEvery = Duration(hours: 6);
  static const _dismissedKey = 'job_hunter.update_dismissed';
  /// "Later" hides the banner for this long, then it comes back.
  static const _remindAfter = Duration(days: 3);

  final AuthController _auth;
  final UpdateService _service;

  UpdateController(this._auth, {UpdateService? service}) : _service = service ?? UpdateService() {
    WidgetsBinding.instance.addObserver(this);
    _auth.addListener(_onAuthChanged);
    _init();
  }

  // Without a built-in server there's nothing to check until sign-in.
  void _onAuthChanged() {
    if (info == null) check();
  }

  PackageInfo? package;
  UpdateInfo? info;
  bool isChecking = false;
  String? lastError;
  DateTime? _lastChecked;
  String? _dismissedVersion;
  DateTime? _dismissedUntil;

  String get currentVersion => package?.version ?? '';
  int get currentBuild => int.tryParse(package?.buildNumber ?? '') ?? 0;

  /// The server the app talks to: the one built in, else the one signed in to.
  String? get serverUrl => hasBuiltInServer ? kBuiltInServerUrl : _auth.relayUrl;

  /// Show the home-screen banner (the About screen shows updates regardless).
  bool get showBanner {
    if (info?.available != true) return false;
    final snoozed = info?.latest?.version == _dismissedVersion && _dismissedUntil != null && DateTime.now().isBefore(_dismissedUntil!);
    return !snoozed;
  }

  Future<void> _init() async {
    package = await PackageInfo.fromPlatform();
    // Stored as "<version>|<until, ms since epoch>".
    final stored = (await SharedPreferences.getInstance()).getString(_dismissedKey)?.split('|');
    if (stored != null && stored.length == 2) {
      _dismissedVersion = stored[0];
      final until = int.tryParse(stored[1]);
      _dismissedUntil = until == null ? null : DateTime.fromMillisecondsSinceEpoch(until);
    }
    notifyListeners();
    await check();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed) return;
    final last = _lastChecked;
    if (last == null || DateTime.now().difference(last) > _autoCheckEvery) check();
  }

  Future<void> check() async {
    final server = serverUrl;
    if (package == null || server == null || server.isEmpty || isChecking) return;
    isChecking = true;
    lastError = null;
    notifyListeners();
    try {
      info = await _service.check(serverUrl: server, currentVersion: currentVersion, currentBuild: currentBuild);
      _lastChecked = DateTime.now();
    } catch (_) {
      lastError = "Couldn't reach the server to check for updates.";
    } finally {
      isChecking = false;
      notifyListeners();
    }
  }

  Future<void> dismiss() async {
    final version = info?.latest?.version;
    if (version == null) return;
    _dismissedVersion = version;
    _dismissedUntil = DateTime.now().add(_remindAfter);
    notifyListeners();
    await (await SharedPreferences.getInstance()).setString(_dismissedKey, '$version|${_dismissedUntil!.millisecondsSinceEpoch}');
  }

  /// Opens the APK download in the browser. Returns false if nothing could open it.
  Future<bool> download() async {
    final url = info?.downloadUrl;
    if (url == null) return false;
    return launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _auth.removeListener(_onAuthChanged);
    super.dispose();
  }
}
