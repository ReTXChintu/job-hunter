import 'dart:async';
import 'dart:math';

import 'package:web_socket_channel/web_socket_channel.dart';

import '../models/envelope.dart';

enum SocketState { disconnected, connecting, connected }

/// Owns the one WebSocket to the relay: connects with `?token=<deviceToken>`,
/// reconnects with backoff on any drop, resolves `request()` calls against
/// matching `response` frames, and exposes pushes (`presence`, `changed`,
/// `agent_status`) as a stream. Pure transport -- it never interprets
/// `payload` beyond routing by `type`/`id`, matching the relay's own "dumb
/// pipe" design (see `docs/mobile-protocol.md`).
class RelayClient {
  final String relayUrl;
  final String deviceToken;

  RelayClient({required this.relayUrl, required this.deviceToken});

  WebSocketChannel? _channel;
  StreamSubscription? _sub;
  Timer? _reconnectTimer;
  Duration _backoff = const Duration(seconds: 1);
  bool _closed = false;

  final _stateController = StreamController<SocketState>.broadcast();
  final _pushController = StreamController<Envelope>.broadcast();
  final _pending = <String, Completer<RelayResponse>>{};

  Stream<SocketState> get state => _stateController.stream;
  Stream<Envelope> get pushes => _pushController.stream;
  SocketState _current = SocketState.disconnected;
  SocketState get currentState => _current;

  String get _wsUrl {
    final withoutScheme = relayUrl.replaceFirst(RegExp(r'^https?://'), '');
    final scheme = relayUrl.startsWith('https://') ? 'wss' : 'ws';
    return '$scheme://$withoutScheme/v1/ws?token=$deviceToken';
  }

  void connect() {
    _closed = false;
    _connectOnce();
  }

  void _connectOnce() {
    if (_closed) return;
    _setState(SocketState.connecting);
    try {
      final channel = WebSocketChannel.connect(Uri.parse(_wsUrl));
      _channel = channel;
      _sub = channel.stream.listen(_onFrame, onDone: _onClosed, onError: (_) => _onClosed(), cancelOnError: true);
      // web_socket_channel's `ready` future completes once the handshake
      // succeeds; ignore its error here since onError/onDone already
      // handle a failed connection the same way.
      channel.ready.then((_) {
        if (!_closed) {
          _setState(SocketState.connected);
          _backoff = const Duration(seconds: 1);
        }
      }).catchError((_) {});
    } catch (_) {
      _scheduleReconnect();
    }
  }

  void _onFrame(dynamic raw) {
    if (raw is! String) return;
    Envelope env;
    try {
      env = Envelope.decode(raw);
    } catch (_) {
      return;
    }
    if (env.type == 'response') {
      final id = env.id?.toString();
      final completer = id == null ? null : _pending.remove(id);
      completer?.complete(RelayResponse.fromPayload(env.payload));
      return;
    }
    _pushController.add(env);
  }

  void _onClosed() {
    _sub?.cancel();
    _sub = null;
    _channel = null;
    if (_closed) {
      _setState(SocketState.disconnected);
      return;
    }
    _setState(SocketState.disconnected);
    _failAllPending('DESKTOP_OFFLINE', 'Lost the connection to the relay.');
    _scheduleReconnect();
  }

  void _scheduleReconnect() {
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(_backoff, _connectOnce);
    _backoff = Duration(seconds: min(_backoff.inSeconds * 2, 30));
  }

  void _setState(SocketState s) {
    _current = s;
    _stateController.add(s);
  }

  /// Sends a request and resolves once the matching `response` arrives, or
  /// after [timeout] with a synthetic timeout error, or immediately with a
  /// synthetic offline error if there is no live connection to send on.
  Future<RelayResponse> request(String type, [Map<String, dynamic> payload = const {}, Duration timeout = const Duration(seconds: 20)]) {
    final channel = _channel;
    if (channel == null || _current != SocketState.connected) {
      return Future.value(const RelayResponse(ok: false, errorCode: 'DESKTOP_OFFLINE', errorMessage: 'Not connected to the relay right now.'));
    }
    final id = _randomId();
    final completer = Completer<RelayResponse>();
    _pending[id] = completer;
    channel.sink.add(Envelope.request(id, type, payload).encode());
    Timer(timeout, () {
      if (!completer.isCompleted) {
        _pending.remove(id);
        completer.complete(const RelayResponse(ok: false, errorCode: 'TIMEOUT', errorMessage: 'The desktop took too long to respond.'));
      }
    });
    return completer.future;
  }

  void _failAllPending(String code, String message) {
    final entries = _pending.values.toList();
    _pending.clear();
    for (final c in entries) {
      if (!c.isCompleted) c.complete(RelayResponse(ok: false, errorCode: code, errorMessage: message));
    }
  }

  void dispose() {
    _closed = true;
    _reconnectTimer?.cancel();
    _sub?.cancel();
    _channel?.sink.close();
    _failAllPending('CLOSED', 'Disconnected.');
    _stateController.close();
    _pushController.close();
  }
}

final _rand = Random.secure();
String _randomId() => List.generate(16, (_) => _rand.nextInt(16).toRadixString(16)).join();
