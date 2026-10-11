import 'dart:async';

import 'package:flutter/foundation.dart';

import '../logic/labels.dart';
import '../models/envelope.dart';
import '../models/queue.dart';
import 'connection_controller.dart';

/// What waits for the desktop's agent (`list_queue`), kept current from
/// the `queued` count the desktop pushes with every `agent_status`: when it
/// changes, the list is read again. Removing and clearing go to the desktop.
class QueueController extends ChangeNotifier {
  final ConnectionController connection;
  StreamSubscription? _changedSub;
  StreamSubscription? _resyncSub;
  int? _lastQueued;

  QueueController(this.connection) {
    connection.addListener(_onConnection);
    _changedSub = connection.changed.listen((u) {
      if (u.collections.contains('agent_queue')) refresh();
    });
    _resyncSub = connection.resync.listen((_) => refresh());
  }

  List<QueuedTaskItem> items = const [];
  bool loading = false;
  String? error;

  /// The desktop's own count, which can be ahead of [items] for a moment.
  int get count => connection.agentStatus?.queued ?? items.length;

  void _onConnection() {
    final q = connection.agentStatus?.queued;
    if (q == _lastQueued) return;
    _lastQueued = q;
    if (q == null) {
      // Desktop offline: its queue can't be read (or acted on).
      items = const [];
      error = null;
      notifyListeners();
    } else {
      refresh();
    }
  }

  Future<void> refresh() async {
    loading = true;
    notifyListeners();
    final resp = await connection.request('list_queue');
    if (resp.ok && resp.data is List) {
      items = parseQueue(resp.data);
      error = null;
    } else if (resp.errorCode == 'DESKTOP_OFFLINE' || resp.errorCode == 'NOT_CONNECTED') {
      items = const [];
      error = null;
    } else {
      error = friendlyError(resp);
    }
    loading = false;
    notifyListeners();
  }

  Future<RelayResponse> remove(String id) => _act('remove_queued_task', {'id': id});

  Future<RelayResponse> clear() => _act('clear_queue', const {});

  Future<RelayResponse> _act(String type, Map<String, dynamic> payload) async {
    final resp = await connection.request(type, payload);
    if (resp.ok) unawaited(refresh());
    return resp;
  }

  @override
  void dispose() {
    connection.removeListener(_onConnection);
    _changedSub?.cancel();
    _resyncSub?.cancel();
    super.dispose();
  }
}
