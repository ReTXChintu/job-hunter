import 'dart:async';

import 'package:flutter/foundation.dart';

import '../models/envelope.dart';
import '../services/relay_client.dart';
import 'auth_controller.dart';

/// Owns the live `RelayClient` for as long as the user is signed in,
/// tracks the phone's own socket state and the desktop's presence
/// (reported by the relay, never guessed), and republishes `changed`
/// pushes for `ApplicationsController` to react to.
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

  final _changedController = StreamController<ChangedUpdate>.broadcast();
  Stream<ChangedUpdate> get changed => _changedController.stream;

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
      socketState = s;
      notifyListeners();
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
    notifyListeners();
  }

  void _onPush(Envelope env) {
    switch (env.type) {
      case 'presence':
        final p = PresenceUpdate.fromPayload(env.payload);
        desktopOnline = p.online;
        desktopLastSeenAt = p.lastSeenAt;
        notifyListeners();
      case 'changed':
        _changedController.add(ChangedUpdate.fromPayload(env.payload));
      case 'agent_status':
        // Reserved for a future "agent is applying" indicator; the phone
        // does not need per-step detail (see docs/mobile-protocol.md).
        break;
    }
  }

  /// Sends a request to the desktop through the relay. Returns a
  /// `DESKTOP_OFFLINE` response immediately if not connected, exactly like
  /// the relay itself would.
  Future<RelayResponse> request(String type, [Map<String, dynamic> payload = const {}]) {
    final client = _client;
    if (client == null) {
      return Future.value(const RelayResponse(ok: false, errorCode: 'DESKTOP_OFFLINE', errorMessage: 'Not signed in.'));
    }
    return client.request(type, payload);
  }

  @override
  void dispose() {
    auth.removeListener(_onAuthChanged);
    _disconnect();
    _changedController.close();
    super.dispose();
  }
}
