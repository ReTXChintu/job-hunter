import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:workmanager/workmanager.dart';

import '../models/app_notification.dart';
import 'secure_store.dart';

const _checkTask = 'job_hunter.check_notifications';
const _alertedKey = 'job_hunter.notifications.alerted_at';

/// Runs in a background isolate when Android's WorkManager wakes the app
/// (about every 15 minutes, even when it's closed): fetch new notifications
/// from the server and show them.
@pragma('vm:entry-point')
void notificationCallbackDispatcher() {
  Workmanager().executeTask((task, inputData) async {
    try {
      await NotificationService.instance.checkServer();
    } catch (e) {
      debugPrint('background notification check failed: $e');
    }
    return true;
  });
}

/// Shows the desktop's notifications on the phone: right away when the
/// server pushes one while the app is running, and from a periodic
/// background check when it isn't. The server is the source; the desktop
/// doesn't need to be online.
class NotificationService {
  NotificationService._();
  static final instance = NotificationService._();

  final _plugin = FlutterLocalNotificationsPlugin();
  bool _initialized = false;

  Future<void> _init() async {
    if (_initialized) return;
    await _plugin.initialize(settings: const InitializationSettings(android: AndroidInitializationSettings('@mipmap/ic_launcher')));
    _initialized = true;
  }

  /// Signed in: ask for permission (Android 13+) and start background checks.
  Future<void> enable() async {
    await _init();
    await _plugin.resolvePlatformSpecificImplementation<AndroidFlutterLocalNotificationsPlugin>()?.requestNotificationsPermission();
    await Workmanager().initialize(notificationCallbackDispatcher);
    await Workmanager().registerPeriodicTask(
      _checkTask,
      _checkTask,
      frequency: const Duration(minutes: 15),
      constraints: Constraints(networkType: NetworkType.connected),
      existingWorkPolicy: ExistingPeriodicWorkPolicy.keep,
    );
  }

  /// Signed out: stop background checks and forget what was shown.
  Future<void> disable() async {
    try {
      await Workmanager().cancelByUniqueName(_checkTask);
    } catch (_) {}
    (await SharedPreferences.getInstance()).remove(_alertedKey);
  }

  /// Every notification on the server for this account, newest first.
  static Future<List<AppNotification>> fetch(String baseUrl, String deviceToken) async {
    final dio = Dio(BaseOptions(baseUrl: baseUrl, connectTimeout: const Duration(seconds: 15), receiveTimeout: const Duration(seconds: 15)));
    final resp = await dio.get<Map<String, dynamic>>('/v1/data/notifications', options: Options(headers: {'Authorization': 'Bearer $deviceToken'}));
    final docs = (resp.data?['documents'] as List? ?? const []).whereType<Map<String, dynamic>>();
    return sortNewest(docs.map(AppNotification.fromJson));
  }

  /// Fetch with the stored pairing and alert what's new. Used by the
  /// background task, where no controller is running.
  Future<List<AppNotification>> checkServer() async {
    final store = LocalStore();
    final url = await store.relayUrl;
    final token = await store.deviceToken;
    if (url == null || token == null) return const [];
    final list = await fetch(url, token);
    await alertFresh(list);
    return list;
  }

  /// Show the notifications this phone hasn't alerted yet. Push and
  /// background checks share one watermark, so nothing shows twice.
  Future<void> alertFresh(List<AppNotification> list) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.reload(); // the background isolate may have moved it
    final alerted = prefs.getString(_alertedKey);
    final latest = newestTimestamp(list, alerted);
    if (latest != null && latest != alerted) await prefs.setString(_alertedKey, latest);
    for (final n in freshSince(list, alerted)) {
      await _show(n);
    }
  }

  Future<void> _show(AppNotification n) async {
    await _init();
    await _plugin.show(
      id: n.id.hashCode & 0x7fffffff,
      title: n.title,
      body: n.body.isEmpty ? null : n.body,
      payload: n.applicationId,
      notificationDetails: const NotificationDetails(
        android: AndroidNotificationDetails(
          'job_hunter_alerts',
          'Job Hunter alerts',
          channelDescription: 'Finished job hunts, applications that need you, failures',
          importance: Importance.high,
          priority: Priority.high,
        ),
      ),
    );
  }
}
