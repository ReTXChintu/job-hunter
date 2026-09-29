import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';

import '../models/app_notification.dart';
import '../models/dashboard.dart';
import '../models/envelope.dart';
import '../services/relay_client.dart';
import 'auth_controller.dart';

/// Owns the live `RelayClient` for as long as the user is signed in,
/// tracks the phone's own socket state, the desktop's presence (reported by
/// the server, never guessed) and the agent's state (`agent_status` pushes),
/// and republishes `changed` pushes for the data controllers to react to.
class ConnectionController extends ChangeNotifier {
  final AuthController auth;
  ConnectionController(this.auth) {
    auth.addListener(_onAuthChanged);
    _onAuthChanged();
  }

  RelayClient? _client;
  StreamSubscription? _stateSub;
  StreamSubscription? _pushSub;

  SocketState socketState = SocketState.disconnected;
  bool desktopOnline = false;
  DateTime? desktopLastSeenAt;

  /// The agent's state as last reported by the desktop; null while unknown
  /// (desktop offline, or not reported yet).
  AgentStatusLite? agentStatus;

  final _changedController = StreamController<ChangedUpdate>.broadcast();
  Stream<ChangedUpdate> get changed => _changedController.stream;

  /// Notifications the server pushes as soon as the desktop raises them.
  final _notificationController = StreamController<AppNotification>.broadcast();
  Stream<AppNotification> get notifications => _notificationController.stream;

  /// Fires when the socket (re)connects or the desktop comes online: the
  /// moment to catch up on anything missed.
  final _resyncController = StreamController<void>.broadcast();
  Stream<void> get resync => _resyncController.stream;

  bool get connected => socketState == SocketState.connected;

  /// Actions can reach the desktop right now.
  bool get desktopReachable => connected && desktopOnline;

  void _onAuthChanged() {
    if (auth.status == AuthStatus.signedIn && auth.relayUrl != null && auth.deviceToken != null) {
      if (_client == null) _connect(auth.relayUrl!, auth.deviceToken!);
    } else {
      _disconnect();
    }
  }

  void _connect(String relayUrl, String deviceToken) {
    final client = RelayClient(relayUrl: relayUrl, deviceToken: deviceToken);
    _client = client;
    _stateSub = client.state.listen((s) {
      final was = socketState;
      socketState = s;
      if (s != SocketState.connected) {
        desktopOnline = false;
        agentStatus = null;
      }
      notifyListeners();
      if (s == SocketState.connected && was != SocketState.connected) _resyncController.add(null);
    });
    _pushSub = client.pushes.listen(_onPush);
    client.connect();
  }

  void _disconnect() {
    _stateSub?.cancel();
    _pushSub?.cancel();
    _client?.dispose();
    _client = null;
    socketState = SocketState.disconnected;
    desktopOnline = false;
    desktopLastSeenAt = null;
    agentStatus = null;
    notifyListeners();
  }

  void _onPush(Envelope env) {
    switch (env.type) {
      case 'presence':
        final p = PresenceUpdate.fromPayload(env.payload);
        final cameOnline = p.online && !desktopOnline;
        desktopOnline = p.online;
        desktopLastSeenAt = p.lastSeenAt;
        if (!p.online) agentStatus = null;
        notifyListeners();
        if (cameOnline) {
          _resyncController.add(null);
          refreshAgentStatus();
        }
      case 'changed':
        _changedController.add(ChangedUpdate.fromPayload(env.payload));
      case 'notification':
        _notificationController.add(AppNotification.fromJson(env.payload));
      case 'agent_status':
        agentStatus = AgentStatusLite.fromJson(env.payload);
        notifyListeners();
    }
  }

  /// Asks the desktop for the agent's state (it also pushes changes).
  Future<void> refreshAgentStatus() async {
    final resp = await request('get_agent_status');
    if (resp.ok && resp.data is Map) {
      agentStatus = AgentStatusLite.fromJson((resp.data as Map).cast<String, dynamic>());
      notifyListeners();
    }
  }

  /// Reads a server view directly (e.g. `/v1/applications`) with this
  /// phone's device token. The server keeps the desktop's synced data, so
  /// the phone can show it while the desktop is offline. Returns null on
  /// any failure.
  Future<dynamic> serverGet(String path) async {
    final url = auth.relayUrl;
    final token = auth.deviceToken;
    if (url == null || token == null) return null;
    try {
      final dio = Dio(BaseOptions(baseUrl: url, connectTimeout: const Duration(seconds: 15), receiveTimeout: const Duration(seconds: 20)));
      final resp = await dio.get<dynamic>(path, options: Options(headers: {'Authorization': 'Bearer $token'}));
      return resp.data;
    } catch (e) {
      debugPrint('server read $path failed: $e');
      return null;
    }
  }

  /// Sends a request to the desktop through the server. Fails immediately
  /// with `NOT_CONNECTED` when there's no socket, and the server itself
  /// answers `DESKTOP_OFFLINE` when the desktop isn't connected -- it never
  /// queues an action.
  Future<RelayResponse> request(String type, [Map<String, dynamic> payload = const {}]) {
    final client = _client;
    if (client == null) {
      return Future.value(const RelayResponse(ok: false, errorCode: 'NOT_CONNECTED', errorMessage: 'Not signed in.'));
    }
    return client.request(type, payload);
  }

  @override
  void dispose() {
    auth.removeListener(_onAuthChanged);
    _disconnect();
    _changedController.close();
    _notificationController.close();
    _resyncController.close();
    super.dispose();
  }
}
