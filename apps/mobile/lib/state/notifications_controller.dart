import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/app_notification.dart';
import '../services/notification_service.dart';
import '../services/push_service.dart';
import 'auth_controller.dart';
import 'connection_controller.dart';

/// The Alerts tab: the desktop's notifications read from the server, an
/// unread count for the badge, and phone alerts for new ones pushed while
/// the app is running (WebSocket or Firebase). Catches up whenever the app
/// comes back to the foreground or the connection returns.
class NotificationsController extends ChangeNotifier with WidgetsBindingObserver {
  static const _seenKey = 'job_hunter.notifications.seen_at';

  final AuthController auth;
  final ConnectionController connection;
  StreamSubscription? _pushSub;
  StreamSubscription? _firebaseSub;
  StreamSubscription? _resyncSub;
  bool _enabled = false;

  List<AppNotification> items = const [];
  String? _seenAt;
  bool loading = false;
  String? error;

  NotificationsController(this.auth, this.connection) {
    _pushSub = connection.notifications.listen(_onPush);
    _firebaseSub = PushService.instance.foreground.listen(_onPush);
    _resyncSub = connection.resync.listen((_) {
      if (_enabled) refresh();
    });
    WidgetsBinding.instance.addObserver(this);
    auth.addListener(_onAuthChanged);
    _onAuthChanged();
  }

  int get unread => _seenAt == null ? items.length : items.where((n) => n.createdAt.compareTo(_seenAt!) > 0).length;
  bool isUnread(AppNotification n) => _seenAt == null || n.createdAt.compareTo(_seenAt!) > 0;

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed && _enabled) refresh();
  }

  Future<void> _onAuthChanged() async {
    final signedIn = auth.status == AuthStatus.signedIn;
    if (signedIn && !_enabled) {
      _enabled = true;
      _seenAt = (await SharedPreferences.getInstance()).getString(_seenKey);
      try {
        await NotificationService.instance.enable();
      } catch (e) {
        debugPrint('could not enable notifications: $e');
      }
      final url = auth.relayUrl, token = auth.deviceToken;
      if (url != null && token != null) unawaited(PushService.instance.register(url, token));
      await refresh();
    } else if (!signedIn && _enabled) {
      _enabled = false;
      items = const [];
      await NotificationService.instance.disable();
      notifyListeners();
    }
  }

  Future<void> refresh() async {
    final url = auth.relayUrl;
    final token = auth.deviceToken;
    if (url == null || token == null) return;
    loading = true;
    notifyListeners();
    try {
      items = await NotificationService.fetch(url, token);
      error = null;
      await NotificationService.instance.alertFresh(items);
    } catch (e) {
      error = 'Could not load notifications. Check your connection.';
    } finally {
      loading = false;
      notifyListeners();
    }
  }

  Future<void> _onPush(AppNotification n) async {
    if (n.id.isNotEmpty && !items.any((x) => x.id == n.id)) {
      items = sortNewest([n, ...items]);
      notifyListeners();
    }
    await NotificationService.instance.alertFresh([n]);
  }

  /// The user looked at the list.
  Future<void> markAllSeen() async {
    final latest = newestTimestamp(items, _seenAt);
    if (latest == null || latest == _seenAt) return;
    _seenAt = latest;
    await (await SharedPreferences.getInstance()).setString(_seenKey, latest);
    notifyListeners();
  }

  @override
  void dispose() {
    _pushSub?.cancel();
    _firebaseSub?.cancel();
    _resyncSub?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    auth.removeListener(_onAuthChanged);
    super.dispose();
  }
}
