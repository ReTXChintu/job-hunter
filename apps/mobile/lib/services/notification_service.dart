import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:workmanager/workmanager.dart';

import '../logic/links.dart';
import '../models/app_notification.dart';
import 'link_bus.dart';
import 'secure_store.dart';

const _checkTask = 'job_hunter.check_notifications';
const _alertedKey = 'job_hunter.notifications.alerted_at';

/// The Android channel every alert uses, including Firebase pushes shown by
/// the system while the app is closed (see the manifest's
/// `default_notification_channel_id`).
const alertsChannelId = 'job_hunter_alerts';
const _alertsChannel = AndroidNotificationChannel(
  alertsChannelId,
  'Job Hunter alerts',
  description: 'Finished job hunts, applications that need you, failures',
  importance: Importance.high,
);

/// Runs in a background isolate when Android's WorkManager wakes the app
/// (about every 15 minutes, even when it's closed): fetch new notifications
/// from the server and show them. The fallback when Firebase push isn't set up.
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
/// server pushes one (WebSocket while the app runs, Firebase otherwise), and
/// from a periodic background check as a fallback. The server is the
/// source; the desktop doesn't need to be online. Every path shares one
/// `createdAt` watermark, so nothing is shown twice.
class NotificationService {
  NotificationService._();
  static final instance = NotificationService._();

  final _plugin = FlutterLocalNotificationsPlugin();
  bool _initialized = false;
  Future<void> _lock = Future.value();
  final _shownIds = <String>{};

  Future<void> _init() async {
    if (_initialized) return;
    await _plugin.initialize(
      settings: const InitializationSettings(android: AndroidInitializationSettings('ic_notification')),
      onDidReceiveNotificationResponse: (response) => LinkBus.instance.open(decodeLinkPayload(response.payload)),
    );
    _initialized = true;
  }

  /// At startup: create the alerts channel (so Firebase pushes that arrive
  /// before anything was shown locally use it too) and, if a notification
  /// tap launched the app, open its screen.
  Future<void> start() async {
    await _init();
    final android = _plugin.resolvePlatformSpecificImplementation<AndroidFlutterLocalNotificationsPlugin>();
    await android?.createNotificationChannel(_alertsChannel);
    final launch = await _plugin.getNotificationAppLaunchDetails();
    if (launch?.didNotificationLaunchApp == true) {
      LinkBus.instance.open(decodeLinkPayload(launch!.notificationResponse?.payload));
    }
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
    final docs = (resp.data?['documents'] as List? ?? const []).whereType<Map>().map((m) => m.cast<String, dynamic>());
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

  /// Runs [body] after every earlier call finished: the WebSocket push and a
  /// Firebase message for the same notification can arrive together.
  Future<T> _serialized<T>(Future<T> Function() body) {
    final result = _lock.then((_) => body());
    _lock = result.then((_) {}, onError: (_) {});
    return result;
  }

  /// Show the notifications this phone hasn't alerted yet.
  Future<void> alertFresh(List<AppNotification> list) => _serialized(() async {
        final prefs = await SharedPreferences.getInstance();
        await prefs.reload(); // the background isolate may have moved it
        final alerted = prefs.getString(_alertedKey);
        final latest = newestTimestamp(list, alerted);
        if (latest != null && latest != alerted) await prefs.setString(_alertedKey, latest);
        for (final n in freshSince(list, alerted)) {
          if (!_shownIds.add(n.id)) continue;
          await _show(n);
        }
      });

  /// A notification was already shown by the system (a Firebase push while
  /// the app was in the background): move the watermark so the periodic
  /// check doesn't show it again.
  Future<void> markAlerted(String createdAt) => _serialized(() async {
        if (createdAt.isEmpty) return;
        final prefs = await SharedPreferences.getInstance();
        await prefs.reload();
        final alerted = prefs.getString(_alertedKey);
        if (alerted == null || createdAt.compareTo(alerted) > 0) await prefs.setString(_alertedKey, createdAt);
      });

  Future<void> _show(AppNotification n) async {
    await _init();
    await _plugin.show(
      id: n.id.hashCode & 0x7fffffff,
      title: n.title,
      body: n.body.isEmpty ? null : n.body,
      payload: encodeLinkPayload(n.linkPage, n.linkId),
      notificationDetails: NotificationDetails(
        android: AndroidNotificationDetails(
          _alertsChannel.id,
          _alertsChannel.name,
          channelDescription: _alertsChannel.description,
          importance: Importance.high,
          priority: Priority.high,
          icon: 'ic_notification',
        ),
      ),
    );
  }
}
